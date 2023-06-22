use crate::messages::coordinator::{CDMessage, Message};
use crate::messages::ui::UIMessage;
use crate::messages::IOMessage;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashSet;
use std::env;
use std::{collections::HashMap, sync::Arc};
use tokio::join;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, mpsc, Mutex},
};

lazy_static! {
    /// Port to port mapping, for which sockets should be proxied to each other.
    pub static ref PORT_MAPPINGS: HashMap<u64, u64> = if let Ok(var) = env::var("PORT_MAPPINGS") {
        let mut map = HashMap::new();
        let x: Vec<Vec<u64>> = serde_json::from_str(&var).expect("wrong config format");
        for mapping in x {
            if mapping.len() != 2 {
                panic!("wrong config format");
            }
            map.insert(mapping[0], mapping[1]);
            map.insert(mapping[1], mapping[0]);
        }
        map
    } else {
        panic!("missing config")
    };
    /// Ports on which the nodes are supposed to connect with their client API socket.
    pub static ref CLIENT_PORTS: Vec<u64> = if let Ok(var) = env::var("CLIENT_PORTS") {
        serde_json::from_str(&var).expect("wrong config format")
    } else {
        panic!("missing config")
    };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct NetworkState {
    pub(crate) nodes: Vec<u64>,
    pub(crate) alive_nodes: Vec<u64>,
    pub(crate) partitions: HashSet<(u64, u64)>,
}

pub struct Coordinator {
    receiver: Receiver<CDMessage>,
    io_sender: Sender<IOMessage>,
    op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
    partitions: Arc<Mutex<HashSet<(u64, u64)>>>,
    nodes: Vec<u64>,
}

impl Coordinator {
    pub(crate) fn new(receiver: Receiver<CDMessage>, io_sender: Sender<IOMessage>) -> Self {
        Self {
            receiver,
            io_sender,
            op_sockets: Arc::new(Mutex::new(HashMap::new())),
            partitions: Arc::new(Mutex::new(HashSet::new())),
            nodes: vec![],
        }
    }

    async fn create_network_state(&self) -> NetworkState {
        NetworkState {
            nodes: self.nodes.clone(),
            alive_nodes: self
                .op_sockets
                .lock()
                .await
                .keys()
                .map(|&key| key)
                .collect(),
            partitions: self.partitions.lock().await.clone(),
        }
    }

    async fn create_omnipaxos_listeners(
        op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
        sender: Sender<IOMessage>,
    ) {
        // setup client sockets to talk to nodes
        for port in CLIENT_PORTS.iter() {
            let op_sockets = op_sockets.clone();
            let sender = sender.clone();

            // Set up API sockets
            tokio::spawn(async move {
                let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
                    .await
                    .unwrap();
                let (socket, _addr) = listener.accept().await.unwrap();
                let (reader, writer) = socket.into_split();
                op_sockets.lock().await.insert(*port, writer);
                // receiver actor
                tokio::spawn(async move {
                    let mut reader = BufReader::new(reader);
                    loop {
                        let mut data = vec![];
                        let bytes_read = reader.read_until(b'\n', &mut data).await.unwrap();
                        if bytes_read == 0 {
                            // dropped socket EOF
                            sender
                                .send(IOMessage::UIMessage(UIMessage::OmnipaxosNodeCrashed(*port)))
                                .await
                                .unwrap();
                            op_sockets.lock().await.remove(port);
                            break;
                        }
                        if let Ok(msg) = serde_json::from_slice::<Message>(&data) {
                            if let Message::APIResponse(response) = msg {
                                sender
                                    .send(IOMessage::UIMessage(UIMessage::OmnipaxosResponse(
                                        response,
                                    )))
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                });
            });
        }
    }

    async fn create_network_actor(partitions: Arc<Mutex<HashSet<(u64, u64)>>>) {
        // setup intra-cluster communication
        //let partitions: Arc<Mutex<Vec<(u64, u64, f32)>>> = Arc::new(Mutex::new(vec![]));
        let mut out_channels = HashMap::new();
        for port in PORT_MAPPINGS.keys() {
            let (sender, _rec) = broadcast::channel::<Vec<u8>>(10000);
            let sender = Arc::new(sender);
            out_channels.insert(*port, sender.clone());
        }
        let out_channels = Arc::new(out_channels);

        let (central_sender, mut central_receiver) = mpsc::channel(10000);
        let central_sender = Arc::new(central_sender);

        for port in PORT_MAPPINGS.keys() {
            let out_chans = out_channels.clone();
            let central_sender = central_sender.clone();
            tokio::spawn(async move {
                let central_sender = central_sender.clone();
                let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
                    .await
                    .unwrap();
                let (socket, _addr) = listener.accept().await.unwrap();
                let (reader, mut writer) = socket.into_split();
                // sender actor
                let out_channels = out_chans.clone();
                tokio::spawn(async move {
                    let mut receiver = out_channels.get(port).unwrap().clone().subscribe();
                    while let Ok(data) = receiver.recv().await {
                        let _ = writer.write_all(&data).await;
                    }
                });
                // receiver actor
                let central_sender = central_sender.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(reader);
                    loop {
                        let mut data = vec![];
                        reader.read_until(b'\n', &mut data).await.unwrap();
                        if let Err(e) = central_sender
                            .send((port, PORT_MAPPINGS.get(port).unwrap(), data))
                            .await
                        {
                            println!("DEBUG: senderror on central_sender: {:?}", e);
                        };
                    }
                });
            });
        }

        // the one central actor that sees all messages
        while let Some((from_port, to_port, msg)) = central_receiver.recv().await {
            // drop message if network is partitioned between sender and receiver
            let nodes_are_connected = !partitions.lock().await.contains(&(*from_port, *to_port));
            if nodes_are_connected {
                let sender = out_channels.get(to_port).unwrap().clone();
                if let Err(e) = sender.send(msg) {
                    println!(
                        "DEBUG: senderror on out_sender for port {:?}: {:?}",
                        to_port, e
                    );
                };
            }
        }
    }

    pub async fn run(&mut self) {
        while let Some(m) = self.receiver.recv().await {
            match m {
                CDMessage::Initialize => {
                    self.nodes = CLIENT_PORTS.clone();
                    let op_sockets = self.op_sockets.clone();
                    let io_sender = self.io_sender.clone();
                    let partitions = self.partitions.clone();
                    join!(
                        Coordinator::create_omnipaxos_listeners(op_sockets, io_sender),
                        Coordinator::create_network_actor(partitions)
                    );
                    let cluster = self.create_network_state().await;
                    self.io_sender
                        .send(IOMessage::UIMessage(UIMessage::OmnipaxosNetworkUpdate(
                            cluster,
                        )))
                        .await
                        .unwrap();
                }
                CDMessage::KVCommand(command) => {
                    let mut sent_command = false;
                    for port in CLIENT_PORTS.iter() {
                        if let Some(writer) = self.op_sockets.lock().await.get_mut(port) {
                            let cmd = Message::APICommand(command);
                            let mut data =
                                serde_json::to_vec(&cmd).expect("could not serialize cmd");
                            data.push(b'\n');
                            writer.write_all(&data).await.unwrap();
                            println!("send KV command to {port}");
                            sent_command = true;
                            break;
                        }
                    }
                    if !sent_command {
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::ClusterUnreachable))
                            .await
                            .unwrap();
                    }
                }
                CDMessage::SetConnection(from, to, is_connected) => {
                    let mut partitions = self.partitions.lock().await;
                    let networked_updated = match is_connected {
                        true => partitions.insert((from, to)),
                        false => partitions.remove(&(from, to)),
                    };
                    if networked_updated {
                        let cluster = self.create_network_state().await;
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::OmnipaxosNetworkUpdate(
                                cluster,
                            )))
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }
}

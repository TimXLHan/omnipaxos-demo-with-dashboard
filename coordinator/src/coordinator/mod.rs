use crate::messages::coordinator::{APIResponse, CDMessage, Message};
use crate::messages::ui::UIMessage;
use crate::messages::IOMessage;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::join;
use std::env;
use std::{
    collections::HashMap,
    io::{stdout, Write},
    sync::Arc,
    time::Duration,
};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, mpsc, Mutex},
    time::sleep,
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

pub struct Coordinator {
    receiver: Receiver<CDMessage>,
    io_sender: Sender<IOMessage>,
    op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
    partitions: Arc<Mutex<Vec<(u64, u64, f32)>>>,
}

impl Coordinator {
    pub(crate) fn new(receiver: Receiver<CDMessage>, io_sender: Sender<IOMessage>) -> Self {
        Self {
            receiver,
            io_sender,
            op_sockets: Arc::new(Mutex::new(HashMap::new())),
            partitions: Arc::new(Mutex::new(vec![])),
        }
    }

    async fn create_omnipaxos_listeners(op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>, sender: Sender<IOMessage>) {
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
                            sender.send(IOMessage::UIMessage(UIMessage::OmnipaxosNodeCrashed(*port))).await.unwrap();
                            op_sockets.lock().await.remove(port);
                            break;
                        }
                        if let Ok(msg) = serde_json::from_slice::<Message>(&data) {
                            if let Message::APIResponse(response) = msg {
                                sender.send(IOMessage::UIMessage(UIMessage::OmnipaxosResponse(
                                    response,
                                ))).await.unwrap();
                            }
                        }
                    }
                });
            });
        }
    }

    async fn create_network_actor(partitions: Arc<Mutex<Vec<(u64, u64, f32)>>>) {
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
            for (from, to, _probability) in partitions.lock().await.iter() {
                if from == from_port && to == to_port {
                    continue;
                }
            }
            let sender = out_channels.get(to_port).unwrap().clone();
            if let Err(e) = sender.send(msg) {
                println!(
                    "DEBUG: senderror on out_sender for port {:?}: {:?}",
                    to_port, e
                );
            };
        }
    }

    pub async fn run(&mut self) {
        println!("Coordinator running");
        let op_sockets = self.op_sockets.clone();
        let sender = self.io_sender.clone();
        while let Some(m) = self.receiver.recv().await {
            match m {
                CDMessage::Initialize => {
                    println!("Coordinator initialized");
                    let op_sockets = self.op_sockets.clone();
                    let sender = self.io_sender.clone();
                    tokio::spawn(async move {
                        Coordinator::create_omnipaxos_listeners(op_sockets, sender).await
                    });
                    let partitions = self.partitions.clone();
                    tokio::spawn(Coordinator::create_network_actor(partitions));
                }
                CDMessage::KVCommand(command) => {
                    let mut sent_command = false;
                    for port in CLIENT_PORTS.iter() {
                        if let Some(writer) = op_sockets.lock().await.get_mut(port) {
                            let cmd = Message::APICommand(command);
                            let mut data = serde_json::to_vec(&cmd).expect("could not serialize cmd");
                            data.push(b'\n');
                            writer.write_all(&data).await.unwrap();
                            sent_command = true;
                            break;
                        }
                    }
                    if !sent_command {
                        sender.send(IOMessage::UIMessage(UIMessage::ClusterUnreachable)).await.unwrap();
                    }
                }
                CDMessage::Partition(from, to) => {
                    for (f, t ,w) in self.partitions.lock().await.iter_mut() {
                        if *f == from && *t == to {
                            *w = 0.0;
                        }
                    }
                }
            }
        }



        // join!(
        //     self.create_omnipaxos_listeners(), 
        //     self.create_network_actor(),
        //     self.create_io_listener()
        // );

        // // Handle user input to propose values
        // let api = api_sockets.clone();
        // tokio::spawn(async move {
        //     loop {
        //         // Get input
        //         let mut input = String::new();
        //         print!("Type a command here <put/delete/get> <args>: ");
        //         let _ = stdout().flush();
        //         let mut reader = BufReader::new(tokio::io::stdin());
        //         reader.read_line(&mut input).await.expect("Did not enter a string");
        //
        //         // Parse and send command
        //         match parse_command(input) {
        //             Ok(command) => {
        //                 let mut sent_command = false;
        //                 for port in CLIENT_PORTS.iter() {
        //                     if let Some(writer) = api.lock().await.get_mut(port) {
        //                         let cmd = Message::APICommand(command.clone());
        //                         let mut data = serde_json::to_vec(&cmd).expect("could not serialize cmd");
        //                         data.push(b'\n');
        //                         writer.write_all(&data).await.unwrap();
        //                         sent_command = true;
        //                         break;
        //                     }
        //                 }
        //                 if !sent_command {
        //                     println!("Couldn't send command, all nodes are unreachable");
        //                 }
        //             }
        //             Err(err) => println!("{err}"),
        //         }
        //         // Wait some amount of time for cluster response
        //         sleep(Duration::from_millis(500)).await;
        //     }
        // });

    }
}

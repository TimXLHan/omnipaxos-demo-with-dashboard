use crate::messages::coordinator::{APIResponse, CDMessage, KVCommand, Message, Round};
use crate::messages::ui::UIMessage;
use crate::messages::IOMessage;
use rand::random;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{HashSet, VecDeque};
use std::env;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::join;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, mpsc, Mutex},
};

use self::proposal_streamer::ProposalStreamer;
pub mod proposal_streamer;

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
    /// Mapping between PORT_MAPPING keys and CLIENT_PORTS
    pub static ref PORT_TO_PID_MAPPING: HashMap<u64, u64> = if let Ok(var) = env::var("PORT_TO_PID_MAPPING") {
        let mut map = HashMap::new();
        let x: Vec<Vec<u64>> = serde_json::from_str(&var).expect("wrong config format");
        for mapping in x {
            if mapping.len() != 2 {
                panic!("wrong config format");
            }
            map.insert(mapping[0], mapping[1]);
        }
        map
    } else {
        panic!("missing config")
    };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkState {
    pub(crate) nodes: Vec<u64>,
    pub(crate) alive_nodes: Vec<u64>,
    pub(crate) partitions: HashSet<(u64, u64)>,
    pub(crate) max_round: Option<Round>,
}

pub struct Coordinator {
    receiver: Receiver<CDMessage>,
    io_sender: Sender<IOMessage>,
    op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
    partitions: Arc<Mutex<HashSet<(u64, u64)>>>,
    nodes: Vec<u64>,
    max_round: Arc<Mutex<Option<Round>>>,
    cmd_queue: Arc<Mutex<VecDeque<KVCommand>>>,
}

impl Coordinator {
    pub(crate) fn new(receiver: Receiver<CDMessage>, io_sender: Sender<IOMessage>) -> Self {
        Self {
            receiver,
            io_sender,
            op_sockets: Arc::new(Mutex::new(HashMap::new())),
            partitions: Arc::new(Mutex::new(HashSet::new())),
            cmd_queue: Arc::new(Mutex::new(VecDeque::new())),
            nodes: vec![],
            max_round: Arc::new(Mutex::new(None)),
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
            max_round: self.max_round.lock().await.clone(),
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
                let client_pid = *PORT_TO_PID_MAPPING.get(port).unwrap();
                op_sockets.lock().await.insert(client_pid, writer);
                sender
                    .send(IOMessage::CDMessage(CDMessage::OmnipaxosNodeJoined(
                        client_pid,
                    )))
                    .await
                    .unwrap();
                // receiver actor
                tokio::spawn(async move {
                    let mut reader = BufReader::new(reader);
                    loop {
                        let mut data = vec![];
                        let bytes_read = reader.read_until(b'\n', &mut data).await.unwrap();
                        if bytes_read == 0 {
                            // dropped socket EOF
                            sender
                                .send(IOMessage::UIMessage(UIMessage::OmnipaxosNodeCrashed(
                                    client_pid,
                                )))
                                .await
                                .unwrap();
                            op_sockets.lock().await.remove(&client_pid);
                            sender
                                .send(IOMessage::CDMessage(CDMessage::OmnipaxosNodeCrashed(
                                    client_pid,
                                )))
                                .await
                                .unwrap();
                            break;
                        }
                        if let Ok(msg) = serde_json::from_slice::<Message>(&data) {
                            match msg {
                                Message::APIResponse(APIResponse::NewRound(round)) => sender
                                    .send(IOMessage::CDMessage(CDMessage::NewRound(
                                        client_pid, round,
                                    )))
                                    .await
                                    .unwrap(),
                                Message::APIResponse(response) => sender
                                    .send(IOMessage::UIMessage(UIMessage::OmnipaxosResponse(
                                        response,
                                    )))
                                    .await
                                    .unwrap(),
                                _ => (),
                            }
                        }
                    }
                });
            });
        }
    }

    async fn create_network_actor(partitions: Arc<Mutex<HashSet<(u64, u64)>>>) {
        // setup intra-cluster communication
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
                        _ = central_sender
                            .send((port, PORT_MAPPINGS.get(port).unwrap(), data))
                            .await
                    }
                });
            });
        }

        // the one central actor that sees all messages
        tokio::spawn(async move {
            while let Some((from_port, to_port, msg)) = central_receiver.recv().await {
                // drop message if network is partitioned between sender and receiver
                // TODO: TUI can only display undirected connections, so we don't support one way
                // connect partitions
                let connection = match (
                    PORT_TO_PID_MAPPING.get(from_port).unwrap(),
                    PORT_TO_PID_MAPPING.get(to_port).unwrap(),
                ) {
                    (from, to) if from <= to => (*from, *to),
                    (from, to) => (*to, *from),
                };
                let nodes_are_connected = !partitions.lock().await.contains(&connection);
                if nodes_are_connected {
                    let sender = out_channels.get(to_port).unwrap().clone();
                    _ = sender.send(msg);
                }
            }
        });
    }

    pub async fn run(&mut self) {
        while let Some(m) = self.receiver.recv().await {
            match m {
                CDMessage::Initialize => {
                    self.nodes = CLIENT_PORTS
                        .iter()
                        .map(|port| *PORT_TO_PID_MAPPING.get(port).unwrap())
                        .collect();
                    let mut proposer = ProposalStreamer::new(self.io_sender.clone(), self.op_sockets.clone(), self.cmd_queue.clone(), self.max_round.clone());
                    tokio::spawn(async move {
                        proposer.run().await
                    });

                    let op_sockets = self.op_sockets.clone();
                    let io_sender = self.io_sender.clone();
                    let partitions = self.partitions.clone();
                    join!(
                        Coordinator::create_omnipaxos_listeners(op_sockets, io_sender),
                        Coordinator::create_network_actor(partitions),
                    );
                }
                CDMessage::KVCommand(command) => self.cmd_queue.lock().await.push_front(command),
                CDMessage::SetConnection(from, to, is_connected) => {
                    if !self.nodes.contains(&from) {
                        self.send_to_ui(UIMessage::NoSuchNode(from, self.nodes.clone()))
                            .await;
                    } else if to.is_some() && !self.nodes.contains(&to.unwrap()) {
                        self.send_to_ui(UIMessage::NoSuchNode(to.unwrap(), self.nodes.clone()))
                            .await;
                    } else {
                        // TODO: translate PIDs to ports so network_actor doesn't have to do it
                        // constantly.
                        // TODO: TUI can only display undirected connections, so we don't support one way
                        // connect partitions
                        match to {
                            Some(to) => {
                                let connection = self.get_connection(from, to);
                                let mut partitions = self.partitions.lock().await;
                                match is_connected {
                                    true => partitions.remove(&connection),
                                    false => !partitions.insert(connection),
                                };
                                drop(partitions);
                                self.send_network_update().await;
                            }
                            None => {
                                let other_nodes = self.nodes.iter().filter(|&&n| n != from);
                                let mut partitions = self.partitions.lock().await;
                                for node in other_nodes {
                                    let connection = self.get_connection(from, *node);
                                    match is_connected {
                                        true => partitions.remove(&connection),
                                        false => !partitions.insert(connection),
                                    };
                                }
                                drop(partitions);
                                self.send_network_update().await;
                            }
                        }
                    }
                }
                CDMessage::OmnipaxosNodeCrashed(_pid) => {
                    self.send_network_update().await;
                }
                CDMessage::OmnipaxosNodeJoined(_pid) => {
                    self.send_network_update().await;
                }
                CDMessage::StartBatchingPropose(num) => self.batch_proposals(num).await,
                CDMessage::NewRound(_client_pid, new_round) => {
                    let mut curr_round = self.max_round.lock().await;
                    match (*curr_round, new_round) {
                        (Some(old_round), Some(round)) if old_round < round => {
                            *curr_round = Some(round);
                            drop(curr_round);
                            self.send_network_update().await;
                        }
                        (None, Some(round)) => {
                            *curr_round = Some(round);
                            drop(curr_round);
                            self.send_network_update().await;
                        }
                        _ => (),
                    }
                },
                CDMessage::Scenario(scenario_type) => {
                    assert!(
                        self.nodes.len() == 5,
                        "Must have 5 nodes to execute scenarios"
                    );
                    self.handle_scenario(scenario_type).await;
                }
            }
        }
    }

    async fn send_to_ui(&self, msg: UIMessage) {
        self.io_sender
            .send(IOMessage::UIMessage(msg))
            .await
            .unwrap();
    }

    async fn send_network_update(&self) {
        let cluster = self.create_network_state().await;
        self.io_sender
            .send(IOMessage::UIMessage(UIMessage::OmnipaxosNetworkUpdate(
                cluster,
            )))
            .await
            .unwrap();
    }

    fn get_connection(&self, from: u64, to: u64) -> (u64, u64) {
        match from <= to {
            true => (from, to),
            false => (to, from),
        }
    }

    async fn batch_proposals(&self, num: u64) {
        let mut cmd_queue = self.cmd_queue.lock().await;
        for _ in 0..num {
            let cmd = KVCommand::Put(KeyValue {
                key: random::<u64>().to_string(),
                value: random::<u64>().to_string(),
            });
            cmd_queue.push_front(cmd);
        }
    }

    async fn handle_scenario(&mut self, scenario_type: String) {
        match scenario_type.as_str() {
            "qloss" => {
                // Remove connections to everyone but next leader
                let current_leader = self
                    .max_round
                    .lock().await
                    .expect("Need to have a current leader for quorum loss scenario")
                    .leader;
                let next_leader = *self
                    .nodes
                    .iter()
                    .filter(|&&n| n != current_leader)
                    .next()
                    .unwrap();
                let other_nodes = self.nodes.iter().filter(|&&n| n != next_leader);
                let mut partitions = self.partitions.lock().await;
                partitions.clear();
                for &from in other_nodes {
                    for &to in self.nodes.iter() {
                        if to != next_leader && to != from {
                            let connection = self.get_connection(from, to);
                            partitions.insert(connection);
                        }
                    }
                }
                drop(partitions);
                self.send_network_update().await;
            }
            "constrained" => {
                // Disconnect next leader
                let current_leader = self
                    .max_round.lock().await
                    .expect("Need to have a current leader for constrained scenario")
                    .leader;
                let next_leader = *self
                    .nodes
                    .iter()
                    .filter(|&&n| n != current_leader)
                    .next()
                    .unwrap();
                let mut partitions = self.partitions.lock().await;
                partitions.clear();
                for &to in self.nodes.iter() {
                    if to != next_leader {
                        let connection = self.get_connection(next_leader, to);
                        partitions.insert(connection);
                    }
                }
                drop(partitions);
                self.send_network_update().await;
                // Decide some values without next leader
                self.batch_proposals(10).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
                // Set connections to Constrained Scenario with next leader
                let other_nodes = self.nodes.iter().filter(|&&n| n != next_leader);
                let mut partitions = self.partitions.lock().await;
                partitions.clear();
                for &from in other_nodes {
                    for &to in self.nodes.iter() {
                        if to != next_leader && to != from {
                            let connection = self.get_connection(from, to);
                            partitions.insert(connection);
                        }
                    }
                }
                partitions.insert(self.get_connection(next_leader, current_leader));
                drop(partitions);
                self.send_network_update().await;
            }
            "chained" => {
                let mut partitions = self.partitions.lock().await;
                partitions.clear();
                partitions.insert((1, 2));
                partitions.insert((1, 3));
                partitions.insert((1, 4));
                partitions.insert((2, 4));
                partitions.insert((2, 5));
                partitions.insert((3, 5));
                drop(partitions);
                self.send_network_update().await;
            }
            "restore" => {
                let mut partitions = self.partitions.lock().await;
                partitions.clear();
                drop(partitions);
                self.send_network_update().await;
            }
            _ => (),
        }
    }
}

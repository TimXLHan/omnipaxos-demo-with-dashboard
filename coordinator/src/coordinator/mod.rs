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

fn connection_to_port(from: &u64, to: &u64) -> u64 {
    8000 + (from * 10) + to
}

fn port_to_connection(port: &u64) -> (u64, u64) {
    let from = ((port / 10) % 10) as u64;
    let to = port % 10;
    match from <= to {
        true => (from, to),
        false => (to, from),
    }
}

lazy_static! {
    // Pids of nodes in the cluster
    static ref NODES: Vec<u64> = if let Ok(var) = env::var("NODES") {
        serde_json::from_str(&var).expect("wrong config format")
    } else {
        panic!("missing config");
    };

    /// Port to port mapping, for which sockets should be proxied to each other.
    pub static ref PORT_MAPPINGS: HashMap<u64, u64> = {
        let mut port_mappings = HashMap::new();
        let mut i = 0;
        for from in NODES.iter() {
            i += 1;
            for to in &NODES[i..] {
                let from_port = connection_to_port(from, to);
                let to_port = connection_to_port(to, from);
                port_mappings.insert(from_port, to_port);
                port_mappings.insert(to_port, from_port);
            }
        }
        port_mappings
    };

    /// Ports on which the nodes are supposed to connect with their client API socket.
    pub static ref CLIENT_PORTS: Vec<u64> = {
        NODES.iter().map(|pid| 8000 + pid).collect()
    };

    /// Mapping between PORT_MAPPING keys and CLIENT_PORTS
    pub static ref PORT_TO_PID_MAPPING: HashMap<u64, u64> = {
        let mut pid_mapping = HashMap::new();
        for node in NODES.iter() {
            let port = 8000 + node;
            pid_mapping.insert(port, *node);
        }
        for from in NODES.iter() {
            for to in NODES.iter() {
                if from != to {
                    let port = connection_to_port(from, to);
                    pid_mapping.insert(port, *from);
                }
            }
        }
        pid_mapping
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
    pub(crate) happiness: HashMap<u64, bool>,
    pub(crate) partitions: HashSet<(u64, u64)>,
    pub(crate) max_round: Option<Round>,
}

pub struct Coordinator {
    receiver: Receiver<CDMessage>,
    io_sender: Sender<IOMessage>,
    op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
    partitions: Arc<Mutex<HashSet<u64>>>,
    nodes: Vec<u64>,
    max_round: Arc<Mutex<Option<Round>>>,
    cmd_queue: Arc<Mutex<VecDeque<KVCommand>>>,
    happiness: HashMap<u64, bool>,
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
            happiness: HashMap::new(),
        }
    }

    async fn create_network_state(&self) -> NetworkState {
        let partitions: HashSet<(u64, u64)> = self
            .partitions
            .lock()
            .await
            .iter()
            .map(port_to_connection)
            .collect();
        NetworkState {
            nodes: self.nodes.clone(),
            alive_nodes: self
                .op_sockets
                .lock()
                .await
                .keys()
                .map(|&key| key)
                .collect(),
            partitions,
            max_round: self.max_round.lock().await.clone(),
            happiness: self.happiness.clone(),
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
                                Message::APIResponse(APIResponse::Happiness(happy)) => sender
                                    .send(IOMessage::CDMessage(CDMessage::Happiness(
                                                client_pid, happy,
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

    async fn create_network_actor(partitions: Arc<Mutex<HashSet<u64>>>) {
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
                let nodes_are_connected = !partitions.lock().await.contains(&from_port);
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
                    let mut proposer = ProposalStreamer::new(
                        self.io_sender.clone(),
                        self.op_sockets.clone(),
                        self.cmd_queue.clone(),
                        self.max_round.clone(),
                    );
                    tokio::spawn(async move { proposer.run().await });


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
                        match to {
                            Some(to) => {
                                self.set_partition(from, to, is_connected).await;
                                self.send_network_update().await;
                            }
                            None => {
                                let other_nodes = self.nodes.iter().filter(|&&n| n != from);
                                for to in other_nodes {
                                    self.set_partition(from, *to, is_connected).await;
                                }
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
                }
                CDMessage::Scenario(scenario_type) => {
                    assert!(
                        self.nodes.len() == 5,
                        "Must have 5 nodes to execute scenarios"
                    );
                    self.handle_scenario(scenario_type).await;
                }
                CDMessage::Happiness(client_pid, happy) => {
                    self.happiness.insert(client_pid, happy);
                    self.send_network_update().await;
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

    async fn set_partition(&self, from: u64, to: u64, is_connected: bool) {
        // UI can only display undirected connections, so we add partitions in both
        // connection directions
        let from_port = connection_to_port(&from, &to);
        let to_port = connection_to_port(&to, &from);
        let mut partitions = self.partitions.lock().await;
        if is_connected {
            partitions.remove(&from_port);

            partitions.remove(&to_port);
        } else {
            partitions.insert(from_port);
            partitions.insert(to_port);
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
                    .lock()
                    .await
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
                drop(partitions);
                for &from in other_nodes {
                    for &to in self.nodes.iter() {
                        if to != next_leader && to != from {
                            self.set_partition(from, to, false).await;
                        }
                    }
                }
                self.send_network_update().await;
            }
            "constrained" => {
                // Disconnect next leader
                let current_leader = self
                    .max_round
                    .lock()
                    .await
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
                drop(partitions);
                for &to in self.nodes.iter() {
                    if to != next_leader {
                        self.set_partition(next_leader, to, false).await;
                    }
                }
                self.send_network_update().await;
                // Decide some values without next leader
                self.batch_proposals(10).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
                // Set connections to Constrained Scenario with next leader
                let other_nodes = self.nodes.iter().filter(|&&n| n != next_leader);
                let mut partitions = self.partitions.lock().await;
                partitions.clear();
                drop(partitions);
                for &from in other_nodes {
                    for &to in self.nodes.iter() {
                        if to != next_leader && to != from {
                            self.set_partition(from, to, false).await;
                        }
                    }
                }
                self.set_partition(next_leader, current_leader, false).await;
                self.send_network_update().await;
            }
            "chained" => {
                let mut partitions = self.partitions.lock().await;
                partitions.clear();
                drop(partitions);
                self.set_partition(1, 2, false).await;
                self.set_partition(1, 3, false).await;
                self.set_partition(1, 4, false).await;
                self.set_partition(2, 4, false).await;
                self.set_partition(2, 5, false).await;
                self.set_partition(3, 5, false).await;
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

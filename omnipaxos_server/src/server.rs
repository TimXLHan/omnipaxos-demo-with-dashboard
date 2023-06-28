use crate::database::Database;
use crate::kv::KVCommand;
use crate::{
    network::{Message, Network},
    util::{ELECTION_TIMEOUT, OUTGOING_MESSAGE_PERIOD},
    OmniPaxosKV,
};
use omnipaxos::ballot_leader_election::Ballot;
use omnipaxos::util::{LogEntry, NodeId};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::time;

#[derive(Clone, Copy, Eq, Debug, Ord, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Round {
    pub round_num: u32,
    pub leader: u64,
}

impl From<Ballot> for Round {
    fn from(ballot: Ballot) -> Self {
        Self {
            round_num: ballot.n,
            leader: ballot.pid,
        }
    }
}

/// Same as in network actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIResponse {
    Decided(u64),
    Read(String, Option<String>),
    NewRound(Option<Round>),
}

pub struct OmniPaxosServer {
    pub omni_paxos: Arc<Mutex<OmniPaxosKV>>,
    pub pid: NodeId,
    pub peers: Vec<NodeId>,
    pub last_sent_decided_idx: u64,
    pub last_sent_leader: Option<Ballot>,
    pub network: Network,
    pub database: Database,
}

impl OmniPaxosServer {
    fn handle_decided(&self, decided_entries: Option<Vec<LogEntry<KVCommand>>>) {
        if let Some(decided_entries) = decided_entries {
            for entry in decided_entries {
                match entry {
                    LogEntry::Decided(cmd) => {
                        self.database.handle_command(cmd);
                    }
                    LogEntry::Snapshotted(_s) => unimplemented!(),
                    _ => {}
                }
            }
        }
    }

    async fn send_outgoing_msgs(&mut self) {
        let messages = self.omni_paxos.lock().unwrap().outgoing_messages();
        for msg in messages {
            let receiver = msg.get_receiver();
            self.network
                .send(receiver, Message::OmniPaxosMsg(msg))
                .await;
        }
    }

    async fn process_incoming_msgs(&mut self) {
        let messages = self.network.get_received().await;
        let mut op = self.omni_paxos.lock().unwrap();
        for msg in messages {
            match msg {
                Message::OmniPaxosMsg(m) => op.handle_incoming(m),
                Message::APICommand(KVCommand::Get(key)) => {
                    let value = self.database.handle_command(KVCommand::Get(key.clone()));
                    let msg = Message::APIResponse(APIResponse::Read(key, value));
                    self.network.send(0, msg).await;
                }
                Message::APICommand(cmd) => op.append(cmd).unwrap(),
                Message::APIResponse(_) => panic!("received API response"),
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        let mut msg_interval = time::interval(OUTGOING_MESSAGE_PERIOD);
        let mut election_interval = time::interval(ELECTION_TIMEOUT);
        loop {
            tokio::select! {
                biased;
                _ = election_interval.tick() => { self.omni_paxos.lock().unwrap().tick(); },
                _ = msg_interval.tick() => {
                    self.process_incoming_msgs().await;
                    self.send_outgoing_msgs().await;
                    // Notify the network_actor of new decided idx
                    let op = self.omni_paxos.lock().unwrap();
                    let new_decided_idx = op.get_decided_idx();
                    if self.last_sent_decided_idx < new_decided_idx {
                        let decided_entries = op.read_decided_suffix(self.last_sent_decided_idx);
                        self.handle_decided(decided_entries);
                        self.last_sent_decided_idx = new_decided_idx;
                        let msg = Message::APIResponse(APIResponse::Decided(new_decided_idx));
                        self.network.send(0, msg).await;
                    }
                    // Notify the network_actor of new leader
                    let new_ballot = op.get_current_leader_ballot();
                    if self.last_sent_leader != new_ballot {
                        self.last_sent_leader = new_ballot;
                        let msg = Message::APIResponse(APIResponse::NewRound(new_ballot.map(|b| b.into())));
                        self.network.send(0, msg).await;
                    }
                },
                else => (),
            }
        }
    }
}

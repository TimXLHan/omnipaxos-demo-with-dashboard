use crate::database::Database;
use crate::kv::KVCommand;
use crate::{
    network::{Message, Network},
    util::{ELECTION_TIMEOUT, OUTGOING_MESSAGE_PERIOD},
    OmniPaxosKV,
};
use omnipaxos::util::{LogEntry, NodeId};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::time;

/// Same as in network actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIResponse {
    Decided(u64),
    Read(String, Option<String>),
}

pub struct OmniPaxosServer {
    // pub omni_paxos: Arc<Mutex<OmniPaxosKV>>,
    // pub pid: NodeId,
    // pub peers: Vec<NodeId>,
    // pub last_sent_decided_idx: u64,
    pub network: Network,
    pub database: Database,
}

impl OmniPaxosServer {

    async fn process_incoming_msgs(&mut self) {
        let messages = self.network.get_received().await;
        for msg in messages {
            match msg {
                Message::APICommand(KVCommand::Get(key)) => {
                    let value = self.database.handle_command(KVCommand::Get(key.clone()));
                    let msg = Message::APIResponse(APIResponse::Read(key, value));
                    self.network.send(0, msg).await;
                }
                Message::APICommand(cmd) => {
                    self.database.handle_command(cmd);
                },
                _ => panic!("received unexpected message"),
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        let mut msg_interval = time::interval(OUTGOING_MESSAGE_PERIOD);
        loop {
            tokio::select! {
                biased;
                _ = msg_interval.tick() => {
                    self.process_incoming_msgs().await;
                },
                else => (),
            }
        }
    }
}

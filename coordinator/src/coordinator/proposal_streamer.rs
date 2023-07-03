use std::{sync::Arc, collections::{HashMap, VecDeque}, time::Duration};
use tokio::io::AsyncWriteExt;
use tokio::{sync::{mpsc::Sender, Mutex}, net::tcp::OwnedWriteHalf};

use crate::messages::{coordinator::{Message, KVCommand}, IOMessage};

const PROPOSE_TICK_RATE: Duration = Duration::from_millis(10);

pub struct ProposalStreamer {
    io_sender: Sender<IOMessage>,
    op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
    cmd_queue: Arc<Mutex<VecDeque<KVCommand>>>,
}

impl ProposalStreamer {
    pub fn new(io_sender: Sender<IOMessage>, op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>, cmd_queue: Arc<Mutex<VecDeque<KVCommand>>>) -> Self {
        Self {
            io_sender,
            op_sockets,
            cmd_queue,
        }
    }

    pub async fn propose_command(&self, cmd: KVCommand) {
        // TODO: If we partition such that op_sockets next() gives us a node that is not
        // connected to the leader, then we won't be able to send any commands. Always
        // send to leader instead?
        if let Some((_, writer)) = self.op_sockets.lock().await.iter_mut().next() {
            let request = Message::APIRequest(cmd);
            let mut data = serde_json::to_vec(&request).expect("could not serialize cmd");
            data.push(b'\n');
            writer.write_all(&data).await.unwrap();
        } else {
            self.io_sender.send(IOMessage::UIMessage(crate::messages::ui::UIMessage::ClusterUnreachable)).await.unwrap();
        }
    }

    pub async fn run(&mut self) {
        let mut propose_interval = tokio::time::interval(PROPOSE_TICK_RATE);
        loop {
            tokio::select! {
                _ = propose_interval.tick() => {
                    if let Some(cmd) = self.cmd_queue.lock().await.pop_back() {
                        self.propose_command(cmd).await;
                    }
                },
            }
        }
    }
}


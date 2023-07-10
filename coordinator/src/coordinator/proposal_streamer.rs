use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncWriteExt;
use tokio::{
    net::tcp::OwnedWriteHalf,
    sync::{mpsc::Sender, Mutex},
};

use crate::messages::{
    coordinator::{KVCommand, Message, Round},
    ui::UIMessage,
    IOMessage,
};

const PROPOSE_TICK_RATE: Duration = Duration::from_millis(10);

pub struct ProposalStreamer {
    io_sender: Sender<IOMessage>,
    op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
    cmd_queue: Arc<Mutex<VecDeque<KVCommand>>>,
    max_round: Arc<Mutex<Option<Round>>>,
    last_queue_size: usize,
    current_batch_size: usize,
    currently_batching: bool,
}

impl ProposalStreamer {
    pub fn new(
        io_sender: Sender<IOMessage>,
        op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
        cmd_queue: Arc<Mutex<VecDeque<KVCommand>>>,
        max_round: Arc<Mutex<Option<Round>>>,
    ) -> Self {
        Self {
            io_sender,
            op_sockets,
            cmd_queue,
            max_round,
            last_queue_size: 0,
            current_batch_size: 0,
            currently_batching: false,
        }
    }

    pub async fn propose_command(&self, cmd: KVCommand) {
        let leader = (*self.max_round.lock().await).unwrap().leader;
        if let Some(writer) = self.op_sockets.lock().await.get_mut(&leader) {
            let request = Message::APIRequest(cmd);
            let mut data = serde_json::to_vec(&request).expect("could not serialize cmd");
            data.push(b'\n');
            writer.write_all(&data).await.unwrap();
        } else {
            self.io_sender
                .send(IOMessage::UIMessage(UIMessage::ClusterUnreachable))
                .await
                .unwrap();
        }
    }

    async fn send_new_batch_size(&self) {
        self.io_sender.send(IOMessage::UIMessage(UIMessage::ProposalStatus(self.current_batch_size as u16))).await.unwrap();
    }

    pub async fn run(&mut self) {
        let mut propose_interval = tokio::time::interval(PROPOSE_TICK_RATE);
        loop {
            tokio::select! {
                _ = propose_interval.tick() => {
                    let mut queue = self.cmd_queue.lock().await;
                    let mut queue_len = queue.len();
                    if queue_len > self.last_queue_size {
                        // Must have batched new proposals
                        self.current_batch_size += queue_len - self.last_queue_size;
                        self.currently_batching = true;
                        self.send_new_batch_size().await;
                    } else if self.currently_batching && queue_len == 0 {
                        // Queue just became empty
                        self.currently_batching = false;
                        self.current_batch_size = 0; // Signal to UI that batching is "finished"
                        self.send_new_batch_size().await;
                    }
                    if let Some(cmd) = queue.pop_back() {
                        self.propose_command(cmd).await;
                        queue_len -= 1;
                    }
                    self.last_queue_size = queue_len;
                },
            }
        }
    }
}

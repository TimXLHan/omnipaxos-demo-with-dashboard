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
    IOMessage,
};

const PROPOSE_TICK_RATE: Duration = Duration::from_millis(10);

pub struct ProposalStreamer {
    io_sender: Sender<IOMessage>,
    op_sockets: Arc<Mutex<HashMap<u64, OwnedWriteHalf>>>,
    cmd_queue: Arc<Mutex<VecDeque<KVCommand>>>,
    max_round: Arc<Mutex<Option<Round>>>,
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
                .send(IOMessage::UIMessage(
                    crate::messages::ui::UIMessage::ClusterUnreachable,
                ))
                .await
                .unwrap();
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

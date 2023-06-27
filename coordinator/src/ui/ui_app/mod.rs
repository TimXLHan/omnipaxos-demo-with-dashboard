use crate::coordinator::NetworkState;
use crate::messages::IOMessage;
use tokio::sync::mpsc::Sender;
use tui_textarea::TextArea;

pub mod cli;
pub mod render;

/// The ui application, containing the ui state
pub struct UIApp<'a> {
    io_sender: Sender<IOMessage>,
    logs: Vec<String>,
    pub scroll: i64,
    pub input_area: TextArea<'a>,
    pub network_state: NetworkState,
    pub throughput_data: Vec<u64>,
    pub decided_idx: u64,
}

impl<'a> UIApp<'a> {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self {
            io_sender,
            input_area: TextArea::default(),
            logs: vec![],
            scroll: 0,
            network_state: NetworkState {
                nodes: vec![],
                alive_nodes: vec![],
                partitions: Default::default(),
                leaders: Default::default(),
            },
            throughput_data: vec![],
            decided_idx: 0,
        }
    }

    pub fn append_log(&mut self, log: String) {
        self.logs.push(log);
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.clone()
    }
}

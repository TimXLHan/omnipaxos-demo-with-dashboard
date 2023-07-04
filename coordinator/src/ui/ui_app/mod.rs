use crate::coordinator::NetworkState;
use crate::messages::IOMessage;
use tokio::sync::mpsc::Sender;
use tui_textarea::TextArea;

pub mod cli;
pub mod render;

pub struct Progress {
    pub starting_idx: u64,
    pub is_ongoing: bool,
    pub finished: u16,
    pub total: u16,
}

/// The ui application, containing the ui state
pub struct UIApp<'a> {
    io_sender: Sender<IOMessage>,
    logs: Vec<String>,
    pub scroll: i64,
    pub input_area: TextArea<'a>,
    pub network_state: NetworkState,
    pub throughput_data: Vec<(String, u64)>,
    pub decided_idx: u64,
    // Progress of the batch: (finished, total)
    pub progress: Progress,
}

impl<'a> UIApp<'a> {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self {
            io_sender,
            input_area: TextArea::default(),
            logs: vec![],
            scroll: 0,
            network_state: Default::default(),
            throughput_data: vec![],
            decided_idx: 0,
            // progress: (89, 166),
            progress: Progress {
                starting_idx: 0,
                is_ongoing: false,
                finished: 100,
                total: 100,
            },
        }
    }

    pub fn append_log(&mut self, log: String) {
        self.logs.push(log);
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.clone()
    }
}

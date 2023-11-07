use crate::coordinator::NetworkState;
use crate::messages::IOMessage;
use crate::utils::COLORS;
use ratatui::style::Color;
use tokio::sync::mpsc::Sender;
use tui_textarea::TextArea;

pub mod cli;
pub mod render;

pub struct Progress {
    pub starting_idx: u64,
    pub is_ongoing: bool,
    pub finished: u64,
    pub total: u64,
}

/// Basic information of a node.
#[derive(Debug, Clone, Default)]
pub struct Node {
    pub(crate) pid: u64,
    pub(crate) color: Color,
}

/// The ui application, containing the ui state
pub struct UIApp<'a> {
    #[allow(dead_code)]
    io_sender: Sender<IOMessage>,
    logs: Vec<String>,
    pub scroll: i64,
    pub input_area: TextArea<'a>,
    pub network_state: NetworkState,
    pub decided_data: Vec<(String, u64)>,
    pub decided_idx: u64,
    pub progress: Progress,
    /// Ids of all the nodes in the cluster specified in the configuration.
    pub nodes: Vec<Node>,
    pub leader: Option<Node>,
    pub(crate) throughput: f64,
}

impl<'a> UIApp<'a> {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self {
            io_sender,
            input_area: TextArea::default(),
            logs: vec![],
            scroll: 0,
            network_state: Default::default(),
            decided_data: vec![],
            decided_idx: 0,
            progress: Progress {
                starting_idx: 0,
                is_ongoing: false,
                finished: 0,
                total: 0,
            },
            nodes: vec![],
            leader: None,
            throughput: 0.0,
        }
    }

    pub fn append_log(&mut self, log: String) {
        self.logs.push(log);
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.clone()
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    pub fn set_network_state(&mut self, network_state: NetworkState) {
        // set up nodes if first time
        if self.nodes.is_empty() {
            for (idx, &pid) in network_state.nodes.iter().enumerate() {
                self.nodes.push(Node {
                    pid,
                    color: COLORS[idx % COLORS.len()],
                });
            }
        }
        // set leader
        if let Some(round) = network_state.max_round {
            let leader_node = self
                .nodes
                .iter()
                .find(|node| node.pid == round.leader)
                .unwrap();
            self.leader = Some(leader_node.clone());
        }
        self.network_state = network_state;
    }
}

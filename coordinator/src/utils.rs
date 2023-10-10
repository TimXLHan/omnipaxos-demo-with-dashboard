use std::time::Duration;
use ratatui::prelude::Color;

pub const CHANNEL_BUFFER_SIZE: usize = 100;
pub const UI_UPDATE_INTERVAL: u64 = 50;
pub const UI_TITLE: &str = "OmniPaxos Demo With Tui";
pub const UI_INPUT_AREA_TITLE: &str = "Input(Enter to send)";
pub const UI_THROUGHPUT_TITLE: &str = "Throughput";
pub const UI_OUTPUT_AREA_TITLE: &str = "Output(Scroll with Up/Down, Quit with Ctrl-C, Help with help)";
pub const UI_TICK_RATE: Duration = Duration::from_millis(100);
pub const UI_MAX_THROUGHPUT_SIZE: usize = 200;
pub const UI_PROGRESS_BAR_TITLE: &str = "Progress(Finished/Total)";
pub const UI_BARCHART_WIDTH: u16 = 3;
pub const UI_BARCHART_GAP: u16 = 1;
pub const UI_LEADER_RECT_COLOR: Color = Color::Green;
pub(crate) const COLORS: [Color; 9] = [
    Color::Green,
    Color::Blue,
    Color::Indexed(27), // Blue
    Color::Cyan,
    Color::Yellow,
    Color::Indexed(211), // Pink
    Color::Indexed(208), // Orange
    Color::Magenta,
    Color::Red,
];

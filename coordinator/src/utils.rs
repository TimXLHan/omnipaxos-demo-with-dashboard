use ratatui::prelude::Color;
use std::time::Duration;

pub const PROPOSE_TICK_RATE: Duration = Duration::from_millis(1);
pub const CHANNEL_BUFFER_SIZE: usize = 100000;
pub const UI_TITLE: &str = "The OmniPaxos Playground (press 'q' or 'esc' to exit)";
pub const UI_INPUT_AREA_TITLE: &str = "Input (Enter to send)";
pub const UI_THROUGHPUT_TITLE: &str = "Throughput";
pub const UI_OUTPUT_AREA_TITLE: &str =
    "Output (Scroll with Up/Down, Quit with Ctrl-C, Help with help)";
pub const UI_TICK_RATE: Duration = Duration::from_millis(100);
pub const UI_MAX_DECIDED_BARS: usize = 200;
pub const UI_PROGRESS_BAR_TITLE: &str = "Progress (Finished/Total)";
pub const UI_BARCHART_WIDTH: u16 = 3;
pub const UI_BARCHART_GAP: u16 = 1;
pub const UI_LEADER_RECT_COLOR: Color = Color::Green;
pub const ORANGE: Color = Color::Indexed(208);
pub const PINK: Color = Color::Indexed(211);

pub(crate) const COLORS: [Color; 8] = [
    Color::Green,
    Color::Blue,
    Color::Red,
    ORANGE,
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    PINK,
];

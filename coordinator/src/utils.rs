use std::time::Duration;

pub const CHANNEL_BUFFER_SIZE: usize = 100;
pub const UI_UPDATE_INTERVAL: u64 = 50;
pub const UI_TITLE: &str = "OmniPaxos Demo With Tui";
pub const UI_INPUT_AREA_TITLE: &str = "Input(Enter to send)";
pub const UI_THROUGHPUT_TITLE: &str = "Throughput(Req/100ms)";
pub const UI_OUTPUT_AREA_TITLE: &str = "Output(Scroll with Up/Down, Quit with Ctrl-C)";
pub const UI_TICK_RATE: Duration = Duration::from_millis(100);
pub const UI_MAX_THROUGHPUT_SIZE: usize = 200;
pub const UI_PROGRESS_BAR_TITLE: &str = "Progress(Finished/Total)";
pub const UI_BARCHART_WIDTH: u16 = 3;
pub const UI_BARCHART_GAP: u16 = 1;

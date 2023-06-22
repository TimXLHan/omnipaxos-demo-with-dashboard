use std::collections::HashSet;
use std::time::Duration;
use tokio::join;
use tokio::sync::mpsc;
use tokio::time::{Sleep, sleep};
use crate::coordinator::NetworkState;

use crate::messages::{coordinator::CDMessage, IOMessage, ui::UIMessage};
use crate::utils::*;

#[macro_use]
extern crate lazy_static;

mod controller;
mod coordinator;
mod messages;
mod ui;
mod utils;

#[tokio::main]
async fn main() {
    let (io_sender, mut io_receiver) = mpsc::channel::<IOMessage>(CHANNEL_BUFFER_SIZE);
    let (cd_sender, cd_receiver) = mpsc::channel::<CDMessage>(CHANNEL_BUFFER_SIZE);
    let mut ui = ui::UI::new(io_sender.clone());
    let mut cd = coordinator::Coordinator::new(cd_receiver, io_sender.clone());
    let mut controller = controller::Controller::new(ui, io_receiver, cd_sender);

    io_sender.send(IOMessage::UIMessage(UIMessage::Initialize)).await.unwrap();
    join!(cd.run(), controller.run());
    // join!(controller.run());

}

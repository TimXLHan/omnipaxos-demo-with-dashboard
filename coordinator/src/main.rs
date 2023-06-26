use tokio::join;
use tokio::sync::mpsc;

use crate::messages::{coordinator::CDMessage, ui::UIMessage, IOMessage};
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
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        std::process::exit(1);
    }));
    let (io_sender, io_receiver) = mpsc::channel::<IOMessage>(CHANNEL_BUFFER_SIZE);
    let (cd_sender, cd_receiver) = mpsc::channel::<CDMessage>(CHANNEL_BUFFER_SIZE);
    let ui = ui::UI::new(io_sender.clone());
    let mut cd = coordinator::Coordinator::new(cd_receiver, io_sender.clone());
    let mut controller = controller::Controller::new(ui, io_receiver, cd_sender);

    io_sender
        .send(IOMessage::UIMessage(UIMessage::Initialize))
        .await
        .unwrap();
    io_sender
        .send(IOMessage::CDMessage(CDMessage::Initialize))
        .await
        .unwrap();

    join!(cd.run(), controller.run());
}

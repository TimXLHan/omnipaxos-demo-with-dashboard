use crate::messages::db::DBMessage;
use crate::messages::IOMessage;
use crate::messages::ui::UIMessage;
use crate::utils::*;

mod utils;
mod messages;
mod controller;
mod db;
mod ui;

#[tokio::main]
async fn main() {
    let (io_sender, mut io_receiver) = tokio::sync::mpsc::channel::<IOMessage>(CHANNEL_BUFFER_SIZE);
    let mut ui = ui::UI::new(io_sender.clone());
    let mut db = db::DB::new(io_sender.clone());
    let mut controller = controller::Controller::new(ui, db, io_receiver);

    io_sender.send(IOMessage::UIMessage(UIMessage::Initialize)).await.unwrap();
    io_sender.send(IOMessage::DBMessage(DBMessage::Initialize)).await.unwrap();

    tokio::spawn(async move {
        controller.run().await;
    }).await.expect("Error in controller::run()");


}

use chrono::{DateTime, Utc};
use tokio::sync::mpsc::Sender;
use crate::messages::coordinator::CDMessage;
use crate::messages::IOMessage;

pub struct CLIHandler {
    io_sender: Sender<IOMessage>,
}

impl CLIHandler {
    pub fn new(io_sender: Sender<IOMessage>) -> Self {
        Self { io_sender }
    }

    // Handle the user input and return a output string (can be empty) to be displayed on the output area.
    pub fn handle_user_input(&mut self, input: String) -> String {
        // Example of how to handle input and send a message to the coordinator
        // match input {
        //     append_a_value => {
        //         self.io_sender.send(IOMessage::CDMessage(CDMessage::KVCommand(value));
        //     }
        // }

        if input.is_empty() {
            return String::new();
        }

        let now: String = Utc::now().format("%F %T> ").to_string();
         now + &*input
    }
}



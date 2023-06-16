use std::fmt;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc::Sender;
use crate::coordinator::KeyValue;
use crate::messages::coordinator::{CDMessage, KVCommand};
use crate::messages::IOMessage;

pub struct CLIHandler {
    io_sender: Sender<IOMessage>,
}

impl CLIHandler {
    pub fn new(io_sender: Sender<IOMessage>) -> Self {
        Self { io_sender }
    }

    // Handle the user input and return a output string (can be empty) to be displayed on the output area.
    pub async fn handle_user_input(&mut self, input: String) -> String {
        match parse_command(input) {
            Ok(io_message) => {
                self.io_sender.send(io_message).await.unwrap();
                String::new()
            },
            Err(err) => {
                //let now: String = Utc::now().format("%F %T> ").to_string();
                //now + &*input
                format!("{err}")
            }
        }
    }
}

struct ParseCommandError(String);
impl fmt::Display for ParseCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
fn parse_command(line: String) -> Result<IOMessage, ParseCommandError> {
    let mut words = line.trim().split(" ");
    let command_type = words
        .next()
        .ok_or(ParseCommandError("Invalid command: valid commands are put/get/delete".to_string()))?;

    let command = match command_type {
        "delete" => {
            let value = words
                .next()
                .ok_or(ParseCommandError("Invalid command, format is: delete <key-to-delete>".to_string()))?;
            IOMessage::CDMessage(CDMessage::KVCommand(KVCommand::Delete(value.to_string())))
        },
        "get" => {
            let value = words
                .next()
                .ok_or(ParseCommandError("Invalid command, format is: get <key-to-get>".to_string()))?;
            IOMessage::CDMessage(CDMessage::KVCommand(KVCommand::Get(value.to_string())))
        },
        "put" => {
            let key = words
                .next()
                .ok_or(ParseCommandError("Invalid command, format is: put <key> <value>".to_string()))?
                .to_string();
            let value = words
                .next()
                .ok_or(ParseCommandError("Invalid command, format is: put <key> <value>".to_string()))?
                .to_string();
            IOMessage::CDMessage(CDMessage::KVCommand(KVCommand::Put(KeyValue { key, value })))
        },
        _ => Err(ParseCommandError("Invalid command: valid commands are put/get/delete".to_string()))?,
    };
    Ok(command)
}


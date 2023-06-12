use crate::messages::db::DBMessage;
use crate::messages::ui::UIMessage;

pub mod db{
    #[derive(Debug, Clone)]
    pub enum DBMessage {
        Initialize,      // Launch to initialize the application
    }
}

pub mod ui{
    #[derive(Debug, Clone)]
    pub enum UIMessage {
        Initialize,      // Launch to initialize the application
    }
}

#[derive(Debug, Clone)]
pub enum IOMessage {
    DBMessage(DBMessage),
    UIMessage(UIMessage),
}
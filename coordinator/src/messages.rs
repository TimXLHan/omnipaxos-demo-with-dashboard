use crate::messages::coordinator::CDMessage;
use crate::messages::ui::UIMessage;

pub mod coordinator {
    #[derive(Debug, Clone)]
    pub enum CDMessage {
        Initialize, // Launch to initialize the application
    }
}

pub mod ui {
    #[derive(Debug, Clone)]
    pub enum UIMessage {
        Initialize, // Launch to initialize the application
    }
}

#[derive(Debug, Clone)]
pub enum IOMessage {
    CDMessage(CDMessage),
    UIMessage(UIMessage),
}

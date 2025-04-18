use std::sync::mpsc::{channel, Receiver, Sender};

/// Contains the text of the file and a channel to communicate with the file panel.
pub struct FileHandler {
    pub text_channel: (Sender<String>, Receiver<String>),
    pub text: String,
}

impl FileHandler {
    pub fn get_sender_cloned(&self) -> Sender<String> {
        self.text_channel.0.clone()
    }

    pub fn try_recv(&self) -> Result<String, std::sync::mpsc::TryRecvError> {
        self.text_channel.1.try_recv()
    }

    pub fn update(&mut self) {
        if let Ok(text) = self.try_recv() {
            self.text = text;
        }
    }
}

impl Default for FileHandler {
    fn default() -> Self {
        Self {
            text_channel: channel(),
            text: "No file loaded".into(),
        }
    }
}

impl PartialEq for FileHandler {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

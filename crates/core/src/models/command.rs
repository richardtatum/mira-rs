use crate::ports::inbound::AsyncCallback;

pub enum Command {
    AddKey(String, AsyncCallback),
    // RemoveKey(String),
}

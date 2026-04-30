use crate::domain::scheduler::AsyncCallback;

pub enum Command {
    AddKey(String, AsyncCallback),
    // RemoveKey(String),
}

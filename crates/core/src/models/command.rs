use crate::ports::inbound::{AsyncCallback, SubscriptionToken};

pub enum Command {
    AddKey(String, SubscriptionToken, AsyncCallback),
    RemoveCallback(SubscriptionToken),
}

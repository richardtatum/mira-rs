pub mod domain;
pub mod models;
pub mod ports;

// re-export
pub use models::error::CoreError;
pub use models::persistence::{Host, StreamState, Subscription};
pub use models::status::{StreamInfo, StreamStatus};
pub use ports::inbound::AsyncCallback;
pub use ports::inbound::SubscriptionToken;
pub use ports::outbound::PersistenceProvider;
pub use ports::outbound::StreamStatusProvider;

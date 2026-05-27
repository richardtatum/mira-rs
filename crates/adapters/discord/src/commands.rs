use mira_core::PersistenceProvider;

pub mod subscribe;

// Expose all commands, extend this list as required
pub fn all<P: PersistenceProvider>() -> Vec<poise::Command<crate::types::Data<P>, crate::types::Error>> {
    vec![subscribe::subscribe()]
}

use mira_core::PersistenceProvider;

pub(crate) mod add_host;
pub(crate) mod subscribe;

// Expose all commands, extend this list as required
pub fn all<P: PersistenceProvider>() -> Vec<poise::Command<crate::types::Data<P>, crate::types::Error>> {
    vec![subscribe::subscribe(), add_host::add_host()]
}

pub mod subscribe;

// Expose all commands, extend this list as required
pub fn all() -> Vec<poise::Command<crate::types::Data, crate::types::Error>> {
    vec![subscribe::subscribe()]
}

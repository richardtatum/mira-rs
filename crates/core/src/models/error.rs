#[derive(Debug)]
pub enum CoreError {
    HttpError(String),
    ConfigError(String),
}

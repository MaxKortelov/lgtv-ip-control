use thiserror::Error;

#[derive(Debug, Error)]
pub enum LgTvError {
    #[error("key code is required")]
    MissingKeyCode,

    #[error("TV power is off")]
    PowerOff,

    #[error("could not parse response: {0}")]
    ParseError(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LgTvError {
    #[error("key code is required")]
    MissingKeyCode,

    #[error("tcp connection error: {0}")]
    TcpConnectionError(String),

    #[error("wake on lan error: {0}")]
    WakeOnLan(String),

    #[error("TV power is off")]
    PowerOff,

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Send command to TV failed: {0}")]
    SendCommand(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("RegEx error: {0}")]
    RegExpression(String),

    #[error("Could not parse current volume")]
    ParseVolumeError,

    #[error("could not parse integer: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("No matches found for mute state")]
    UnknownMuteState,

    #[error("Could not parse mute state")]
    UnableToParseMuteState,
}

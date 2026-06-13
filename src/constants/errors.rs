use thiserror::Error;

/// Error type used by `lgtv-ip-control`.
///
/// Most fallible crate operations return this error type.
#[derive(Debug, Error)]
pub enum LgTvError {
    /// Returned when no LG IP Control key code is provided.
    #[error("key code is required")]
    MissingKeyCode,

    /// Returned when opening, closing, or re-opening a TCP connection fails.
    #[error("tcp connection error: {0}")]
    TcpConnectionError(String),

    /// Returned when sending a Wake-on-LAN packet fails.
    #[error("wake on lan error: {0}")]
    WakeOnLan(String),

    /// Returned when the TV appears to be powered off.
    #[error("TV power is off")]
    PowerOff,

    /// Returned when command encryption fails.
    #[error("Encryption error: {0}")]
    EncryptionError(String),

    /// Returned when sending a command to the TV fails.
    #[error("Send command to TV failed: {0}")]
    SendCommand(String),

    /// Returned when decrypting a TV response fails.
    #[error("Decryption error: {0}")]
    DecryptionError(String),

    /// Returned when compiling or using a regular expression fails.
    #[error("RegEx error: {0}")]
    RegExpression(String),

    /// Returned when the current volume response cannot be parsed.
    #[error("Could not parse current volume")]
    ParseVolumeError,

    /// Returned when parsing a number fails.
    #[error("could not parse integer: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// Returned when the mute response contains an unknown state.
    #[error("No matches found for mute state")]
    UnknownMuteState,

    /// Returned when the mute response does not match the expected format.
    #[error("Could not parse mute state")]
    UnableToParseMuteState,
}

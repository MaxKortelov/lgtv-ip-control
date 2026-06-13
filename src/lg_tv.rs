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
use crate::constants::types::{
    AppDetails, Apps, ConnectionType, EnergySavingLevels, Inputs, Keys, PictureModes, PowerStates,
    ScreenMuteModes, VolumeLevel,
};
use crate::encryption::Encryption;
use crate::tcp_connection::{Connected, Disconnected, TcpConnection};
use regex::Regex;
use std::collections::HashMap;
use std::io::Error;
use tokio::time::{Duration, sleep};

#[trait_variant::make(CommandExecutor: Send)]
pub trait LocalCommandExecutor: Sized {
    async fn send_command(&mut self, command: Vec<u8>) -> Result<Vec<u8>, Error>;
    async fn wake_on_lan(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn disconnect(self) -> Result<Self, &'static str>;
    async fn reconnect(&mut self) -> Result<(), String>;
}

/// Client for controlling an LG TV over IP Control.
///
/// The client uses a typestate pattern:
///
/// - `LGTV<Disconnected>` represents a disconnected client.
/// - `LGTV<Connected>` represents a connected client.
///
/// Most control methods are only available after a successful connection.
pub struct LGTV<C, State = Disconnected> {
    executor: C,
    encryption: Encryption,
    _state: std::marker::PhantomData<State>,
}

impl LGTV<TcpConnection<Connected>, Disconnected> {
    /// Connects to an LG TV over IP Control.
    ///
    /// # Arguments
    ///
    /// - `tv_ip` - The IP address of the TV.
    /// - `mac_address` - The MAC address of the TV.
    /// - `key` - The LG IP Control key code.
    ///
    /// # Errors
    ///
    /// Returns [`LgTvError::MissingKeyCode`] if no key code is provided.
    /// Returns [`LgTvError::TcpConnectionError`] if the TCP connection fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lgtv_ip_control::LGTV;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let tv = LGTV::connect_tcp(
    ///     "192.168.1.100",
    ///     "AA:BB:CC:DD:EE:FF",
    ///     Some("YOUR_KEY_CODE"),
    /// )
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_tcp(
        tv_ip: &str,
        mac_address: &str,
        key: Option<&str>,
    ) -> Result<LGTV<TcpConnection<Connected>, Connected>, LgTvError> {
        let tcp_connection = match TcpConnection::new(tv_ip, mac_address).await {
            Ok(connection) => connection,
            Err(error) => return Err(LgTvError::TcpConnectionError(error.to_string())),
        };

        let lgtv_disconnected = Self::new(tcp_connection, key)?;

        Ok(lgtv_disconnected.transition_to_connected())
    }
}

impl<C: CommandExecutor> LGTV<C, Disconnected> {
    pub(crate) fn new(executor: C, key: Option<&str>) -> Result<Self, LgTvError> {
        let key_code = key
            .map(|k| k.to_string())
            .ok_or(LgTvError::MissingKeyCode)?;

        let encryption = Encryption::new(key_code);

        Ok(LGTV {
            executor,
            encryption,
            _state: std::marker::PhantomData,
        })
    }

    pub(crate) fn transition_to_connected(self) -> LGTV<C, Connected> {
        LGTV {
            executor: self.executor,
            encryption: self.encryption,
            _state: std::marker::PhantomData,
        }
    }
}

impl<C: CommandExecutor> LGTV<C, Connected> {
    /// Disconnects from the TV.
    ///
    /// Returns the client back in the disconnected state.
    pub async fn disconnect(self) -> Result<LGTV<C, Disconnected>, LgTvError> {
        let tcp_connection = match self.executor.disconnect().await {
            Ok(connection) => connection,
            Err(error) => return Err(LgTvError::TcpConnectionError(error.to_string())),
        };

        Ok(LGTV {
            executor: tcp_connection,
            encryption: self.encryption,
            _state: std::marker::PhantomData,
        })
    }

    /// Sends a Wake-on-LAN packet and waits for the TV to become available.
    ///
    /// If `retries` is `None`, a default retry count is used.
    pub async fn power_on(&mut self, retries: Option<u8>) -> Result<(), LgTvError> {
        let attempts_left = retries.unwrap_or(10);

        self.executor
            .wake_on_lan()
            .await
            .map_err(|error| LgTvError::WakeOnLan(error.to_string()))?;

        sleep(Duration::from_millis(500)).await;

        self.executor
            .reconnect()
            .await
            .map_err(LgTvError::TcpConnectionError)?;

        self.test_power_on(attempts_left).await
    }

    /// Attempts to power off the TV.
    ///
    /// If `retries` is `None`, a default retry count is used.
    pub async fn power_off(&mut self, retries: Option<u8>) -> Result<(), LgTvError> {
        let mut attempts_left = retries.unwrap_or(10);

        loop {
            if attempts_left == 0 {
                return Err(LgTvError::PowerOff);
            }

            // Send the command once per iteration of the main retry loop
            self.send_command("POWER off").await?;
            sleep(Duration::from_secs(1)).await;

            match self.test_power_on(1).await {
                Ok(_) => {
                    // TV is still on, decrement attempts and try again on the next loop cycle
                    attempts_left -= 1;
                }
                Err(_) => {
                    // TV failed the "power on" test, meaning it successfully turned off!
                    return Ok(());
                }
            }
        }
    }

    /// Returns information about the currently active app/input.
    ///
    /// # Errors
    ///
    /// Returns [`LgTvError::PowerOff`] if the TV returns an empty response.
    pub async fn get_current_app(&mut self) -> Result<AppDetails, LgTvError> {
        let result = self.send_command("CURRENT_APP").await?;

        match result.as_str() {
            "" => Err(LgTvError::PowerOff),
            _ => {
                let mut pairs: HashMap<String, String> = HashMap::new();
                let re = match Regex::new(r"([\w\s]+):(\S+)") {
                    Ok(re) => re,
                    Err(error) => return Err(LgTvError::RegExpression(error.to_string())),
                };

                for captures in re.captures_iter(&result) {
                    pairs.insert(captures[1].trim().to_string(), captures[2].to_string());
                }

                Ok(AppDetails {
                    app: pairs.get("APP").cloned().unwrap_or_default(),
                    hot_plug: pairs.get("Hot plug").cloned().unwrap_or_default(),
                    signal: pairs.get("Signal").cloned().unwrap_or_default(),
                    hdcp_version: pairs.get("HDCP").cloned().unwrap_or_default(),
                    hdcp_status: pairs.get("HDCP Status").cloned().unwrap_or_default(),
                })
            }
        }
    }

    /// Returns the current volume level.
    ///
    /// The returned value is parsed from a TV response such as `VOL:25`.
    pub async fn get_current_volume(&mut self) -> Result<u8, LgTvError> {
        let result = self.send_command("CURRENT_VOL").await?;

        let re = match Regex::new(r"^VOL:(\d+)$") {
            Ok(re) => re,
            Err(error) => return Err(LgTvError::RegExpression(error.to_string())),
        };

        let capture = re.captures(&result);

        match capture {
            Some(capture) => Ok(capture[1].parse::<u8>()?),
            None => Err(LgTvError::ParseVolumeError),
        }
    }

    /// Returns whether IP Control is enabled on the TV.
    pub async fn get_ip_control_state(&mut self) -> Result<bool, LgTvError> {
        let result = self.send_command("GET_IPCONTROL_STATE").await?;

        match result.as_str() {
            "ON" => Ok(true),
            _ => Ok(false),
        }
    }

    /// Launches an app on the TV.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lgtv_ip_control::{Apps, LGTV};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut tv = LGTV::new(
    ///     "192.168.1.100",
    ///     "AA:BB:CC:DD:EE:FF",
    ///     Some("YOUR_KEY_CODE"),
    /// )
    /// .await?;
    ///
    /// tv.launch_app(Apps::Youtube).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_mac_address(
        &mut self,
        connection_type: ConnectionType,
    ) -> Result<String, LgTvError> {
        let command = format!("GET_MACADDRESS {:?}", connection_type);
        let result = self.send_command(&command).await?;
        Ok(result)
    }

    pub async fn get_mute_state(&mut self) -> Result<bool, LgTvError> {
        let result = self.send_command("MUTE_STATE").await?;
        println!("{}", result);
        let re = match Regex::new(r"^MUTE:(on|off)$") {
            Ok(re) => re,
            Err(error) => return Err(LgTvError::RegExpression(error.to_string())),
        };

        if let Some(captures) = re.captures(&result) {
            match &captures[1] {
                "on" => Ok(true),
                "off" => Ok(false),
                _ => Err(LgTvError::UnknownMuteState),
            }
        } else {
            Err(LgTvError::UnableToParseMuteState)
        }
    }

    pub async fn get_power_state(&mut self) -> PowerStates {
        match self.test_power_on(5).await {
            Ok(_) => PowerStates::On,
            Err(_) => PowerStates::Off,
        }
    }

    pub async fn launch_app(&mut self, app: Apps) -> Result<(), LgTvError> {
        let command = format!("APP_LAUNCH {}", app.as_str());
        self.send_command(&command).await?;
        Ok(())
    }

    pub async fn set_picture_mode(&mut self, mode: PictureModes) -> Result<(), LgTvError> {
        let command = format!("PICTURE_MODE {}", mode.as_str());
        self.send_command(&command).await?;
        Ok(())
    }

    pub async fn send_key(&mut self, key: Keys) -> Result<(), LgTvError> {
        let command = format!("KEY_ACTION {}", key.as_str());
        self.send_command(&command).await?;
        Ok(())
    }

    pub async fn set_screen_mute(&mut self, mode: ScreenMuteModes) -> Result<(), LgTvError> {
        let command = format!("SCREEN_MUTE {}", mode.as_str());
        self.send_command(&command).await?;
        Ok(())
    }

    pub async fn set_energy_saving(&mut self, level: EnergySavingLevels) -> Result<(), LgTvError> {
        let command = format!("ENERGY_SAVING {}", level.as_str());
        self.send_command(&command).await?;
        Ok(())
    }

    pub async fn set_input(&mut self, input: Inputs) -> Result<(), LgTvError> {
        let command = format!("INPUT_SELECT {}", input.as_str());
        self.send_command(&command).await?;
        Ok(())
    }

    pub async fn set_volume(
        &mut self,
        level: VolumeLevel,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("VOLUME_CONTROL {}", level.value());
        self.send_command(&command).await?;
        Ok(())
    }

    pub async fn set_volume_mute(&mut self, is_muted: bool) -> Result<(), LgTvError> {
        let action = if is_muted { "on" } else { "off" };
        let command = format!("VOLUME_MUTE {action}");
        self.send_command(&command).await?;
        Ok(())
    }

    async fn send_command(&mut self, command: &str) -> Result<String, LgTvError> {
        let encrypted_command = match self.encryption.encrypt(command) {
            Ok(encrypted_command) => encrypted_command,
            Err(error) => return Err(LgTvError::EncryptionError(error.to_string())),
        };

        let result = match self.executor.send_command(encrypted_command).await {
            Ok(result) => result,
            Err(error) => return Err(LgTvError::SendCommand(error.to_string())),
        };

        match self.encryption.decrypt(&result) {
            Ok(decrypted_result) => Ok(decrypted_result),
            Err(error) => Err(LgTvError::DecryptionError(error.to_string())),
        }
    }

    async fn test_power_on(&mut self, retries: u8) -> Result<(), LgTvError> {
        let mut attempts_left = retries;
        loop {
            match self.get_current_app().await {
                Ok(app_details) if !app_details.app.is_empty() => return Ok(()),
                _ => {
                    if attempts_left == 0 {
                        return Err(LgTvError::PowerOff);
                    }
                    attempts_left -= 1;
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // =========================================================================
    // 1. Define the Mock Executor
    // =========================================================================
    /// A mock network layer tracking sent packets, state flags, and mock responses.
    #[derive(Clone)]
    struct MockExecutor {
        // Shared mutable state across the async trait methods
        inner: Arc<Mutex<MockInner>>,
    }

    struct MockInner {
        mock_response: String,
        wake_on_lan_called: bool,
        reconnect_called: bool,
        disconnect_called: bool,
        encryption_key: String,
        commands_sent: Vec<String>, // Tracks all decrypted commands passed down
        inject_error: Option<String>,
    }

    impl MockExecutor {
        fn new(initial_response: &str, encryption_key: &str) -> Self {
            Self {
                inner: Arc::new(Mutex::new(MockInner {
                    mock_response: initial_response.to_string(),
                    wake_on_lan_called: false,
                    reconnect_called: false,
                    disconnect_called: false,
                    encryption_key: encryption_key.to_string(),
                    commands_sent: Vec::new(),
                    inject_error: None,
                })),
            }
        }

        fn set_response(&self, new_response: &str) {
            if let Ok(mut inner) = self.inner.lock() {
                inner.mock_response = new_response.to_string();
            }
        }

        fn inject_error(&self, error_msg: &str) {
            if let Ok(mut inner) = self.inner.lock() {
                inner.inject_error = Some(error_msg.to_string());
            }
        }

        fn get_last_command(&self) -> Option<String> {
            let inner = self.inner.lock().unwrap();
            inner.commands_sent.last().cloned()
        }
    }

    // =========================================================================
    // 2. Implement CommandExecutor Contract for MockExecutor
    // =========================================================================
    impl CommandExecutor for MockExecutor {
        async fn send_command(&mut self, command: Vec<u8>) -> Result<Vec<u8>, Error> {
            let mut inner = self.inner.lock().unwrap();

            if let Some(ref err_msg) = inner.inject_error {
                return Err(std::io::Error::other(err_msg.clone()));
            }

            let mock_encryption = Encryption::new(inner.encryption_key.clone());
            let decrypted_command = mock_encryption.decrypt((command).as_ref()).unwrap();

            inner.commands_sent.push(decrypted_command);

            let encrypted_response = mock_encryption.encrypt(&inner.mock_response).unwrap();
            Ok(encrypted_response)
        }

        async fn wake_on_lan(&self) -> Result<(), Box<dyn std::error::Error>> {
            let mut inner = self.inner.lock().unwrap();
            if let Some(ref err_msg) = inner.inject_error {
                return Err(err_msg.clone().into());
            }
            inner.wake_on_lan_called = true;
            Ok(())
        }

        async fn disconnect(self) -> Result<Self, &'static str> {
            let inner_clone = self.inner.clone();
            let mut inner = inner_clone.lock().unwrap();
            if let Some(ref _err_msg) = inner.inject_error {
                return Err("Failed to disconnect");
            }
            inner.disconnect_called = true;
            Ok(self)
        }

        async fn reconnect(&mut self) -> Result<(), String> {
            let mut inner = self.inner.lock().unwrap();
            if let Some(ref err_msg) = inner.inject_error {
                return Err(err_msg.clone());
            }
            inner.reconnect_called = true;
            Ok(())
        }
    }

    // =========================================================================
    // Helper function to build connected TV instance cleanly
    // =========================================================================
    fn setup_connected_tv(mock: MockExecutor, key: &str) -> LGTV<MockExecutor, Connected> {
        let lgtv_disconnected = LGTV::new(mock, Some(key)).unwrap();
        lgtv_disconnected.transition_to_connected()
    }

    const TEST_KEY: &str = "TESTKEY123";

    // =========================================================================
    // Method Group: LGTV::new & Initialization Edge Cases
    // =========================================================================
    #[test]
    fn test_new_missing_key_code_error() {
        let mock_executor = MockExecutor::new("", TEST_KEY);
        let result = LGTV::new(mock_executor, None);

        assert!(matches!(result, Err(LgTvError::MissingKeyCode)));
    }

    // =========================================================================
    // Method Group: disconnect()
    // =========================================================================
    #[tokio::test]
    async fn test_disconnect_success() {
        let mock = MockExecutor::new("", TEST_KEY);
        let tracker = mock.clone();
        let tv = setup_connected_tv(mock, TEST_KEY);

        let _disconnected_tv = tv.disconnect().await.unwrap();
        assert!(tracker.inner.lock().unwrap().disconnect_called);
    }

    #[tokio::test]
    async fn test_disconnect_failure_error_mapping() {
        let mock = MockExecutor::new("", TEST_KEY);
        mock.inject_error("Network Dropped");
        let tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.disconnect().await;
        assert!(matches!(result, Err(LgTvError::TcpConnectionError(_))));
    }

    // =========================================================================
    // Method Group: power_on()
    // =========================================================================
    #[tokio::test]
    async fn test_power_on_success() {
        let mock = MockExecutor::new("APP:live_tv", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.power_on(Some(2)).await;
        assert!(result.is_ok());

        let inner = tracker.inner.lock().unwrap();
        assert!(inner.wake_on_lan_called);
        assert!(inner.reconnect_called);
    }

    #[tokio::test]
    async fn test_power_on_wol_fails() {
        let mock = MockExecutor::new("APP:live_tv", TEST_KEY);
        mock.inject_error("UDP socket failure");
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.power_on(Some(1)).await;
        assert!(matches!(result, Err(LgTvError::WakeOnLan(_))));
    }

    #[tokio::test]
    async fn test_power_on_timeout_exhausted() {
        // TV keeps returning blank data indicating it hasn't booted up yet
        let mock = MockExecutor::new("APP:", TEST_KEY);
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.power_on(Some(1)).await; // Limit to 1 retry cycle
        assert!(matches!(result, Err(LgTvError::PowerOff)));
    }

    // =========================================================================
    // Method Group: power_off()
    // =========================================================================
    #[tokio::test]
    async fn test_power_off_immediate_success() {
        // The first test loop check fails to find an app (meaning TV shut off immediately)
        let mock = MockExecutor::new("OFF", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.power_off(Some(3)).await;
        assert!(result.is_ok());
        assert_eq!(tracker.get_last_command().unwrap(), "CURRENT_APP");
    }

    #[tokio::test]
    async fn test_power_off_retries_exhausted() {
        // TV remains permanently on, continually returning active app
        let mock = MockExecutor::new("APP:hdmi1", TEST_KEY);
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.power_off(Some(2)).await;
        assert!(matches!(result, Err(LgTvError::PowerOff)));
    }

    // =========================================================================
    // Method Group: get_current_app()
    // =========================================================================
    #[tokio::test]
    async fn test_get_current_app_malformed_response() {
        // Corrupted response configuration missing structural colons
        let mock = MockExecutor::new("APP netflix HotPlug true", TEST_KEY);
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let details = tv.get_current_app().await.unwrap();
        // Values default to empty strings if regex fails to match key-value formats
        assert_eq!(details.app, "");
    }

    #[tokio::test]
    async fn test_get_current_app_empty_string_tv_off() {
        let mock = MockExecutor::new("", TEST_KEY);
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.get_current_app().await;
        assert!(matches!(result, Err(LgTvError::PowerOff)));
    }

    // =========================================================================
    // Method Group: get_current_volume()
    // =========================================================================
    #[tokio::test]
    async fn test_get_current_volume_parse_error() {
        // Volume value contains letters, preventing numerical extraction
        let mock = MockExecutor::new("VOL:abc", TEST_KEY);
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.get_current_volume().await;
        assert!(matches!(result, Err(LgTvError::ParseVolumeError)));
    }

    // =========================================================================
    // Method Group: get_ip_control_state()
    // =========================================================================
    #[tokio::test]
    async fn test_get_ip_control_state_variants() {
        let mock = MockExecutor::new("ON", TEST_KEY);
        let mut tv = setup_connected_tv(mock.clone(), TEST_KEY);
        assert!(tv.get_ip_control_state().await.unwrap());

        mock.set_response("OFF");
        assert!(!tv.get_ip_control_state().await.unwrap());
    }

    // =========================================================================
    // Method Group: get_mac_address()
    // =========================================================================
    #[tokio::test]
    async fn test_get_mac_address_command_formatting() {
        let mock = MockExecutor::new("AA:BB:CC:DD:EE:FF", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let mac = tv.get_mac_address(ConnectionType::Wired).await.unwrap();
        assert_eq!(mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(tracker.get_last_command().unwrap(), "GET_MACADDRESS Wired");
    }

    // =========================================================================
    // Method Group: get_mute_state()
    // =========================================================================
    #[tokio::test]
    async fn test_get_mute_state_variants() {
        let mock = MockExecutor::new("MUTE:on", TEST_KEY);
        let mut tv = setup_connected_tv(mock.clone(), TEST_KEY);
        assert!(tv.get_mute_state().await.unwrap());

        mock.set_response("MUTE:off");
        assert!(!tv.get_mute_state().await.unwrap());
    }

    #[tokio::test]
    async fn test_get_mute_state_invalid_payload_error() {
        let mock = MockExecutor::new("MUTE:unknown", TEST_KEY);
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.get_mute_state().await;

        assert!(matches!(result, Err(LgTvError::UnableToParseMuteState)));
    }

    #[tokio::test]
    async fn test_get_mute_state_regex_miss_error() {
        let mock = MockExecutor::new("INVALID_MUTE_FORMAT", TEST_KEY);
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.get_mute_state().await;
        assert!(matches!(result, Err(LgTvError::UnableToParseMuteState)));
    }

    // =========================================================================
    // Method Group: get_power_state()
    // =========================================================================
    #[tokio::test]
    async fn test_get_power_state_variants() {
        let mock = MockExecutor::new("APP:hdmi2", TEST_KEY);
        let mut tv = setup_connected_tv(mock.clone(), TEST_KEY);

        assert!(matches!(tv.get_power_state().await, PowerStates::On));

        mock.set_response("OFF");
        assert!(matches!(tv.get_power_state().await, PowerStates::Off));
    }

    // =========================================================================
    // Method Group: Setters, Actions, and State Modification Vectors
    // =========================================================================
    #[tokio::test]
    async fn test_launch_app_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.launch_app(Apps::Netflix).await.unwrap();
        assert_eq!(tracker.get_last_command().unwrap(), "APP_LAUNCH netflix");
    }

    #[tokio::test]
    async fn test_set_picture_mode_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.set_picture_mode(PictureModes::Cinema).await.unwrap();
        assert_eq!(tracker.get_last_command().unwrap(), "PICTURE_MODE cinema");
    }

    #[tokio::test]
    async fn test_send_key_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.send_key(Keys::VolumeMute).await.unwrap();
        assert_eq!(tracker.get_last_command().unwrap(), "KEY_ACTION volumemute");
    }

    #[tokio::test]
    async fn test_set_screen_mute_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.set_screen_mute(ScreenMuteModes::ScreenMuteOn)
            .await
            .unwrap();
        assert_eq!(
            tracker.get_last_command().unwrap(),
            "SCREEN_MUTE screenmuteon"
        );
    }

    #[tokio::test]
    async fn test_set_energy_saving_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.set_energy_saving(EnergySavingLevels::Maximum)
            .await
            .unwrap();
        assert_eq!(tracker.get_last_command().unwrap(), "ENERGY_SAVING maximum");
    }

    #[tokio::test]
    async fn test_set_input_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.set_input(Inputs::Hdmi1).await.unwrap();
        assert_eq!(tracker.get_last_command().unwrap(), "INPUT_SELECT hdmi1");
    }

    #[tokio::test]
    async fn test_set_volume_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.set_volume(VolumeLevel::try_from(12).unwrap())
            .await
            .unwrap();
        assert_eq!(tracker.get_last_command().unwrap(), "VOLUME_CONTROL 12");
    }

    #[tokio::test]
    async fn test_set_volume_mute_payload() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        let tracker = mock.clone();
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        tv.set_volume_mute(true).await.unwrap();
        assert_eq!(tracker.get_last_command().unwrap().trim(), "VOLUME_MUTE on");

        tv.set_volume_mute(false).await.unwrap();
        assert_eq!(
            tracker.get_last_command().unwrap().trim(),
            "VOLUME_MUTE off"
        );
    }

    #[tokio::test]
    async fn test_send_command_underlying_network_error_propagation() {
        let mock = MockExecutor::new("OK", TEST_KEY);
        mock.inject_error("Broken Pipe");
        let mut tv = setup_connected_tv(mock, TEST_KEY);

        let result = tv.launch_app(Apps::Youtube).await;
        assert!(matches!(result, Err(LgTvError::SendCommand(_))));
    }
}

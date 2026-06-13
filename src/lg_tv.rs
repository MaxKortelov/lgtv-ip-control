use crate::constants::errors::LgTvError;
use crate::constants::types::{
    AppDetails, Apps, ConnectionType, EnergySavingLevels, Inputs, Keys, PictureModes, PowerStates,
    ScreenMuteModes, VolumeLevel,
};
use crate::encryption::Encryption;
use crate::tcp_connection::{Connected, Disconnected, TcpConnection};
use regex::Regex;
use std::collections::HashMap;
use std::io::Error;
use tokio::time::{sleep, Duration};

#[trait_variant::make(CommandExecutor: Send)]
pub trait LocalCommandExecutor: Sized {
    async fn send_command(&mut self, command: Vec<u8>) -> Result<Vec<u8>, Error>;
    async fn wake_on_lan(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn disconnect(self) -> Result<Self, &'static str>;
    async fn reconnect(&mut self) -> Result<(), String>;
}

pub struct LGTV<C, State = Disconnected> {
    executor: C,
    encryption: Encryption,
    _state: std::marker::PhantomData<State>,
}

impl LGTV<TcpConnection<Connected>, Disconnected> {
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

    pub async fn get_ip_control_state(&mut self) -> Result<bool, LgTvError> {
        let result = self.send_command("GET_IPCONTROL_STATE").await?;

        match result.as_str() {
            "ON" => Ok(true),
            _ => Ok(false),
        }
    }

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
                })),
            }
        }

        fn set_response(&self, new_response: &str) {
            if let Ok(mut inner) = self.inner.lock() {
                inner.mock_response = new_response.to_string();
            }
        }
    }

    // =========================================================================
    // 2. Implement CommandExecutor Contract for MockExecutor
    // =========================================================================
    impl CommandExecutor for MockExecutor {
        async fn send_command(&mut self, command: Vec<u8>) -> Result<Vec<u8>, Error> {
            let inner = self.inner.lock().unwrap();

            // 💡 Validate that the command arriving here can be successfully decrypted
            let mock_encryption = Encryption::new(inner.encryption_key.clone());
            let decrypted_command = mock_encryption.decrypt((&command).as_ref()).unwrap();

            println!("Mock Received Decrypted Command: {}", decrypted_command);

            // Encrypt our configured mock response so the LGTV parser can read it properly
            let encrypted_response = mock_encryption.encrypt(&inner.mock_response).unwrap();
            Ok(encrypted_response)
        }

        async fn wake_on_lan(&self) -> Result<(), Box<dyn std::error::Error>> {
            let mut inner = self.inner.lock().unwrap();
            inner.wake_on_lan_called = true;
            Ok(())
        }

        async fn disconnect(self) -> Result<Self, &'static str> {
            let inner_clone = self.inner.clone();

            let mut inner = inner_clone.lock().unwrap();
            inner.disconnect_called = true;

            Ok(self)
        }

        async fn reconnect(&mut self) -> Result<(), String> {
            let mut inner = self.inner.lock().unwrap();
            inner.reconnect_called = true;
            Ok(())
        }
    }

    // =========================================================================
    // 3. Test Cases
    // =========================================================================

    #[tokio::test]
    async fn test_get_current_volume_success() {
        let key = "TESTKEY123";
        // The mock string format expected by your Regex match parser logic
        let mock_executor = MockExecutor::new("VOL:25", key);

        // Setup initial wrapper instance bypass using our crate-internal constructor
        let lgtv_disconnected = LGTV::new(mock_executor, Some(key)).unwrap();
        let mut tv = lgtv_disconnected.transition_to_connected();

        let vol = tv.get_current_volume().await.unwrap();
        assert_eq!(vol, 25);
    }

    #[tokio::test]
    async fn test_get_current_app_success() {
        let key = "TESTKEY123";
        let mock_response =
            "APP:netflix\nHot plug:yes\nSignal:true\nHDCP:2.2\nHDCP Status:authenticated";
        let mock_executor = MockExecutor::new(mock_response, key);

        let lgtv_disconnected = LGTV::new(mock_executor, Some(key)).unwrap();
        let mut tv = lgtv_disconnected.transition_to_connected();

        let app_details = tv.get_current_app().await.unwrap();
        assert_eq!(app_details.app, "netflix");
        assert_eq!(app_details.hdcp_version, "2.2");
    }

    #[tokio::test]
    async fn test_power_on_execution_pipeline() {
        let key = "TESTKEY123";
        // To complete power_on loop safely, test_power_on needs to pull a valid active app response
        let mock_executor = MockExecutor::new("APP:hdmi1", key);
        let tracker = mock_executor.clone();

        let lgtv_disconnected = LGTV::new(mock_executor, Some(key)).unwrap();
        let mut tv = lgtv_disconnected.transition_to_connected();

        tv.power_on(Some(1)).await.unwrap();

        // Check if both low level UDP/TCP transition commands fired successfully
        let inner = tracker.inner.lock().unwrap();
        assert!(inner.wake_on_lan_called);
        assert!(inner.reconnect_called);
    }

    #[tokio::test]
    async fn test_power_off_retry_loop_until_success() {
        let key = "TESTKEY123";
        let mock_executor = MockExecutor::new("APP:youtube", key);
        let tracker = mock_executor.clone();

        let lgtv_disconnected = LGTV::new(mock_executor, Some(key)).unwrap();
        let mut tv = lgtv_disconnected.transition_to_connected();

        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(1500)).await;
            // 💡 CHANGE: Use a valid string "OFF" instead of an empty string "".
            // This ensures the crypto engine has actual text to safely encrypt,
            // preventing the FromUtf8Error while still failing the Regex "APP:" check!
            tracker_clone.set_response("OFF");
        });

        let result = tv.power_off(Some(5)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_disconnect_state_transition() {
        let key = "TESTKEY123";
        let mock_executor = MockExecutor::new("", key);
        let tracker = mock_executor.clone();

        let lgtv_disconnected = LGTV::new(mock_executor, Some(key)).unwrap();
        let tv = lgtv_disconnected.transition_to_connected();

        // This consumes ownership of the Connected instance
        let _disconnected_tv = tv.disconnect().await.unwrap();

        let inner = tracker.inner.lock().unwrap();
        assert!(inner.disconnect_called);
    }
}

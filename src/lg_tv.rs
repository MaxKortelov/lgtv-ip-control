use crate::constants::errors::LgTvError;
use crate::constants::types::{
    AppDetails, Apps, ConnectionType, EnergySavingLevels, Inputs, Keys, PictureModes, PowerStates,
    ScreenMuteModes, VolumeLevel,
};
use crate::encryption::Encryption;
use crate::tcp_connection::{Connected, Disconnected, TcpConnection};
use regex::Regex;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

pub struct LGTV<State = Disconnected> {
    tcp_connection: TcpConnection<State>,
    encryption: Encryption,
}

impl LGTV<Disconnected> {
    pub async fn new(
        tv_ip: &str,
        mac_address: &str,
        key: Option<&str>,
    ) -> Result<LGTV<Connected>, LgTvError> {
        let tcp_connection = match TcpConnection::new(tv_ip, mac_address).await {
            Ok(connection) => connection,
            Err(error) => return Err(LgTvError::TcpConnectionError(error.to_string())),
        };

        let key_code = key.map(|k| k.to_string());

        match key_code {
            Some(key_code) => {
                let encryption = Encryption::new(key_code);
                Ok(LGTV {
                    tcp_connection,
                    encryption,
                })
            }
            None => Err(LgTvError::MissingKeyCode),
        }
    }
}

impl LGTV<Connected> {
    pub async fn disconnect(self) -> Result<LGTV<Disconnected>, LgTvError> {
        let tcp_connection = match self.tcp_connection.disconnect().await {
            Ok(connection) => connection,
            Err(error) => return Err(LgTvError::TcpConnectionError(error.to_string())),
        };

        Ok(LGTV {
            tcp_connection,
            encryption: self.encryption,
        })
    }

    pub async fn power_on(&mut self, retries: Option<u8>) -> Result<(), LgTvError> {
        let attempts_left = retries.unwrap_or(10);

        match self.tcp_connection.wake_on_lan().await {
            Ok(_) => (),
            Err(error) => return Err(LgTvError::WakeOnLan(error.to_string())),
        };

        let tcp_connection =
            match TcpConnection::new(self.tcp_connection.ip(), self.tcp_connection.mac_address())
                .await
            {
                Ok(connection) => connection,
                Err(error) => return Err(LgTvError::TcpConnectionError(error.to_string())),
            };

        self.tcp_connection = tcp_connection;

        match self.test_power_on(attempts_left).await {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
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

        let result = match self.tcp_connection.send_command(encrypted_command).await {
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

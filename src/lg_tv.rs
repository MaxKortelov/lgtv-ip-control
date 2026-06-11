use crate::constants::types::{AppDetails, Apps, ConnectionType, PowerStates};
use crate::encryption::Encryption;
use crate::tcp_connection::{Connected, Disconnected, TcpConnection};
use regex::Regex;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};

pub struct LGTV<State = Disconnected> {
    tcp_connection: TcpConnection<State>,
    encryption: Encryption,
}

impl LGTV<Disconnected> {
    pub async fn new(
        tv_ip: &str,
        mac_address: &str,
        key: Option<&str>,
    ) -> Result<LGTV<Connected>, &'static str> {
        let tcp_connection = TcpConnection::new(tv_ip, mac_address).await?;

        let key_code = key.map(|k| k.to_string());

        match key_code {
            Some(key_code) => {
                let encryption = Encryption::new(key_code);
                Ok(LGTV {
                    tcp_connection,
                    encryption,
                })
            }
            None => Err("Key Code is required"),
        }
    }
}

impl LGTV<Connected> {
    pub async fn disconnect(self) -> Result<LGTV<Disconnected>, &'static str> {
        let tcp_connection = self.tcp_connection.disconnect().await?;

        Ok(LGTV {
            tcp_connection,
            encryption: self.encryption,
        })
    }

    pub async fn power_on(
        &mut self,
        retries: Option<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let attempts_left = retries.unwrap_or(10);
        self.tcp_connection.wake_on_lan().await?;

        let tcp_connection =
            TcpConnection::new(self.tcp_connection.ip(), self.tcp_connection.mac_address()).await?;

        self.tcp_connection = tcp_connection;

        self.test_power_on(attempts_left).await?;

        Ok(())
    }

    pub async fn power_off(
        &mut self,
        retries: Option<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut attempts_left = retries.unwrap_or(10);

        loop {
            if attempts_left == 0 {
                return Err("TV power off failed".into());
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

    pub async fn get_current_app(&mut self) -> Result<AppDetails, Box<dyn std::error::Error>> {
        let result = self.send_command("CURRENT_APP").await?;

        match result.as_str() {
            "" => Err("TV power is off".into()),
            _ => {
                let mut pairs: HashMap<String, String> = HashMap::new();
                let re = Regex::new(r"([\w\s]+):(\S+)")?;

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

    pub async fn get_current_volume(&mut self) -> Result<u8, Box<dyn std::error::Error>> {
        let result = self.send_command("CURRENT_VOL").await?;

        let re = Regex::new(r"^VOL:(\d+)$")?;

        let capture = re.captures(&result);

        match capture {
            Some(capture) => Ok(capture[1].parse::<u8>()?),
            None => Err("Could not parse current volume".into()),
        }
    }

    pub async fn get_ip_control_state(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let result = self.send_command("GET_IPCONTROL_STATE").await?;

        match result.as_str() {
            "ON" => Ok(true),
            _ => Ok(false),
        }
    }

    pub async fn get_mac_address(
        &mut self,
        connection_type: ConnectionType,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let command = format!("GET_MACADDRESS {:?}", connection_type);
        let result = self.send_command(&command).await?;
        Ok(result)
    }

    pub async fn get_mute_state(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let result = self.send_command("MUTE_STATE").await?;
        println!("{}", result);
        let re = Regex::new(r"^MUTE:(on|off)$")?;

        if let Some(captures) = re.captures(&result) {
            match &captures[1] {
                "on" => Ok(true),
                "off" => Ok(false),
                _ => Err("No matches found for mute state".into()),
            }
        } else {
            Err("Could not parse mute state".into())
        }
    }

    pub async fn get_power_state(&mut self) -> Result<PowerStates, Box<dyn std::error::Error>> {
        match self.test_power_on(5).await {
            Ok(_) => Ok(PowerStates::On),
            Err(_) => Ok(PowerStates::Off),
        }
    }

    pub async fn launch_app(&mut self, app: Apps) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("LAUNCH_APP {}", app.as_str());
        self.send_command(&command).await?;
        Ok(())
    }

    async fn send_command(
        &mut self,
        command: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let encrypted_command = self.encryption.encrypt(command)?;

        let result = self.tcp_connection.send_command(encrypted_command).await?;

        let decrypted_result = self.encryption.decrypt(&result)?;

        Ok(decrypted_result)
    }

    async fn test_power_on(&mut self, retries: u8) -> Result<(), Box<dyn std::error::Error>> {
        let mut attempts_left = retries;
        loop {
            match self.get_current_app().await {
                Ok(app_details) if !app_details.app.is_empty() => return Ok(()),
                _ => {
                    if attempts_left == 0 {
                        return Err("Power is off".into());
                    }
                    attempts_left -= 1;
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

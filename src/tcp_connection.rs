use regex::Regex;
use std::io::Error;
use std::marker::PhantomData;
use std::net::IpAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

use crate::constants::default_settings::DefaultSettings;

use crate::lg_tv::CommandExecutor;

pub struct Disconnected;
pub struct Connected;

pub struct TcpConnection<State = Disconnected> {
    state: PhantomData<State>,
    stream: TcpStream,
    settings: DefaultSettings,
    mac_address: String,
    ip: String,
}

impl TcpConnection<Disconnected> {
    pub async fn new(
        tv_ip: &str,
        mac_address: &str,
    ) -> Result<TcpConnection<Connected>, &'static str> {
        let settings = DefaultSettings::default();
        let addr = format!("{tv_ip}:{}", settings.network_port);

        println!("Connecting to TCP server...");

        match TcpStream::connect(addr).await {
            Ok(stream) => {
                println!("TCP connection established");

                Ok(TcpConnection {
                    stream,
                    state: PhantomData,
                    settings,
                    mac_address: mac_address.to_string(),
                    ip: tv_ip.to_string(),
                })
            }
            Err(_) => Err("Error connecting to TCP server"),
        }
    }
}

impl TcpConnection<Connected> {
    pub fn ip(&self) -> &str {
        &self.ip
    }

    pub fn mac_address(&self) -> &str {
        &self.mac_address
    }

    fn create_magic_packet(&self) -> Result<Vec<u8>, &'static str> {
        let re = Regex::new(r"(?i)^([0-9a-f]{2}:){5}([0-9a-f]{2})$").unwrap();

        if !re.is_match(&self.mac_address) {
            return Err("Invalid MAC address");
        }

        let mac_bytes: Vec<u8> = self
            .mac_address
            .split(':')
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect();

        let mut packet = vec![0xff; 6];

        for _ in 0..16 {
            packet.extend_from_slice(&mac_bytes);
        }

        Ok(packet)
    }
}

impl CommandExecutor for TcpConnection<Connected> {
    async fn send_command(&mut self, command: Vec<u8>) -> Result<Vec<u8>, Error> {
        self.stream.write_all(&command).await?;

        let mut buffer = [0u8; 1024];
        // 💡 Capture how many bytes were actually pulled from the wire
        let bytes_read = self.stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            return Err(Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Connection closed by TV",
            ));
        }

        // Return only the slice of the buffer that contains real data
        Ok(buffer[..bytes_read].to_vec())
    }
    async fn wake_on_lan(&self) -> Result<(), Box<dyn std::error::Error>> {
        let ip: IpAddr = self.settings.network_wol_address.parse()?;

        let bind_addr = if ip.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };

        let socket = UdpSocket::bind(bind_addr).await?;

        if ip.is_ipv4() {
            socket.set_broadcast(true)?;
            println!("Using IP v4");
        } else {
            println!("Using IP v6");
        }

        let target = if ip.is_ipv6() {
            format!(
                "[{}]:{}",
                self.settings.network_wol_address, self.settings.network_wol_port
            )
        } else {
            format!(
                "{}:{}",
                self.settings.network_wol_address, self.settings.network_wol_port
            )
        };

        let magic_packet = self.create_magic_packet()?;

        socket.send_to(&magic_packet, &target).await?;

        Ok(())
    }

    async fn disconnect(mut self) -> Result<Self, &'static str> {
        match self.stream.shutdown().await {
            Ok(_) => {
                println!("TCP connection closed");
                Ok(TcpConnection {
                    stream: self.stream,
                    state: PhantomData,
                    settings: self.settings,
                    mac_address: self.mac_address.clone(),
                    ip: self.ip.clone(),
                })
            }
            Err(_) => Err("Error closing TCP connection"),
        }
    }

    async fn reconnect(&mut self) -> Result<(), String> {
        let addr = format!("{}:{}", self.ip, self.settings.network_port);

        // Re-establish the TCP stream socket connection
        match TcpStream::connect(addr).await {
            Ok(new_stream) => {
                self.stream = new_stream;
                Ok(())
            }
            Err(e) => Err(format!("Failed to reconnect TCP socket: {e}")),
        }
    }
}

use regex::Regex;

pub struct DefaultSettings {
    pub encryption_iv_length: usize,
    pub encryption_key_digest: &'static str,
    pub encryption_key_iterations: u32,
    pub encryption_key_length: usize,
    pub encryption_key_salt: [u8; 16],
    pub keycode_format: Regex,
    pub message_block_size: usize,
    pub message_terminator: char,
    pub response_terminator: char,
    pub network_port: u16,
    pub network_timeout: u64,
    pub network_wol_address: &'static str,
    pub network_wol_port: u16,
}

impl Default for DefaultSettings {
    fn default() -> Self {
        Self {
            encryption_iv_length: 16,
            encryption_key_digest: "sha256",
            encryption_key_iterations: 2_u32.pow(14),
            encryption_key_length: 16,
            encryption_key_salt: [
                0x63, 0x61, 0xb8, 0x0e, 0x9b, 0xdc, 0xa6, 0x63, 0x8d, 0x07, 0x20, 0xf2, 0xcc, 0x56,
                0x8f, 0xb9,
            ],
            keycode_format: Regex::new(r"[A-Z0-9]{8}").unwrap(),
            message_block_size: 16,
            message_terminator: '\r',
            response_terminator: '\n',
            network_port: 9761,
            network_timeout: 5000,
            network_wol_address: "255.255.255.255",
            network_wol_port: 9,
        }
    }
}

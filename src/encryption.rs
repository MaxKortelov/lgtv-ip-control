use crate::constants::default_settings::DefaultSettings;
use openssl::hash::MessageDigest;
use openssl::pkcs5;
use openssl::symm::{Cipher, Crypter, Mode};
use rand::RngCore;

pub struct Encryption {
    key_code: String,
    settings: DefaultSettings,
}

impl Encryption {
    pub fn new(key_code: String) -> Encryption {
        let settings = DefaultSettings::default();

        Encryption { key_code, settings }
    }

    pub fn encrypt(&self, message: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let iv = self.generate_random_iv();
        let terminated_message = self.terminate_message(message)?;
        let padded_message = self.pad_message(&terminated_message);

        let iv_enc = self.encrypt_ecb_iv(&iv)?;
        let data_enc = self.encrypt_cbc_message(&iv, (padded_message).as_ref())?;

        let mut result = Vec::with_capacity(iv_enc.len() + data_enc.len());
        result.extend_from_slice(&iv_enc);
        result.extend_from_slice(&data_enc);

        Ok(result)
    }

    pub fn decrypt(&self, cipher_text: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let key_len = self.settings.encryption_key_length; // 16
        let derived_key = self.derive_key()?;

        let iv_enc = &cipher_text[..key_len];
        let data_enc = &cipher_text[key_len..];

        // Decrypt IV using AES-128-ECB
        let iv = self.decrypt_ecb(&derived_key, iv_enc)?;

        // Decrypt data using AES-128-CBC with decrypted IV
        let decrypted = self.decrypt_cbc(&derived_key, &iv, data_enc)?;

        let text = String::from_utf8_lossy(&decrypted).to_string();

        Ok(self.strip_end(&text))
    }

    fn generate_random_iv(&self) -> Vec<u8> {
        let iv_length = self.settings.encryption_iv_length;
        let mut iv = vec![0u8; iv_length];
        rand::rngs::OsRng.fill_bytes(&mut iv);
        iv
    }

    fn terminate_message(&self, message: &str) -> Result<String, &'static str> {
        let terminator = self.settings.message_terminator;
        if message.contains(terminator) {
            return Err("message must not include the message terminator character");
        }

        Ok(format!("{message}{terminator}"))
    }

    fn pad_message(&self, message: &str) -> String {
        let block_size = self.settings.message_block_size;

        let mut new_message = message.to_string();

        if new_message.len().is_multiple_of(block_size) {
            new_message.push(' ');
        }

        let remainder = new_message.len() % block_size;

        if remainder != 0 {
            let padding = block_size - remainder;
            let pad_char = char::from(padding as u8);

            for _ in 0..padding {
                new_message.push(pad_char);
            }
        }

        new_message
    }

    fn derive_key(&self) -> Result<Vec<u8>, openssl::error::ErrorStack> {
        let mut derived_key = vec![0u8; self.settings.encryption_key_length]; // 16

        pkcs5::pbkdf2_hmac(
            self.key_code.as_bytes(),
            &self.settings.encryption_key_salt,
            self.settings.encryption_key_iterations as usize,
            MessageDigest::sha256(),
            &mut derived_key,
        )?;

        Ok(derived_key)
    }

    fn encrypt_ecb_iv(&self, iv: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
        let cipher = Cipher::aes_128_ecb();
        let derived_key = self.derive_key()?;

        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &derived_key, None)?;
        crypter.pad(false);

        let mut output = vec![0u8; iv.len() + cipher.block_size()];
        let count = crypter.update(iv, &mut output)?;
        let rest = crypter.finalize(&mut output[count..])?;

        output.truncate(count + rest);
        Ok(output)
    }

    fn encrypt_cbc_message(
        &self,
        iv: &[u8],
        padded_message: &[u8],
    ) -> Result<Vec<u8>, openssl::error::ErrorStack> {
        let cipher = Cipher::aes_128_cbc();
        let derived_key = self.derive_key()?;

        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &derived_key, Some(iv))?;
        crypter.pad(false);

        let mut output = vec![0u8; padded_message.len() + cipher.block_size()];
        let count = crypter.update(padded_message, &mut output)?;
        let rest = crypter.finalize(&mut output[count..])?;

        output.truncate(count + rest);
        Ok(output)
    }

    fn decrypt_ecb(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
        let cipher = Cipher::aes_128_ecb();

        let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, None)?;
        crypter.pad(false);

        let mut output = vec![0u8; data.len() + cipher.block_size()];
        let count = crypter.update(data, &mut output)?;
        let rest = crypter.finalize(&mut output[count..])?;

        output.truncate(count + rest);
        Ok(output)
    }

    fn decrypt_cbc(
        &self,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
    ) -> Result<Vec<u8>, openssl::error::ErrorStack> {
        let cipher = Cipher::aes_128_cbc();

        let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))?;
        crypter.pad(false);

        let mut output = vec![0u8; data.len() + cipher.block_size()];
        let count = crypter.update(data, &mut output)?;
        let rest = crypter.finalize(&mut output[count..])?;

        output.truncate(count + rest);
        Ok(output)
    }

    fn strip_end(&self, input: &str) -> String {
        input
            .trim_end_matches(|c| c == '\r' || c == '\n' || (c as u8) <= 16)
            .to_string()
    }
}

//! Lightweight obfuscation for AI Insights history entries.
//!
//! Prevents casual plaintext exposure of AI Insights data on disk.
//! This is NOT cryptographic encryption -- it deters casual inspection
//! but will not resist a determined attacker with disk access.

/// Fixed application salt mixed into the obfuscation key.
const APP_SALT: &[u8] = b"telescope-ai-insights-v1";

/// Derive a repeating key stream from the application salt.
/// The key is deterministic and machine-independent so that entries
/// written on one launch can be read on the next.
fn derive_key_stream(len: usize) -> Vec<u8> {
    // Simple key expansion: cycle through salt bytes combined with
    // their position to avoid trivial period in the XOR stream.
    let mut key = Vec::with_capacity(len);
    for i in 0..len {
        let salt_byte = APP_SALT[i % APP_SALT.len()];
        let position_mix = (i / APP_SALT.len()) as u8;
        key.push(salt_byte.wrapping_add(position_mix));
    }
    key
}

/// Obfuscate a plaintext string and return it as a hex-encoded string.
pub fn encrypt_history_entry(plaintext: &str) -> Result<String, String> {
    let data = plaintext.as_bytes();
    let key = derive_key_stream(data.len());
    let ciphertext: Vec<u8> = data.iter().zip(key.iter()).map(|(d, k)| d ^ k).collect();
    Ok(hex_encode(&ciphertext))
}

/// De-obfuscate a hex-encoded string back to plaintext.
pub fn decrypt_history_entry(encoded: &str) -> Result<String, String> {
    let ciphertext = hex_decode(encoded)?;
    let key = derive_key_stream(ciphertext.len());
    let plaintext: Vec<u8> = ciphertext
        .iter()
        .zip(key.iter())
        .map(|(d, k)| d ^ k)
        .collect();
    String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8 after decryption: {e}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Hex string has odd length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at position {i}: {e}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_simple() {
        let original = "hello world";
        let encrypted = encrypt_history_entry(original).unwrap();
        assert_ne!(encrypted, original);
        let decrypted = decrypt_history_entry(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn round_trip_json() {
        let json = r#"[{"prompt":"check pods","response":"All pods are running","timestamp":"2025-01-01T00:00:00Z"}]"#;
        let encrypted = encrypt_history_entry(json).unwrap();
        assert_ne!(encrypted, json);
        let decrypted = decrypt_history_entry(&encrypted).unwrap();
        assert_eq!(decrypted, json);
    }

    #[test]
    fn round_trip_empty() {
        let encrypted = encrypt_history_entry("").unwrap();
        assert_eq!(encrypted, "");
        let decrypted = decrypt_history_entry(&encrypted).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn round_trip_unicode() {
        let text = "Kubernetes pods: healthy -- cluster: aks-prod";
        let encrypted = encrypt_history_entry(text).unwrap();
        let decrypted = decrypt_history_entry(&encrypted).unwrap();
        assert_eq!(decrypted, text);
    }

    #[test]
    fn encrypted_output_is_hex() {
        let encrypted = encrypt_history_entry("test").unwrap();
        assert!(encrypted.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn decrypt_invalid_hex_errors() {
        let result = decrypt_history_entry("zzzz");
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_odd_length_errors() {
        let result = decrypt_history_entry("abc");
        assert!(result.is_err());
    }

    #[test]
    fn deterministic_encryption() {
        let text = "same input";
        let a = encrypt_history_entry(text).unwrap();
        let b = encrypt_history_entry(text).unwrap();
        assert_eq!(a, b);
    }
}

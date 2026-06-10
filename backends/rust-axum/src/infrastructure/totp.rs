use rand::RngCore;
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, TOTP};
use base64::Engine;

pub fn generate_totp_secret() -> String {
    let mut rng = rand::rng();
    let mut secret = [0u8; 20];
    rng.fill_bytes(&mut secret);
    base64::engine::general_purpose::STANDARD.encode(secret).trim_end_matches('=').to_string()
}

pub fn generate_totp_uri(secret: &str, email: &str, issuer: &str) -> String {
    let secret_bytes = base64::engine::general_purpose::STANDARD
        .decode(secret.as_bytes())
        .unwrap_or_else(|_| secret.as_bytes().to_vec());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some(issuer.to_string()),
        email.to_string(),
    )
    .expect("Failed to create TOTP");

    totp.get_url()
}

pub fn verify_totp_code(secret: &str, code: &str) -> bool {
    let secret_bytes = base64::engine::general_purpose::STANDARD
        .decode(secret.as_bytes())
        .unwrap_or_else(|_| secret.as_bytes().to_vec());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        String::new(),
    )
    .expect("Failed to create TOTP");

    totp.check_current(code).unwrap_or(false)
}

pub fn generate_backup_codes(count: usize) -> Vec<String> {
    let mut rng = rand::rng();
    (0..count)
        .map(|_| {
            let mut bytes = [0u8; 4];
            rng.fill_bytes(&mut bytes);
            hex::encode(bytes)
        })
        .collect()
}

pub fn hash_backup_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_backup_code(code: &str, hashed_codes: &[String]) -> bool {
    let hashed = hash_backup_code(code);
    hashed_codes.contains(&hashed)
}
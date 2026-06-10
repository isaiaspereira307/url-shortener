use crate::infrastructure::totp;

pub struct TotpSetupResult {
    pub secret: String,
    pub qr_code_uri: String,
    pub backup_codes: Vec<String>,
    pub hashed_backup_codes: Vec<String>,
}

pub fn setup_totp(email: &str) -> TotpSetupResult {
    let secret = totp::generate_totp_secret();
    let qr_code_uri = totp::generate_totp_uri(&secret, email, "URL Shortener");
    let backup_codes = totp::generate_backup_codes(8);
    let hashed_backup_codes: Vec<String> = backup_codes
        .iter()
        .map(|c| totp::hash_backup_code(c))
        .collect();

    TotpSetupResult {
        secret,
        qr_code_uri,
        backup_codes,
        hashed_backup_codes,
    }
}

pub fn verify_totp(secret: &str, code: &str) -> bool {
    totp::verify_totp_code(secret, code)
}

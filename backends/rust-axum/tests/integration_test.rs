#[cfg(test)]
mod tests {
    use url_shortener_rust::application::auth_service;
    use url_shortener_rust::application::link_service;
    use url_shortener_rust::infrastructure::{auth, password, totp};

    #[test]
    fn test_validate_email_valid() {
        assert!(auth_service::validate_email("user@example.com"));
        assert!(auth_service::validate_email("user+tag@example.com"));
        assert!(auth_service::validate_email("user@mail.example.com"));
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(!auth_service::validate_email("invalid-email"));
        assert!(!auth_service::validate_email(""));
        assert!(!auth_service::validate_email("user@"));
        assert!(!auth_service::validate_email("@example.com"));
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(auth_service::validate_password("securepass123").is_ok());
        assert!(auth_service::validate_password("abcdefgh").is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        assert!(auth_service::validate_password("short").is_err());
        assert!(auth_service::validate_password("abcdefg").is_err());
    }

    #[test]
    fn test_generate_slug() {
        assert_eq!(auth_service::generate_slug("MyCompany"), "mycompany");
        assert_eq!(auth_service::generate_slug("My Company"), "my-company");
        assert_eq!(auth_service::generate_slug("ACME Corp"), "acme-corp");
        assert_eq!(auth_service::generate_slug("-test-"), "test");
    }

    #[test]
    fn test_password_hash_and_verify() {
        let password = "securepassword123";
        let hash = password::hash_password(password);
        assert!(password::verify_password(password, &hash));
        assert!(!password::verify_password("wrongpassword", &hash));
    }

    #[test]
    fn test_password_hash_is_different_each_time() {
        let hash1 = password::hash_password("samepassword");
        let hash2 = password::hash_password("samepassword");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_create_and_verify_access_token() {
        let settings = url_shortener_rust::infrastructure::config::Settings::default();
        let user_id = uuid::Uuid::new_v4();
        let tenant_id = uuid::Uuid::new_v4();

        let token = auth::create_access_token(&settings.jwt, user_id, tenant_id, "user@example.com");
        let claims = auth::verify_token(&token, &settings.jwt).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.tenant_id, tenant_id.to_string());
        assert_eq!(claims.email, "user@example.com");
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_create_and_verify_refresh_token() {
        let settings = url_shortener_rust::infrastructure::config::Settings::default();
        let user_id = uuid::Uuid::new_v4();
        let tenant_id = uuid::Uuid::new_v4();

        let token = auth::create_refresh_token(&settings.jwt, user_id, tenant_id);
        let claims = auth::verify_token(&token, &settings.jwt).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.tenant_id, tenant_id.to_string());
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn test_verify_invalid_token() {
        let settings = url_shortener_rust::infrastructure::config::Settings::default();
        let result = auth::verify_token("invalid.token.here", &settings.jwt);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_expiry() {
        let settings = url_shortener_rust::infrastructure::config::Settings::default();
        let user_id = uuid::Uuid::new_v4();
        let tenant_id = uuid::Uuid::new_v4();

        let token = auth::create_access_token(&settings.jwt, user_id, tenant_id, "user@example.com");
        let claims = auth::verify_token(&token, &settings.jwt).unwrap();

        assert!(claims.exp > 0);
        assert!(claims.iat > 0);
    }

    #[test]
    fn test_generate_totp_secret() {
        let secret = totp::generate_totp_secret();
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_generate_totp_uri() {
        let secret = totp::generate_totp_secret();
        let uri = totp::generate_totp_uri(&secret, "user@example.com", "URL Shortener");
        assert!(uri.contains("otpauth://totp"));
        assert!(uri.contains("user%40example.com"));
    }

    #[test]
    fn test_verify_totp_valid_code() {
        let secret = totp::generate_totp_secret();
        let totp = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            totp_rs::Secret::Raw(secret.as_bytes().to_vec()),
            None,
            String::new(),
        )
        .unwrap();
        let code = totp.generate_current().unwrap();
        assert!(totp::verify_totp_code(&secret, &code));
    }

    #[test]
    fn test_verify_totp_invalid_code() {
        let secret = totp::generate_totp_secret();
        assert!(!totp::verify_totp_code(&secret, "000000"));
    }

    #[test]
    fn test_generate_backup_codes() {
        let codes = totp::generate_backup_codes(8);
        assert_eq!(codes.len(), 8);
        for code in &codes {
            assert_eq!(code.len(), 8);
        }
    }

    #[test]
    fn test_hash_and_verify_backup_code() {
        let code = "abcd1234";
        let hashed = totp::hash_backup_code(code);
        assert_ne!(hashed, code);
        assert!(totp::verify_backup_code(code, &[hashed]));
        assert!(!totp::verify_backup_code("wrongcode", &[hashed]));
    }

    #[test]
    fn test_generate_short_code() {
        let code = link_service::generate_short_code(7);
        assert_eq!(code.len(), 7);
        assert!(code.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_short_code_unique() {
        let codes: std::collections::HashSet<_> = (0..100)
            .map(|_| link_service::generate_short_code(7))
            .collect();
        assert_eq!(codes.len(), 100);
    }

    #[test]
    fn test_generate_short_code_no_special_chars() {
        let code = link_service::generate_short_code(7);
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_short_code_different_lengths() {
        for length in [5, 7, 10, 15] {
            let code = link_service::generate_short_code(length);
            assert_eq!(code.len(), length, "Failed for length {}", length);
        }
    }

    #[test]
    fn test_url_validation_in_shorten_request() {
        use url_shortener_rust::presentation::types::ShortenRequest;
        use validator::Validate;

        let valid_req = ShortenRequest { url: "https://example.com".to_string() };
        assert!(valid_req.validate().is_ok());

        let invalid_req = ShortenRequest { url: "".to_string() };
        assert!(invalid_req.validate().is_err());
    }
}

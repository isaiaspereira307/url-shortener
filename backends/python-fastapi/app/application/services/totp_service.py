from app.infrastructure.auth.totp import (
    generate_totp_secret,
    generate_totp_uri,
    verify_totp_code,
    generate_backup_codes,
    hash_backup_code,
)


def setup_totp(email: str) -> dict:
    secret = generate_totp_secret()
    uri = generate_totp_uri(secret, email)
    backup_codes = generate_backup_codes()
    hashed_codes = [hash_backup_code(c) for c in backup_codes]
    return {
        "secret": secret,
        "qr_code_uri": uri,
        "backup_codes": backup_codes,
        "hashed_backup_codes": hashed_codes,
    }


def verify_totp(secret: str, code: str) -> bool:
    return verify_totp_code(secret, code)

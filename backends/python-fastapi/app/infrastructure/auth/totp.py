import pyotp
import secrets
import hashlib


def generate_totp_secret() -> str:
    return pyotp.random_base32()


def generate_totp_uri(secret: str, email: str, issuer: str = "URL Shortener") -> str:
    return pyotp.totp.TOTP(secret).provisioning_uri(name=email, issuer_name=issuer)


def verify_totp_code(secret: str, code: str, valid_window: int = 1) -> bool:
    totp = pyotp.TOTP(secret)
    return totp.verify(code, valid_window=valid_window)


def generate_backup_codes(count: int = 8) -> list[str]:
    codes = []
    for _ in range(count):
        code = secrets.token_hex(4)
        codes.append(code)
    return codes


def hash_backup_code(code: str) -> str:
    return hashlib.sha256(code.encode()).hexdigest()


def verify_backup_code(code: str, hashed_codes: list[str]) -> bool:
    hashed = hash_backup_code(code)
    return hashed in hashed_codes

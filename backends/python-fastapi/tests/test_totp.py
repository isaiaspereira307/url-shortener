import pytest
from app.infrastructure.auth.totp import (
    generate_totp_secret,
    generate_totp_uri,
    verify_totp_code,
    generate_backup_codes,
    hash_backup_code,
    verify_backup_code,
)
import pyotp


class TestTOTP:
    def test_generate_secret(self):
        secret = generate_totp_secret()
        assert len(secret) > 0
        assert isinstance(secret, str)

    def test_generate_uri(self):
        secret = generate_totp_secret()
        uri = generate_totp_uri(secret, "user@example.com")
        assert "otpauth://totp" in uri
        assert "user%40example.com" in uri or "user@example.com" in uri

    def test_verify_valid_code(self):
        secret = generate_totp_secret()
        totp = pyotp.TOTP(secret)
        code = totp.now()
        assert verify_totp_code(secret, code) is True

    def test_verify_invalid_code(self):
        secret = generate_totp_secret()
        assert verify_totp_code(secret, "000000") is False

    def test_generate_backup_codes(self):
        codes = generate_backup_codes()
        assert len(codes) == 8
        for code in codes:
            assert len(code) == 8

    def test_hash_and_verify_backup_code(self):
        code = "abcd1234"
        hashed = hash_backup_code(code)
        assert hashed != code
        assert verify_backup_code(code, [hashed]) is True

    def test_verify_wrong_backup_code(self):
        hashed = hash_backup_code("abcd1234")
        assert verify_backup_code("wrongcode", [hashed]) is False

import pytest
from app.infrastructure.auth.password import hash_password, verify_password


class TestPasswordHashing:
    def test_hash_and_verify(self):
        password = "securepassword123"
        hashed = hash_password(password)
        assert hashed != password
        assert verify_password(password, hashed) is True

    def test_wrong_password(self):
        hashed = hash_password("correctpassword")
        assert verify_password("wrongpassword", hashed) is False

    def test_hash_is_different_each_time(self):
        password = "samepassword"
        hash1 = hash_password(password)
        hash2 = hash_password(password)
        assert hash1 != hash2

    def test_empty_password(self):
        hashed = hash_password("")
        assert verify_password("", hashed) is True
        assert verify_password("notempty", hashed) is False

import pytest
from app.application.services.auth_service import validate_email, validate_password, generate_slug


class TestValidateEmail:
    def test_valid_email(self):
        assert validate_email("user@example.com") is True

    def test_valid_email_with_subdomain(self):
        assert validate_email("user@mail.example.com") is True

    def test_valid_email_with_plus(self):
        assert validate_email("user+tag@example.com") is True

    def test_invalid_email_no_at(self):
        assert validate_email("userexample.com") is False

    def test_invalid_email_no_domain(self):
        assert validate_email("user@") is False

    def test_invalid_email_empty(self):
        assert validate_email("") is False


class TestValidatePassword:
    def test_valid_password(self):
        valid, msg = validate_password("securepass123")
        assert valid is True

    def test_invalid_password_too_short(self):
        valid, msg = validate_password("short")
        assert valid is False
        assert "8 characters" in msg

    def test_invalid_password_exactly_7(self):
        valid, msg = validate_password("abcdefg")
        assert valid is False

    def test_valid_password_exactly_8(self):
        valid, msg = validate_password("abcdefgh")
        assert valid is True


class TestGenerateSlug:
    def test_simple_name(self):
        assert generate_slug("MyCompany") == "mycompany"

    def test_spaces_to_hyphens(self):
        assert generate_slug("My Company") == "my-company"

    def test_special_chars(self):
        assert generate_slug("My Company! @#") == "my-company"

    def test_lowercase(self):
        assert generate_slug("ACME Corp") == "acme-corp"

    def test_strips_hyphens(self):
        assert generate_slug("-test-") == "test"

    def test_multiple_spaces(self):
        assert generate_slug("My  Company") == "my-company"

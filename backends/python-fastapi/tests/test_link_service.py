import pytest
from app.application.services.link_service import generate_short_code


class TestShortCode:
    def test_generates_correct_length(self):
        code = generate_short_code()
        assert len(code) == 7

    def test_generates_alphanumeric(self):
        code = generate_short_code()
        assert code.isalnum()

    def test_generates_unique_codes(self):
        codes = {generate_short_code() for _ in range(100)}
        assert len(codes) == 100

    def test_no_special_chars(self):
        code = generate_short_code()
        import re
        assert re.match(r"^[a-zA-Z0-9]+$", code)

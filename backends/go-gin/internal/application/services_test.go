package application

import (
	"testing"
)

func TestValidateEmail(t *testing.T) {
	tests := []struct {
		email string
		valid bool
	}{
		{"user@example.com", true},
		{"user+tag@example.com", true},
		{"user@mail.example.com", true},
		{"invalid-email", false},
		{"", false},
		{"user@", false},
		{"@example.com", false},
	}

	for _, tt := range tests {
		result := ValidateEmail(tt.email)
		if result != tt.valid {
			t.Errorf("ValidateEmail(%q) = %v, want %v", tt.email, result, tt.valid)
		}
	}
}

func TestValidatePassword(t *testing.T) {
	if err := ValidatePassword("securepass123"); err != nil {
		t.Errorf("expected no error for valid password, got %v", err)
	}

	if err := ValidatePassword("abcdefgh"); err != nil {
		t.Errorf("expected no error for 8-char password, got %v", err)
	}

	if err := ValidatePassword("short"); err == nil {
		t.Error("expected error for short password")
	}

	if err := ValidatePassword("abcdefg"); err == nil {
		t.Error("expected error for 7-char password")
	}
}

func TestGenerateSlug(t *testing.T) {
	tests := []struct {
		name     string
		expected string
	}{
		{"MyCompany", "mycompany"},
		{"My Company", "my-company"},
		{"ACME Corp", "acme-corp"},
		{"-test-", "test"},
		{"  Hello  World  ", "hello-world"},
	}

	for _, tt := range tests {
		result := GenerateSlug(tt.name)
		if result != tt.expected {
			t.Errorf("GenerateSlug(%q) = %q, want %q", tt.name, result, tt.expected)
		}
	}
}

func TestGenerateShortCode(t *testing.T) {
	code := GenerateShortCode(7)
	if len(code) != 7 {
		t.Errorf("expected length 7, got %d", len(code))
	}

	for _, c := range code {
		if !((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')) {
			t.Errorf("invalid character %c in short code", c)
		}
	}
}

func TestGenerateShortCodeUniqueness(t *testing.T) {
	codes := make(map[string]bool)
	for i := 0; i < 100; i++ {
		code := GenerateShortCode(7)
		if codes[code] {
			t.Errorf("duplicate short code generated: %s", code)
		}
		codes[code] = true
	}
}

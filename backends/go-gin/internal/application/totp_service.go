package application

import "url-shortener-go/internal/infrastructure"

type TotpSetupResult struct {
	Secret            string
	QRCodeURI         string
	BackupCodes       []string
	HashedBackupCodes []string
}

func SetupTOTP(email string) *TotpSetupResult {
	secret := infrastructure.GenerateTOTPSecret()
	uri := infrastructure.GenerateTOTPURI(secret, email, "URL Shortener")
	codes := infrastructure.GenerateBackupCodes(8)
	hashed := make([]string, len(codes))
	for i, c := range codes {
		hashed[i] = infrastructure.HashBackupCode(c)
	}

	return &TotpSetupResult{
		Secret:            secret,
		QRCodeURI:         uri,
		BackupCodes:       codes,
		HashedBackupCodes: hashed,
	}
}

func VerifyTOTP(secret, code string) bool {
	return infrastructure.VerifyTOTPCode(secret, code)
}

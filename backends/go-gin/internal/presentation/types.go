package presentation

import "time"

type RegisterRequest struct {
	Email      string `json:"email" binding:"required,email"`
	Password   string `json:"password" binding:"required,min=8"`
	TenantName string `json:"tenant_name" binding:"required"`
}

type LoginRequest struct {
	Email    string `json:"email" binding:"required"`
	Password string `json:"password" binding:"required"`
}

type RefreshRequest struct {
	RefreshToken string `json:"refresh_token" binding:"required"`
}

type TotpVerifyRequest struct {
	Code string `json:"code" binding:"required"`
}

type ShortenRequest struct {
	URL string `json:"url" binding:"required,url"`
}

type TokenResponse struct {
	AccessToken  string `json:"access_token"`
	RefreshToken string `json:"refresh_token"`
	TokenType    string `json:"token_type"`
	TOTPRequired bool   `json:"totp_required"`
}

type TotpSetupResponse struct {
	Secret      string   `json:"secret"`
	QRCodeURI   string   `json:"qr_code_uri"`
	BackupCodes []string `json:"backup_codes"`
}

type LinkResponse struct {
	ID          string    `json:"id"`
	ShortURL    string    `json:"short_url"`
	OriginalURL string    `json:"original_url"`
	ShortCode   string    `json:"short_code"`
	Clicks      int64     `json:"clicks"`
	CreatedAt   time.Time `json:"created_at"`
}

type LinkListResponse struct {
	Links []LinkResponse `json:"links"`
	Total int64          `json:"total"`
	Page  int            `json:"page"`
	Limit int            `json:"limit"`
}

type LinkStatsResponse struct {
	ShortCode       string                 `json:"short_code"`
	OriginalURL      string                 `json:"original_url"`
	TotalClicks      int64                  `json:"total_clicks"`
	UniqueVisitors   int64                  `json:"unique_visitors"`
	ClicksByCountry map[string]int64       `json:"clicks_by_country"`
	ClicksByDay      []interface{}          `json:"clicks_by_day"`
	RecentClicks     []map[string]interface{} `json:"recent_clicks"`
	Browsers         map[string]int64       `json:"browsers"`
	Platforms        map[string]int64       `json:"platforms"`
}

type HealthResponse struct {
	Status    string    `json:"status"`
	Service   string    `json:"service"`
	Database  string    `json:"database"`
	Redis     string    `json:"redis"`
	Timestamp time.Time `json:"timestamp"`
}

type PaginationParams struct {
	Page  int    `form:"page,default=1"`
	Limit int    `form:"limit,default=20"`
	Sort  string `form:"sort,default=created_at"`
	Order string `form:"order,default=desc"`
}

type MyIPResponse struct {
	IP        string  `json:"ip"`
	Country   *string `json:"country"`
	City      *string `json:"city"`
	Latitude  *float64 `json:"latitude"`
	Longitude *float64 `json:"longitude"`
	ISP       *string `json:"isp"`
}

type URLCheckRequest struct {
	URL string `json:"url" binding:"required"`
}

type RedirectStep struct {
	URL    string `json:"url"`
	Status *int   `json:"status"`
}

type URLCheckResponse struct {
	OriginalURL    string            `json:"original_url"`
	FinalURL        *string           `json:"final_url"`
	RedirectChain  []RedirectStep    `json:"redirect_chain"`
	TotalRedirects int               `json:"total_redirects"`
	IsSafe         bool              `json:"is_safe"`
	Warnings       []string          `json:"warnings"`
	ServerIP       *string           `json:"server_ip"`
	ServerHeaders  map[string]string `json:"server_headers"`
}

type PixelCreateRequest struct {
	Name *string `json:"name"`
}

type PixelResponse struct {
	ID        string    `json:"id"`
	Code      string    `json:"code"`
	Name      *string   `json:"name"`
	PixelURL  string    `json:"pixel_url"`
	Clicks    int64     `json:"clicks"`
	CreatedAt time.Time `json:"created_at"`
}

type PixelListResponse struct {
	Pixels []PixelResponse `json:"pixels"`
	Total  int64            `json:"total"`
	Page   int              `json:"page"`
	Limit  int              `json:"limit"`
}

type UTMBuildRequest struct {
	URL         string  `json:"url" binding:"required"`
	UTMSource   *string `json:"utm_source"`
	UTMMedium   *string `json:"utm_medium"`
	UTMCampaign *string `json:"utm_campaign"`
	UTMTerm     *string `json:"utm_term"`
	UTMContent  *string `json:"utm_content"`
}

type UTMResponse struct {
	OriginalURL string            `json:"original_url"`
	UTMURL     string            `json:"utm_url"`
	Params     map[string]string `json:"params"`
}
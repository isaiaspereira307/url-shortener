package domain

type AppError struct {
	Code    int
	Message string
}

func (e *AppError) Error() string {
	return e.Message
}

func NewValidationError(msg string) *AppError {
	return &AppError{Code: 400, Message: msg}
}

func NewNotFoundError(msg string) *AppError {
	return &AppError{Code: 404, Message: msg}
}

func NewUnauthorizedError(msg string) *AppError {
	return &AppError{Code: 401, Message: msg}
}

func NewConflictError(msg string) *AppError {
	return &AppError{Code: 409, Message: msg}
}

func NewInternalError() *AppError {
	return &AppError{Code: 500, Message: "Internal error"}
}

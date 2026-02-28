use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Matrix error: {0}")]
    Matrix(#[source] MatrixError),

    #[error("WeChat error: {0}")]
    WeChat(#[source] WeChatError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Rate limited, retry after {0} seconds")]
    RateLimited(u64),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Message send failed after {0} retries")]
    SendFailed(u32),

    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Portal not found: {0}")]
    PortalNotFound(String),

    #[error("Encryption error: {0}")]
    Encryption(String),
}

#[derive(Debug, Error)]
pub enum MatrixError {
    #[error("API error: {code} - {message}")]
    Api { code: String, message: String },

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Media upload failed: {0}")]
    MediaUpload(String),

    #[error("Media download failed: {0}")]
    MediaDownload(String),

    #[error("Room creation failed: {0}")]
    RoomCreation(String),

    #[error("Event send failed: {0}")]
    EventSend(String),

    #[error("Encryption not supported")]
    EncryptionNotSupported,

    #[error("Device not verified")]
    DeviceNotVerified,

    #[error("Session not found")]
    SessionNotFound,
}

#[derive(Debug, Error)]
pub enum WeChatError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Login required")]
    LoginRequired,

    #[error("QR code expired")]
    QrCodeExpired,

    #[error("Scan timeout")]
    ScanTimeout,

    #[error("User blocked")]
    UserBlocked,

    #[error("Message send failed: {0}")]
    SendFailed(String),

    #[error("Media download failed: {0}")]
    MediaDownload(String),

    #[error("Contact not found: {0}")]
    ContactNotFound(String),

    #[error("Group not found: {0}")]
    GroupNotFound(String),

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Invalid message type: {0}")]
    InvalidMessageType(String),

    #[error("File too large: {0} bytes")]
    FileTooLarge(u64),
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Store error: {0}")]
    StoreError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

impl From<MatrixError> for BridgeError {
    fn from(e: MatrixError) -> Self {
        BridgeError::Matrix(e)
    }
}

impl From<WeChatError> for BridgeError {
    fn from(e: WeChatError) -> Self {
        BridgeError::WeChat(e)
    }
}

impl From<CryptoError> for BridgeError {
    fn from(e: CryptoError) -> Self {
        BridgeError::Crypto(e.to_string())
    }
}

impl From<reqwest::Error> for BridgeError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            BridgeError::Timeout(e.to_string())
        } else if e.is_connect() {
            BridgeError::Network(format!("Connection failed: {}", e))
        } else {
            BridgeError::Http(e.to_string())
        }
    }
}

impl From<diesel::result::Error> for BridgeError {
    fn from(e: diesel::result::Error) -> Self {
        BridgeError::Database(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, BridgeError>;
pub type MatrixResult<T> = std::result::Result<T, MatrixError>;
pub type WeChatResult<T> = std::result::Result<T, WeChatError>;
pub type CryptoResult<T> = std::result::Result<T, CryptoError>;

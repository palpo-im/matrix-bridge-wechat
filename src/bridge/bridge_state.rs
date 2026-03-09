use std::fmt;

/// Error codes for WeChat bridge state, mirroring the Go reference's status.BridgeStateErrorCode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeStateError {
    WechatLoggedOut,
    WechatNotConnected,
    WechatConnecting,
    WechatConnectionFailed,
}

impl BridgeStateError {
    /// Returns the error code string used in the bridge protocol.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::WechatLoggedOut => "wechat-logged-out",
            Self::WechatNotConnected => "wechat-not-connected",
            Self::WechatConnecting => "wechat-connecting",
            Self::WechatConnectionFailed => "wechat-connection-failed",
        }
    }

    /// Returns a human-readable error message for this state.
    pub fn human_message(&self) -> &'static str {
        match self {
            Self::WechatLoggedOut => {
                "You were logged out from another device. Relogin to continue using the bridge."
            }
            Self::WechatNotConnected => "You're not connected to WeChat.",
            Self::WechatConnecting => "Reconnecting to WeChat...",
            Self::WechatConnectionFailed => "Connect to the WeChat servers failed.",
        }
    }
}

impl fmt::Display for BridgeStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_code(), self.human_message())
    }
}

/// Represents the current state of the WeChat bridge connection.
#[derive(Debug, Clone)]
pub struct BridgeState {
    /// The current error state, if any.
    pub error: Option<BridgeStateError>,
    /// An optional additional message providing context about the current state.
    pub message: Option<String>,
}

impl BridgeState {
    /// Creates a new BridgeState with no error (healthy state).
    pub fn ok() -> Self {
        Self {
            error: None,
            message: None,
        }
    }

    /// Creates a new BridgeState with the given error.
    pub fn with_error(error: BridgeStateError) -> Self {
        Self {
            error: Some(error),
            message: None,
        }
    }

    /// Creates a new BridgeState with the given error and additional message.
    pub fn with_error_and_message(error: BridgeStateError, message: impl Into<String>) -> Self {
        Self {
            error: Some(error),
            message: Some(message.into()),
        }
    }

    /// Returns true if the bridge is in a healthy (no error) state.
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }

    /// Returns the error code string, or None if the bridge is healthy.
    pub fn error_code(&self) -> Option<&'static str> {
        self.error.map(|e| e.error_code())
    }
}

impl fmt::Display for BridgeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.error {
            Some(error) => {
                write!(f, "{}", error)?;
                if let Some(msg) = &self.message {
                    write!(f, " ({})", msg)?;
                }
                Ok(())
            }
            None => write!(f, "OK"),
        }
    }
}

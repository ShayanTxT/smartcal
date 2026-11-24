use thiserror::Error;

/// Exact required user-facing messages from specification.
pub const MSG_TOUCH_INACCURATE: &str =
    "Touch point was not detected accurately. Please touch the target again.";
pub const MSG_QUALITY_INSUFFICIENT: &str = "Calibration quality is insufficient. Recalibrate?";
pub const MSG_PERMISSIONS_REQUIRED: &str =
    "Additional system permissions are required to apply this calibration.";

#[derive(Debug, Error)]
pub enum SmartCalError {
    #[error("{MSG_PERMISSIONS_REQUIRED} Details: {0}")]
    InsufficientPermissions(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Display not found: {0}")]
    DisplayNotFound(String),

    #[error("Device disconnected during operation: {0}")]
    DeviceDisconnected(String),

    #[error("{MSG_TOUCH_INACCURATE}")]
    TouchDetectionInaccurate,

    #[error("Invalid calibration geometry or degenerate points: {0}")]
    DegenerateGeometry(String),

    #[error("Profile error: {0}")]
    Profile(String),

    #[error("Backend operation failed ({backend}): {message}")]
    Backend { backend: String, message: String },

    #[error("Unsupported display server or feature: {0}")]
    Unsupported(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl SmartCalError {
    /// Returns a safe, concise user-facing error message without sensitive internals.
    pub fn user_message(&self) -> String {
        match self {
            SmartCalError::InsufficientPermissions(_) => MSG_PERMISSIONS_REQUIRED.to_string(),
            SmartCalError::TouchDetectionInaccurate => MSG_TOUCH_INACCURATE.to_string(),
            other => other.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, SmartCalError>;

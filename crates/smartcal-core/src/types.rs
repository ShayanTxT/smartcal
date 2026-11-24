use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    English,
    Persian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warning,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DisplayServer {
    X11,
    Wayland,
    #[default]
    Unknown,
}

impl fmt::Display for DisplayServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayServer::X11 => write!(f, "X11"),
            DisplayServer::Wayland => write!(f, "Wayland"),
            DisplayServer::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WaylandCompositor {
    Mutter,
    KWin,
    Sway,
    Hyprland,
    Wlroots,
    Weston,
    #[default]
    NoneOrUnknown,
}

impl fmt::Display for WaylandCompositor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaylandCompositor::Mutter => write!(f, "GNOME / Mutter"),
            WaylandCompositor::KWin => write!(f, "KDE / KWin"),
            WaylandCompositor::Sway => write!(f, "Sway"),
            WaylandCompositor::Hyprland => write!(f, "Hyprland"),
            WaylandCompositor::Wlroots => write!(f, "wlroots"),
            WaylandCompositor::Weston => write!(f, "Weston"),
            WaylandCompositor::NoneOrUnknown => write!(f, "Unknown / N/A"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionInfo {
    pub os_name: String,
    pub kernel_version: String,
    pub desktop_environment: String,
    pub session_type: String,
    pub display_server: DisplayServer,
    pub compositor: WaylandCompositor,
    pub display_env: Option<String>,
    pub wayland_display_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub friendly_label: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub refresh_hz: f64,
    pub scale_factor: f64,
    pub primary: bool,
    pub connected: bool,
    pub physical_width_mm: Option<u32>,
    pub physical_height_mm: Option<u32>,
}

impl DisplayInfo {
    pub fn resolution_label(&self) -> String {
        format!("{} × {}", self.width, self.height)
    }

    pub fn is_projector_candidate(&self) -> bool {
        let lower = format!("{} {}", self.name, self.friendly_label).to_lowercase();
        !self.primary
            || lower.contains("hdmi")
            || lower.contains("dp")
            || lower.contains("vga")
            || lower.contains("projector")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    #[default]
    Touchscreen,
    PenStylus,
    Mouse,
    Touchpad,
    Other,
}

impl fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceKind::Touchscreen => write!(f, "Touchscreen"),
            DeviceKind::PenStylus => write!(f, "Pen / Stylus"),
            DeviceKind::Mouse => write!(f, "Mouse"),
            DeviceKind::Touchpad => write!(f, "Touchpad"),
            DeviceKind::Other => write!(f, "Other"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TouchCapability {
    SingleTouch,
    MultiTouch {
        max_slots: u32,
    },
    #[default]
    PointerOnly,
}

impl fmt::Display for TouchCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TouchCapability::SingleTouch => write!(f, "Single Touch"),
            TouchCapability::MultiTouch { max_slots } => {
                if *max_slots > 0 {
                    write!(f, "Multi Touch ({} pts)", max_slots)
                } else {
                    write!(f, "Multi Touch")
                }
            }
            TouchCapability::PointerOnly => write!(f, "Pointer / Relative"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    #[default]
    Connected,
    Disconnected,
    PermissionDenied,
    Busy,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::PermissionDenied => write!(f, "Permission Denied"),
            ConnectionStatus::Busy => write!(f, "Busy"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisRange {
    pub min: i32,
    pub max: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

impl Default for AxisRange {
    fn default() -> Self {
        Self {
            min: 0,
            max: 32767,
            fuzz: 0,
            flat: 0,
            resolution: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AxisBounds {
    pub x: AxisRange,
    pub y: AxisRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDeviceInfo {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub event_path: String,
    pub vendor_id: String,
    pub product_id: String,
    pub bus_type: String,
    pub phys: Option<String>,
    pub uniq: Option<String>,
    pub touch_capability: TouchCapability,
    pub status: ConnectionStatus,
    pub readable: bool,
    pub writable: bool,
    pub xinput_id: Option<u32>,
    pub libinput_name: Option<String>,
    pub abs_bounds: Option<AxisBounds>,
    pub current_x11_matrix: Option<[f64; 9]>,
}

impl InputDeviceInfo {
    pub fn vid_pid_label(&self) -> String {
        format!("{}:{}", self.vendor_id, self.product_id)
    }

    pub fn is_interactive_touch(&self) -> bool {
        matches!(self.kind, DeviceKind::Touchscreen | DeviceKind::PenStylus)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationType {
    Points4,
    #[default]
    Points9,
    Points16,
    Points25,
    Advanced,
}

impl CalibrationType {
    pub fn point_count(&self) -> usize {
        match self {
            CalibrationType::Points4 => 4,
            CalibrationType::Points9 => 9,
            CalibrationType::Points16 => 16,
            CalibrationType::Points25 => 25,
            CalibrationType::Advanced => 25,
        }
    }

    pub fn grid_dims(&self) -> (usize, usize) {
        match self {
            CalibrationType::Points4 => (2, 2),
            CalibrationType::Points9 => (3, 3),
            CalibrationType::Points16 => (4, 4),
            CalibrationType::Points25 => (5, 5),
            CalibrationType::Advanced => (5, 5),
        }
    }

    pub fn recommended_method(&self) -> CalibrationMethod {
        match self {
            CalibrationType::Points4 => CalibrationMethod::Homography,
            CalibrationType::Points9 | CalibrationType::Points16 | CalibrationType::Points25 => {
                CalibrationMethod::HomographyRansac
            }
            CalibrationType::Advanced => CalibrationMethod::HybridTps,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationMethod {
    Homography,
    #[default]
    HomographyRansac,
    Affine,
    PolynomialDeg2,
    PolynomialDeg3,
    HybridTps,
}

impl fmt::Display for CalibrationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalibrationMethod::Homography => write!(f, "homography"),
            CalibrationMethod::HomographyRansac => write!(f, "homography_ransac"),
            CalibrationMethod::Affine => write!(f, "affine"),
            CalibrationMethod::PolynomialDeg2 => write!(f, "polynomial_deg2"),
            CalibrationMethod::PolynomialDeg3 => write!(f, "polynomial_deg3"),
            CalibrationMethod::HybridTps => write!(f, "hybrid_tps"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point2D) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationConfig {
    pub calibration_type: CalibrationType,
    pub method: CalibrationMethod,
    pub projector_mode: bool,
    pub margin_ratio: f64,
    pub sample_count: usize,
    pub initial_discard_samples: usize,
    pub min_touch_duration_ms: u64,
    pub max_deviation_norm: f64,
    pub outlier_sigma: f64,
    pub target_size_px: f32,
    pub high_contrast: bool,
    pub target_timeout_sec: u32,
    pub touch_sensitivity: f32,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            calibration_type: CalibrationType::Points9,
            method: CalibrationMethod::HomographyRansac,
            projector_mode: true,
            margin_ratio: 0.08,
            sample_count: 16,
            initial_discard_samples: 3,
            min_touch_duration_ms: 220,
            max_deviation_norm: 0.045,
            outlier_sigma: 2.2,
            target_size_px: 38.0,
            high_contrast: false,
            target_timeout_sec: 30,
            touch_sensitivity: 0.75,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationPointRecord {
    pub index: usize,
    pub expected_screen: [f64; 2],
    pub raw_center: [f64; 2],
    pub raw_normalized: [f64; 2],
    pub predicted_screen: [f64; 2],
    pub error_dx: f64,
    pub error_dy: f64,
    pub pixel_error: f64,
    pub accepted_samples: usize,
    pub rejected_outliers: usize,
    pub is_ransac_inlier: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum CalibrationQuality {
    Excellent,
    Good,
    Acceptable,
    #[default]
    Poor,
}

impl fmt::Display for CalibrationQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalibrationQuality::Excellent => write!(f, "Excellent"),
            CalibrationQuality::Good => write!(f, "Good"),
            CalibrationQuality::Acceptable => write!(f, "Acceptable"),
            CalibrationQuality::Poor => write!(f, "Poor"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorStats {
    pub average: f64,
    pub maximum: f64,
    pub rms: f64,
    pub quality: CalibrationQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpsModelData {
    pub control_points: Vec<[f64; 2]>,
    pub weights_x: Vec<f64>,
    pub weights_y: Vec<f64>,
    pub affine_x: [f64; 3],
    pub affine_y: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationModelData {
    pub homography_norm_to_screen: [f64; 9],
    pub homography_norm_to_output_norm: [f64; 9],
    pub x11_ctm: [f64; 9],
    pub libinput_affine: [f64; 6],
    pub poly_degree: Option<usize>,
    pub poly_coeff_x: Option<Vec<f64>>,
    pub poly_coeff_y: Option<Vec<f64>>,
    pub tps_residual: Option<TpsModelData>,
    pub ransac_inliers: usize,
    pub total_points: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationProfile {
    pub profile_name: String,
    pub device: String,
    pub device_id: String,
    pub vendor_id: String,
    pub product_id: String,
    pub device_name: String,
    pub device_path: String,
    pub display: String,
    pub display_id: String,
    pub resolution: [u32; 2],
    pub display_position: [i32; 2],
    pub virtual_desktop_size: [u32; 2],
    pub calibration_type: CalibrationType,
    pub method: CalibrationMethod,
    pub matrix: [f64; 9],
    pub model: CalibrationModelData,
    pub raw_bounds: AxisBounds,
    pub points: Vec<CalibrationPointRecord>,
    pub error: ErrorStats,
    pub timestamp: DateTime<Utc>,
    pub session_type: String,
    pub display_server: DisplayServer,
    pub auto_apply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    pub backup_id: String,
    pub created_at: DateTime<Utc>,
    pub reason: String,
    pub previous_profile: Option<CalibrationProfile>,
    pub previous_x11_matrix: Option<[f64; 9]>,
    pub device_id: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub language: Language,
    pub theme: ThemeMode,
    pub calibration: CalibrationConfig,
    pub auto_detect_on_startup: bool,
    pub prefer_uinput_daemon_for_projective: bool,
    pub log_level: LogLevel,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::English,
            theme: ThemeMode::Light,
            calibration: CalibrationConfig::default(),
            auto_detect_on_startup: true,
            prefer_uinput_daemon_for_projective: false,
            log_level: LogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemPermissions {
    pub is_root: bool,
    pub in_input_group: bool,
    pub user_groups: Vec<String>,
    pub evdev_readable_count: usize,
    pub evdev_total_count: usize,
    pub uinput_writable: bool,
    pub udev_rules_writable: bool,
    pub udev_rule_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibinputStatus {
    pub available: bool,
    pub version: Option<String>,
    pub detected_touch_devices: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub generated_at: DateTime<Utc>,
    pub session: SessionInfo,
    pub displays: Vec<DisplayInfo>,
    pub input_devices: Vec<InputDeviceInfo>,
    pub libinput: LibinputStatus,
    pub permissions: SystemPermissions,
    pub profiles_count: usize,
    pub profile_names: Vec<String>,
    pub backups_count: usize,
    pub autostart_enabled: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub backend_used: String,
    pub applied_matrix: [f64; 9],
    pub persisted_autostart: bool,
    pub requires_relogin_or_udev: bool,
    pub warnings: Vec<String>,
}

pub mod apply;
pub mod diagnostics;
pub mod displays;
pub mod input_stream;
pub mod inputs;
pub mod projector;
pub mod session;

pub use apply::{
    UDEV_PERMISSIONS_RULE, apply_calibration, generate_udev_calibration_rule,
    install_permissions_udev_rule, reset_calibration, run_uinput_daemon,
};
pub use diagnostics::{collect_diagnostics, format_diagnostics_text};
pub use displays::{detect_displays, virtual_desktop_size};
pub use input_stream::{
    InputStreamHandle, RawTouchEvent, start_device_stream, start_hotplug_monitor,
};
pub use inputs::{detect_input_devices, probe_libinput_status};
pub use projector::ProjectorGuard;
pub use session::{check_permissions, detect_session};

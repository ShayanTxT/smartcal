use smartcal_core::{DisplayServer, SessionInfo, SystemPermissions, WaylandCompositor};
use std::env;

#[cfg(target_os = "linux")]
fn read_os_release() -> String {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                return val.trim_matches('"').to_string();
            }
        }
    }
    "Linux".to_string()
}

#[cfg(not(target_os = "linux"))]
fn read_os_release() -> String {
    format!(
        "{} (Development / Cross-Platform Host)",
        std::env::consts::OS
    )
}

#[cfg(target_os = "linux")]
fn read_kernel_version() -> String {
    unsafe {
        let mut uts: libc::utsname = std::mem::zeroed();
        if libc::uname(&mut uts) == 0 {
            let rel = std::ffi::CStr::from_ptr(uts.release.as_ptr());
            return rel.to_string_lossy().into_owned();
        }
    }
    "unknown".to_string()
}

#[cfg(not(target_os = "linux"))]
fn read_kernel_version() -> String {
    "non-linux-host".to_string()
}

pub fn detect_session() -> SessionInfo {
    let os_name = read_os_release();
    let kernel_version = read_kernel_version();
    let desktop_environment = env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "Unknown".to_string());
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| {
        if env::var_os("WAYLAND_DISPLAY").is_some() {
            "wayland".to_string()
        } else if env::var_os("DISPLAY").is_some() {
            "x11".to_string()
        } else {
            "unknown".to_string()
        }
    });
    let display_env = env::var("DISPLAY").ok();
    let wayland_display_env = env::var("WAYLAND_DISPLAY").ok();

    let lower_sess = session_type.to_lowercase();
    let display_server = if lower_sess.contains("wayland") || wayland_display_env.is_some() {
        DisplayServer::Wayland
    } else if lower_sess.contains("x11") || display_env.is_some() {
        DisplayServer::X11
    } else {
        DisplayServer::Unknown
    };

    let compositor = if display_server == DisplayServer::Wayland {
        detect_wayland_compositor(&desktop_environment)
    } else {
        WaylandCompositor::NoneOrUnknown
    };

    SessionInfo {
        os_name,
        kernel_version,
        desktop_environment,
        session_type,
        display_server,
        compositor,
        display_env,
        wayland_display_env,
    }
}

fn detect_wayland_compositor(de: &str) -> WaylandCompositor {
    if env::var_os("SWAYSOCK").is_some() {
        return WaylandCompositor::Sway;
    }
    if env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        return WaylandCompositor::Hyprland;
    }
    let lower = de.to_lowercase();
    if lower.contains("gnome") || lower.contains("ubuntu") || lower.contains("cinnamon") {
        WaylandCompositor::Mutter
    } else if lower.contains("kde") || lower.contains("plasma") {
        WaylandCompositor::KWin
    } else if lower.contains("sway") {
        WaylandCompositor::Sway
    } else if lower.contains("hyprland") {
        WaylandCompositor::Hyprland
    } else if lower.contains("weston") {
        WaylandCompositor::Weston
    } else if lower.contains("wlroots") || lower.contains("labwc") || lower.contains("wayfire") {
        WaylandCompositor::Wlroots
    } else {
        WaylandCompositor::NoneOrUnknown
    }
}

#[cfg(target_os = "linux")]
pub fn check_permissions() -> SystemPermissions {
    use std::fs;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Path;
    use std::process::Command;

    let is_root = unsafe { libc::geteuid() == 0 };

    let mut user_groups = Vec::new();
    if let Ok(out) = Command::new("id").arg("-Gn").output()
        && out.status.success()
    {
        user_groups = String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
    }
    let in_input_group = is_root || user_groups.iter().any(|g| g == "input");

    let mut evdev_total_count = 0usize;
    let mut evdev_readable_count = 0usize;
    if let Ok(entries) = fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("event") {
                evdev_total_count += 1;
                if fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NONBLOCK)
                    .open(entry.path())
                    .is_ok()
                {
                    evdev_readable_count += 1;
                }
            }
        }
    }

    let uinput_writable = fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")
        .is_ok();

    let udev_rules_writable = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open("/etc/udev/rules.d/.smartcal-perm-test")
        .map(|_| {
            let _ = fs::remove_file("/etc/udev/rules.d/.smartcal-perm-test");
            true
        })
        .unwrap_or(false);

    let udev_rule_installed = Path::new("/etc/udev/rules.d/99-smartcal-permissions.rules").exists()
        || Path::new("/lib/udev/rules.d/99-smartcal-permissions.rules").exists();

    SystemPermissions {
        is_root,
        in_input_group,
        user_groups,
        evdev_readable_count,
        evdev_total_count,
        uinput_writable,
        udev_rules_writable,
        udev_rule_installed,
    }
}

#[cfg(not(target_os = "linux"))]
pub fn check_permissions() -> SystemPermissions {
    SystemPermissions {
        is_root: false,
        in_input_group: false,
        user_groups: vec![],
        evdev_readable_count: 0,
        evdev_total_count: 0,
        uinput_writable: false,
        udev_rules_writable: false,
        udev_rule_installed: false,
    }
}

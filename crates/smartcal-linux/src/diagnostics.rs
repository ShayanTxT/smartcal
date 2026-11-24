use crate::displays::detect_displays;
use crate::inputs::{detect_input_devices, probe_libinput_status};
use crate::session::{check_permissions, detect_session};
use chrono::Utc;
use smartcal_core::{DiagnosticsReport, ProfileStore};
use std::fmt::Write;

pub fn collect_diagnostics(store: &ProfileStore) -> DiagnosticsReport {
    let session = detect_session();
    let displays = detect_displays(session.display_server);
    let input_devices = detect_input_devices();
    let libinput = probe_libinput_status();
    let permissions = check_permissions();
    let profiles = store.list_profiles();
    let backups = store.list_backups();

    let mut notes = Vec::new();
    if permissions.evdev_total_count > 0 && permissions.evdev_readable_count == 0 {
        notes.push(
            "No /dev/input/event* devices are readable by the current user. Add user to 'input' group or install 99-smartcal-permissions.rules.".into(),
        );
    }
    if session.display_server == smartcal_core::DisplayServer::Wayland {
        notes.push(
            "Wayland session detected: compositor security policies prevent arbitrary global pointer injection; calibration uses compositor IPC, udev LIBINPUT_CALIBRATION_MATRIX, or uinput daemon.".into(),
        );
    }
    if displays.len() > 1 {
        notes.push(format!(
            "Multi-monitor configuration detected ({} displays). Ensure Calibration Target matches the Interactive Projector display.",
            displays.len()
        ));
    }

    DiagnosticsReport {
        generated_at: Utc::now(),
        session,
        displays,
        input_devices,
        libinput,
        permissions,
        profiles_count: profiles.len(),
        profile_names: profiles.into_iter().map(|p| p.profile_name).collect(),
        backups_count: backups.len(),
        autostart_enabled: ProfileStore::is_autostart_enabled(),
        notes,
    }
}

pub fn format_diagnostics_text(report: &DiagnosticsReport) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "========================================================"
    );
    let _ = writeln!(
        out,
        " SmartCal — Smart Board & Projector Calibration Report"
    );
    let _ = writeln!(out, " Generated: {}", report.generated_at.to_rfc3339());
    let _ = writeln!(
        out,
        "========================================================\n"
    );

    let _ = writeln!(out, "[SYSTEM & SESSION]");
    let _ = writeln!(out, "OS                  : {}", report.session.os_name);
    let _ = writeln!(
        out,
        "Kernel              : {}",
        report.session.kernel_version
    );
    let _ = writeln!(
        out,
        "Desktop Environment : {}",
        report.session.desktop_environment
    );
    let _ = writeln!(out, "Session Type        : {}", report.session.session_type);
    let _ = writeln!(
        out,
        "Display Server      : {}",
        report.session.display_server
    );
    let _ = writeln!(out, "Wayland Compositor  : {}", report.session.compositor);
    let _ = writeln!(
        out,
        "DISPLAY / WAYLAND   : {:?} / {:?}\n",
        report.session.display_env, report.session.wayland_display_env
    );

    let _ = writeln!(out, "[DISPLAYS ({})]", report.displays.len());
    for d in &report.displays {
        let _ = writeln!(
            out,
            "- {} ({}) : {}x{} @ {:.2} Hz | Pos ({}, {}) | Primary={} | Scale={:.2}",
            d.id, d.name, d.width, d.height, d.refresh_hz, d.x, d.y, d.primary, d.scale_factor
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(
        out,
        "[INPUT DEVICES / EVDEV ({})]",
        report.input_devices.len()
    );
    for dev in &report.input_devices {
        let _ = writeln!(
            out,
            "- {} [{}] | VID:PID {} | Path: {} | Cap: {} | Status: {} | R/W: {}/{} | XInputID: {:?}",
            dev.name,
            dev.kind,
            dev.vid_pid_label(),
            dev.event_path,
            dev.touch_capability,
            dev.status,
            dev.readable,
            dev.writable,
            dev.xinput_id
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "[LIBINPUT STATUS]");
    let _ = writeln!(out, "Available : {}", report.libinput.available);
    let _ = writeln!(out, "Version   : {:?}", report.libinput.version);
    let _ = writeln!(out, "Summary   : {}", report.libinput.summary);
    for td in &report.libinput.detected_touch_devices {
        let _ = writeln!(out, "  * {td}");
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "[PERMISSIONS]");
    let _ = writeln!(out, "Running as root      : {}", report.permissions.is_root);
    let _ = writeln!(
        out,
        "In 'input' group     : {}",
        report.permissions.in_input_group
    );
    let _ = writeln!(
        out,
        "Groups               : {}",
        report.permissions.user_groups.join(", ")
    );
    let _ = writeln!(
        out,
        "evdev readable       : {}/{}",
        report.permissions.evdev_readable_count, report.permissions.evdev_total_count
    );
    let _ = writeln!(
        out,
        "/dev/uinput writable : {}",
        report.permissions.uinput_writable
    );
    let _ = writeln!(
        out,
        "udev rule installed  : {}",
        report.permissions.udev_rule_installed
    );
    let _ = writeln!(
        out,
        "udev rules writable  : {}\n",
        report.permissions.udev_rules_writable
    );

    let _ = writeln!(out, "[CALIBRATION PROFILES & BACKUPS]");
    let _ = writeln!(
        out,
        "Profiles ({}) : {}",
        report.profiles_count,
        report.profile_names.join(", ")
    );
    let _ = writeln!(out, "Backups       : {}", report.backups_count);
    let _ = writeln!(out, "Autostart     : {}\n", report.autostart_enabled);

    if !report.notes.is_empty() {
        let _ = writeln!(out, "[NOTES & RECOMMENDATIONS]");
        for n in &report.notes {
            let _ = writeln!(out, "* {n}");
        }
    }

    out
}

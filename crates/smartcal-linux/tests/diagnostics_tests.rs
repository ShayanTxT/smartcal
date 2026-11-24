use smartcal_core::ProfileStore;
use smartcal_linux::{UDEV_PERMISSIONS_RULE, collect_diagnostics, format_diagnostics_text};

#[test]
fn diagnostics_report_formats_all_required_sections() {
    let dir = std::env::temp_dir().join(format!("smartcal_diag_test_{}", std::process::id()));
    let store = ProfileStore::with_root(dir.clone());
    let report = collect_diagnostics(&store);
    let txt = format_diagnostics_text(&report);

    for header in [
        "[SYSTEM & SESSION]",
        "[DISPLAYS",
        "[INPUT DEVICES / EVDEV",
        "[LIBINPUT STATUS]",
        "[PERMISSIONS]",
        "[CALIBRATION PROFILES & BACKUPS]",
    ] {
        assert!(txt.contains(header), "Missing diagnostics header: {header}");
    }
    assert!(UDEV_PERMISSIONS_RULE.contains("ID_INPUT_TOUCHSCREEN"));
    let _ = std::fs::remove_dir_all(&dir);
}

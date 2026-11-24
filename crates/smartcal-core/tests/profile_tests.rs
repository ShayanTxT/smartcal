use chrono::Utc;
use smartcal_core::{
    AxisBounds, CalibrationMethod, CalibrationModelData, CalibrationProfile, CalibrationQuality,
    CalibrationType, ConnectionStatus, DeviceKind, DisplayInfo, DisplayServer, ErrorStats,
    InputDeviceInfo, ProfileStore, TouchCapability,
};
use std::path::PathBuf;

fn temp_store(name: &str) -> (ProfileStore, PathBuf) {
    let dir = std::env::temp_dir().join(format!("smartcal_test_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    (ProfileStore::with_root(dir.clone()), dir)
}

#[test]
fn profile_save_load_backup_and_match() {
    let (store, dir) = temp_store("roundtrip");

    let profile = CalibrationProfile {
        profile_name: "classroom-projector".into(),
        device: "USB Touchscreen".into(),
        device_id: "04f3:2a1c@event5".into(),
        vendor_id: "04f3".into(),
        product_id: "2a1c".into(),
        device_name: "USB Touchscreen".into(),
        device_path: "/dev/input/event5".into(),
        display: "HDMI-1".into(),
        display_id: "Display 2".into(),
        resolution: [1920, 1080],
        display_position: [1920, 0],
        virtual_desktop_size: [3840, 1080],
        calibration_type: CalibrationType::Points9,
        method: CalibrationMethod::HomographyRansac,
        matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        model: CalibrationModelData {
            homography_norm_to_screen: [1920.0, 0.0, 0.0, 0.0, 1080.0, 0.0, 0.0, 0.0, 1.0],
            homography_norm_to_output_norm: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            x11_ctm: [0.5, 0.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            libinput_affine: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            poly_degree: None,
            poly_coeff_x: None,
            poly_coeff_y: None,
            tps_residual: None,
            ransac_inliers: 9,
            total_points: 9,
        },
        raw_bounds: AxisBounds::default(),
        points: vec![],
        error: ErrorStats {
            average: 2.1,
            maximum: 4.5,
            rms: 2.5,
            quality: CalibrationQuality::Excellent,
        },
        timestamp: Utc::now(),
        session_type: "x11".into(),
        display_server: DisplayServer::X11,
        auto_apply: true,
    };

    let saved_path = store.save_profile(&profile).unwrap();
    assert!(saved_path.exists());

    let loaded = store.load_profile("classroom-projector").unwrap();
    assert_eq!(loaded.vendor_id, "04f3");
    assert_eq!(loaded.resolution, [1920, 1080]);

    let dev = InputDeviceInfo {
        id: "04f3:2a1c@event7".into(),
        name: "USB Touchscreen".into(),
        kind: DeviceKind::Touchscreen,
        event_path: "/dev/input/event7".into(),
        vendor_id: "04f3".into(),
        product_id: "2a1c".into(),
        bus_type: "USB".into(),
        phys: None,
        uniq: None,
        touch_capability: TouchCapability::MultiTouch { max_slots: 10 },
        status: ConnectionStatus::Connected,
        readable: true,
        writable: false,
        xinput_id: Some(11),
        libinput_name: None,
        abs_bounds: Some(AxisBounds::default()),
        current_x11_matrix: None,
    };
    let disp = DisplayInfo {
        id: "Display 2".into(),
        name: "HDMI-1".into(),
        friendly_label: "HDMI-1".into(),
        width: 1920,
        height: 1080,
        x: 1920,
        y: 0,
        refresh_hz: 60.0,
        scale_factor: 1.0,
        primary: false,
        connected: true,
        physical_width_mm: None,
        physical_height_mm: None,
    };

    let matched = store.find_matching_profile(&dev, Some(&disp));
    assert!(matched.is_some());

    store
        .create_backup("test", &dev.id, &dev.name, Some(loaded), None)
        .unwrap();
    assert!(store.latest_backup().is_some());

    let _ = std::fs::remove_dir_all(&dir);
}

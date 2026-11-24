#[cfg(target_os = "linux")]
use smartcal_core::AxisRange;
use smartcal_core::{
    AxisBounds, ConnectionStatus, DeviceKind, InputDeviceInfo, LibinputStatus, TouchCapability,
};

#[cfg(target_os = "linux")]
pub fn detect_input_devices() -> Vec<InputDeviceInfo> {
    use evdev::{AbsoluteAxisCode, Device, EventType, KeyCode};
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    let xinput_map = query_x11_xinput_devices();
    let libinput_names = query_libinput_device_names();

    let mut devices = Vec::new();
    let Ok(entries) = fs::read_dir("/dev/input") else {
        return devices;
    };

    let mut event_paths: Vec<_> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("event") {
                let num = name
                    .strip_prefix("event")
                    .and_then(|n| n.parse::<u32>().ok())
                    .unwrap_or(999);
                Some((num, name, e.path()))
            } else {
                None
            }
        })
        .collect();
    event_paths.sort_by_key(|(num, _, _)| *num);

    for (_, ev_name, path) in event_paths {
        let ev_path_str = path.to_string_lossy().into_owned();
        let sysfs_dir = Path::new("/sys/class/input").join(&ev_name).join("device");

        let sys_name = fs::read_to_string(sysfs_dir.join("name"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| ev_name.clone());
        let sys_vendor = fs::read_to_string(sysfs_dir.join("id/vendor"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let sys_product = fs::read_to_string(sysfs_dir.join("id/product"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let sys_bus = fs::read_to_string(sysfs_dir.join("id/bustype"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "usb".to_string());

        // Read udev database properties from /run/udev/data/c13:<minor>
        let mut udev_props: HashMap<String, String> = HashMap::new();
        if let Ok(meta) = fs::metadata(&path) {
            let rdev = meta.rdev();
            let major = libc::major(rdev);
            let minor = libc::minor(rdev);
            let udev_file = format!("/run/udev/data/c{major}:{minor}");
            if let Ok(content) = fs::read_to_string(udev_file) {
                for line in content.lines() {
                    if let Some(kv) = line.strip_prefix("E:")
                        && let Some((k, v)) = kv.split_once('=')
                    {
                        udev_props.insert(k.to_string(), v.to_string());
                    }
                }
            }
        }

        let is_udev_touchscreen = udev_props
            .get("ID_INPUT_TOUCHSCREEN")
            .is_some_and(|v| v == "1");
        let is_udev_tablet = udev_props.get("ID_INPUT_TABLET").is_some_and(|v| v == "1");
        let is_udev_touchpad = udev_props
            .get("ID_INPUT_TOUCHPAD")
            .is_some_and(|v| v == "1");
        let is_udev_mouse = udev_props.get("ID_INPUT_MOUSE").is_some_and(|v| v == "1");
        let is_udev_keyboard = udev_props
            .get("ID_INPUT_KEYBOARD")
            .is_some_and(|v| v == "1");

        let vendor_id = udev_props
            .get("ID_VENDOR_ID")
            .cloned()
            .filter(|s| !s.is_empty())
            .unwrap_or(sys_vendor);
        let product_id = udev_props
            .get("ID_MODEL_ID")
            .cloned()
            .filter(|s| !s.is_empty())
            .unwrap_or(sys_product);

        let writable = fs::OpenOptions::new().write(true).open(&path).is_ok();

        match Device::open(&path) {
            Ok(dev) => {
                let name = dev.name().unwrap_or(&sys_name).to_string();
                let id_info = dev.input_id();
                let vid = if vendor_id.is_empty() {
                    format!("{:04x}", id_info.vendor())
                } else {
                    vendor_id
                };
                let pid = if product_id.is_empty() {
                    format!("{:04x}", id_info.product())
                } else {
                    product_id
                };

                let keys = dev.supported_keys();
                let abs = dev.supported_absolute_axes();
                let events = dev.supported_events();

                let has_mt = abs.is_some_and(|a| {
                    a.contains(AbsoluteAxisCode::ABS_MT_POSITION_X)
                        && a.contains(AbsoluteAxisCode::ABS_MT_POSITION_Y)
                });
                let has_abs_xy = abs.is_some_and(|a| {
                    a.contains(AbsoluteAxisCode::ABS_X) && a.contains(AbsoluteAxisCode::ABS_Y)
                });
                let has_btn_touch = keys.is_some_and(|k| k.contains(KeyCode::BTN_TOUCH));
                let has_pen = keys.is_some_and(|k| {
                    k.contains(KeyCode::BTN_TOOL_PEN) || k.contains(KeyCode::BTN_STYLUS)
                });
                let has_mouse_btn = keys.is_some_and(|k| k.contains(KeyCode::BTN_LEFT));
                let has_rel = events.contains(EventType::RELATIVE);

                let kind = if is_udev_tablet || has_pen {
                    DeviceKind::PenStylus
                } else if is_udev_touchscreen
                    || (has_btn_touch && (has_mt || has_abs_xy) && !is_udev_touchpad)
                {
                    DeviceKind::Touchscreen
                } else if is_udev_touchpad {
                    DeviceKind::Touchpad
                } else if is_udev_mouse || (has_rel && has_mouse_btn) {
                    DeviceKind::Mouse
                } else if has_abs_xy {
                    DeviceKind::Touchscreen
                } else if is_udev_keyboard {
                    continue; // Skip pure keyboards from pointer/touch calibration list
                } else {
                    continue;
                };

                let mut max_slots = 0u32;
                let mut abs_bounds = None;
                if let Ok(state) = dev.get_abs_state() {
                    let mut x_range = None;
                    let mut y_range = None;
                    let mut mt_x = None;
                    let mut mt_y = None;

                    for (code, info) in state.iter().enumerate() {
                        let c = AbsoluteAxisCode(code as u16);
                        let r = AxisRange {
                            min: info.minimum,
                            max: info.maximum,
                            fuzz: info.fuzz,
                            flat: info.flat,
                            resolution: info.resolution,
                        };
                        if c == AbsoluteAxisCode::ABS_X {
                            x_range = Some(r);
                        } else if c == AbsoluteAxisCode::ABS_Y {
                            y_range = Some(r);
                        } else if c == AbsoluteAxisCode::ABS_MT_POSITION_X {
                            mt_x = Some(r);
                        } else if c == AbsoluteAxisCode::ABS_MT_POSITION_Y {
                            mt_y = Some(r);
                        } else if c == AbsoluteAxisCode::ABS_MT_SLOT && info.maximum >= info.minimum
                        {
                            max_slots = (info.maximum - info.minimum + 1) as u32;
                        }
                    }

                    if let (Some(xr), Some(yr)) = (mt_x.or(x_range), mt_y.or(y_range))
                        && xr.max > xr.min
                        && yr.max > yr.min
                    {
                        abs_bounds = Some(AxisBounds { x: xr, y: yr });
                    }
                }

                let touch_capability = if has_mt {
                    TouchCapability::MultiTouch { max_slots }
                } else if has_abs_xy && (has_btn_touch || has_pen) {
                    TouchCapability::SingleTouch
                } else {
                    TouchCapability::PointerOnly
                };

                let xmatch = xinput_map.get(&name).or_else(|| {
                    xinput_map
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(&name))
                        .map(|(_, v)| v)
                });

                let libinput_name = libinput_names
                    .iter()
                    .find(|n| n.eq_ignore_ascii_case(&name))
                    .cloned();

                devices.push(InputDeviceInfo {
                    id: format!("{vid}:{pid}@{ev_name}"),
                    name,
                    kind,
                    event_path: ev_path_str,
                    vendor_id: vid,
                    product_id: pid,
                    bus_type: format!("{:?}", id_info.bus_type()),
                    phys: dev.physical_path().map(|s| s.to_string()),
                    uniq: dev.unique_name().map(|s| s.to_string()),
                    touch_capability,
                    status: ConnectionStatus::Connected,
                    readable: true,
                    writable,
                    xinput_id: xmatch.map(|m| m.0),
                    libinput_name,
                    abs_bounds,
                    current_x11_matrix: xmatch.and_then(|m| m.1),
                });
            }
            Err(_) => {
                // Still show pointer/touch devices from sysfs + udev even without read permission!
                if is_udev_keyboard && !is_udev_touchscreen && !is_udev_mouse && !is_udev_tablet {
                    continue;
                }
                let kind = if is_udev_tablet {
                    DeviceKind::PenStylus
                } else if is_udev_touchscreen {
                    DeviceKind::Touchscreen
                } else if is_udev_touchpad {
                    DeviceKind::Touchpad
                } else if is_udev_mouse {
                    DeviceKind::Mouse
                } else {
                    continue;
                };

                let xmatch = xinput_map.get(&sys_name);
                devices.push(InputDeviceInfo {
                    id: format!("{vendor_id}:{product_id}@{ev_name}"),
                    name: sys_name,
                    kind,
                    event_path: ev_path_str,
                    vendor_id,
                    product_id,
                    bus_type: sys_bus,
                    phys: None,
                    uniq: None,
                    touch_capability: if matches!(
                        kind,
                        DeviceKind::Touchscreen | DeviceKind::PenStylus
                    ) {
                        TouchCapability::SingleTouch
                    } else {
                        TouchCapability::PointerOnly
                    },
                    status: ConnectionStatus::PermissionDenied,
                    readable: false,
                    writable: false,
                    xinput_id: xmatch.map(|m| m.0),
                    libinput_name: None,
                    abs_bounds: None,
                    current_x11_matrix: xmatch.and_then(|m| m.1),
                });
            }
        }
    }

    // Sort Touchscreens & Pen/Stylus first for classroom usability.
    devices.sort_by_key(|d| match d.kind {
        DeviceKind::Touchscreen => 0,
        DeviceKind::PenStylus => 1,
        DeviceKind::Touchpad => 2,
        DeviceKind::Mouse => 3,
        DeviceKind::Other => 4,
    });

    devices
}

#[cfg(not(target_os = "linux"))]
pub fn detect_input_devices() -> Vec<InputDeviceInfo> {
    vec![InputDeviceInfo {
        id: "0000:0000@preview".to_string(),
        name: "Simulated Classroom Touchscreen (Non-Linux Host)".to_string(),
        kind: DeviceKind::Touchscreen,
        event_path: "/dev/input/event0 (preview)".to_string(),
        vendor_id: "0000".to_string(),
        product_id: "0000".to_string(),
        bus_type: "VIRTUAL".to_string(),
        phys: None,
        uniq: None,
        touch_capability: TouchCapability::MultiTouch { max_slots: 10 },
        status: ConnectionStatus::Connected,
        readable: true,
        writable: false,
        xinput_id: None,
        libinput_name: None,
        abs_bounds: Some(AxisBounds::default()),
        current_x11_matrix: None,
    }]
}

#[cfg(target_os = "linux")]
fn query_x11_xinput_devices() -> std::collections::HashMap<String, (u32, Option<[f64; 9]>)> {
    use std::collections::HashMap;
    use x11rb::protocol::xinput::{self, ConnectionExt as XiExt};
    use x11rb::protocol::xproto::ConnectionExt as XprotoExt;

    let mut map = HashMap::new();
    let Ok((conn, _)) = x11rb::connect(None) else {
        return map;
    };

    let ctm_atom = conn
        .intern_atom(false, b"Coordinate Transformation Matrix")
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom);
    let float_atom = conn
        .intern_atom(false, b"FLOAT")
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom)
        .unwrap_or(0);

    let Ok(cookie) = conn.xinput_xi_query_device(xinput::Device::ALL) else {
        return map;
    };
    let Ok(reply) = cookie.reply() else {
        return map;
    };

    for dev in reply.infos {
        if !matches!(
            dev.type_,
            xinput::DeviceType::SLAVE_POINTER | xinput::DeviceType::FLOATING_SLAVE
        ) {
            continue;
        }
        let name = String::from_utf8_lossy(&dev.name).into_owned();
        let mut matrix = None;
        if let Some(atom) = ctm_atom
            && let Ok(prop_c) =
                conn.xinput_xi_get_property(dev.deviceid, false, atom, float_atom, 0, 9)
            && let Ok(prop) = prop_c.reply()
            && prop.num_items == 9
            && let Some(vals) = prop.items.as_data32()
            && vals.len() == 9
        {
            let mut m = [0.0_f64; 9];
            for (i, bits) in vals.iter().enumerate() {
                m[i] = f32::from_bits(*bits) as f64;
            }
            matrix = Some(m);
        }
        map.insert(name, (dev.deviceid as u32, matrix));
    }

    map
}

#[cfg(target_os = "linux")]
fn query_libinput_device_names() -> Vec<String> {
    use std::process::Command;
    let Ok(out) = Command::new("libinput").arg("list-devices").output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|l| l.strip_prefix("Device:").map(|v| v.trim().to_string()))
        .collect()
}

pub fn probe_libinput_status() -> LibinputStatus {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let version = Command::new("libinput")
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        let list_out = Command::new("libinput").arg("list-devices").output().ok();
        let mut touch_devs = Vec::new();
        if let Some(out) = &list_out
            && out.status.success()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut cur_dev = String::new();
            for line in text.lines() {
                if let Some(d) = line.strip_prefix("Device:") {
                    cur_dev = d.trim().to_string();
                } else if line.starts_with("Capabilities:")
                    && (line.contains("touch") || line.contains("tablet"))
                    && !cur_dev.is_empty()
                {
                    touch_devs.push(cur_dev.clone());
                }
            }
        }

        let available = version.is_some() || list_out.as_ref().is_some_and(|o| o.status.success());
        let summary = if available {
            format!(
                "libinput {} active ({} touch/tablet devices listed)",
                version.as_deref().unwrap_or("tools"),
                touch_devs.len()
            )
        } else {
            "libinput CLI not installed or insufficient permissions; using evdev/udev directly"
                .into()
        };

        LibinputStatus {
            available,
            version,
            detected_touch_devices: touch_devs,
            summary,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        LibinputStatus {
            available: false,
            version: None,
            detected_touch_devices: vec![],
            summary: "libinput is only active on Linux hosts".into(),
        }
    }
}

use smartcal_core::{DisplayInfo, DisplayServer};
#[cfg(target_os = "linux")]
use smartcal_core::{LogLevel, record_log};

pub fn virtual_desktop_size(displays: &[DisplayInfo]) -> (u32, u32) {
    let mut max_w = 0_i32;
    let mut max_h = 0_i32;
    for d in displays {
        if d.connected {
            max_w = max_w.max(d.x + d.width as i32);
            max_h = max_h.max(d.y + d.height as i32);
        }
    }
    ((max_w.max(1)) as u32, (max_h.max(1)) as u32)
}

#[cfg(target_os = "linux")]
pub fn detect_displays(server: DisplayServer) -> Vec<DisplayInfo> {
    let mut displays = match server {
        DisplayServer::X11 => {
            let mut x11 = detect_displays_x11();
            if x11.is_empty() {
                x11 = detect_displays_xrandr_cli();
            }
            x11
        }
        DisplayServer::Wayland => {
            let mut wl = detect_displays_wayland();
            if wl.is_empty() {
                wl = detect_displays_compositor_cli();
            }
            if wl.is_empty() {
                wl = detect_displays_xrandr_cli();
            }
            wl
        }
        DisplayServer::Unknown => {
            let mut d = detect_displays_x11();
            if d.is_empty() {
                d = detect_displays_wayland();
            }
            d
        }
    };

    if displays.is_empty() {
        displays = detect_displays_drm_sysfs();
    }

    if displays.is_empty() {
        record_log(
            LogLevel::Warning,
            "displays",
            "No active X11/Wayland/DRM outputs reported; using fallback virtual display",
        );
        displays.push(DisplayInfo {
            id: "Display-1".to_string(),
            name: "Virtual-1".to_string(),
            friendly_label: "Primary Display".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            refresh_hz: 60.0,
            scale_factor: 1.0,
            primary: true,
            connected: true,
            physical_width_mm: None,
            physical_height_mm: None,
        });
    }

    displays
}

#[cfg(not(target_os = "linux"))]
pub fn detect_displays(_server: DisplayServer) -> Vec<DisplayInfo> {
    vec![DisplayInfo {
        id: "Display-1".to_string(),
        name: "Default-1".to_string(),
        friendly_label: "Primary Display (Host Preview)".to_string(),
        width: 1920,
        height: 1080,
        x: 0,
        y: 0,
        refresh_hz: 60.0,
        scale_factor: 1.0,
        primary: true,
        connected: true,
        physical_width_mm: None,
        physical_height_mm: None,
    }]
}

#[cfg(target_os = "linux")]
fn detect_displays_x11() -> Vec<DisplayInfo> {
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::{self, ConnectionExt as RandrExt};

    let Ok((conn, screen_num)) = x11rb::connect(None) else {
        return Vec::new();
    };
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let primary_cookie = conn.randr_get_output_primary(root).ok();
    let primary_atom_output = primary_cookie
        .and_then(|c| c.reply().ok())
        .map(|r| r.output)
        .unwrap_or(0);

    let Ok(res_cookie) = conn.randr_get_screen_resources_current(root) else {
        return Vec::new();
    };
    let Ok(res) = res_cookie.reply() else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for (idx, &output) in res.outputs.iter().enumerate() {
        let Ok(info_cookie) = conn.randr_get_output_info(output, res.config_timestamp) else {
            continue;
        };
        let Ok(info) = info_cookie.reply() else {
            continue;
        };

        if info.connection != randr::Connection::CONNECTED || info.crtc == 0 {
            continue;
        }

        let Ok(crtc_cookie) = conn.randr_get_crtc_info(info.crtc, res.config_timestamp) else {
            continue;
        };
        let Ok(crtc) = crtc_cookie.reply() else {
            continue;
        };

        let mut refresh_hz = 60.0;
        if let Some(mode) = res.modes.iter().find(|m| m.id == crtc.mode) {
            let denom = (mode.htotal as f64) * (mode.vtotal as f64);
            if denom > 0.0 {
                refresh_hz = (mode.dot_clock as f64) / denom;
            }
        }

        let name = String::from_utf8_lossy(&info.name).into_owned();
        result.push(DisplayInfo {
            id: format!("Display {}", idx + 1),
            friendly_label: format!("{name} ({} × {})", crtc.width, crtc.height),
            name,
            width: crtc.width as u32,
            height: crtc.height as u32,
            x: crtc.x as i32,
            y: crtc.y as i32,
            refresh_hz,
            scale_factor: 1.0,
            primary: output == primary_atom_output,
            connected: true,
            physical_width_mm: (info.mm_width > 0).then_some(info.mm_width),
            physical_height_mm: (info.mm_height > 0).then_some(info.mm_height),
        });
    }

    result
}

#[cfg(target_os = "linux")]
fn detect_displays_xrandr_cli() -> Vec<DisplayInfo> {
    use std::process::Command;

    let Ok(out) = Command::new("xrandr").arg("--query").output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut displays = Vec::new();
    let mut current_idx: Option<usize> = None;

    for line in text.lines() {
        if !line.starts_with(' ') && line.contains(" connected") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let name = parts.first().copied().unwrap_or("Display").to_string();
            let primary = parts.contains(&"primary");
            if let Some(geom) = parts.iter().find(|p| p.contains('x') && p.contains('+'))
                && let Some((wh, xy)) = geom.split_once('+')
                && let Some((w_s, h_s)) = wh.split_once('x')
                && let Some((x_s, y_s)) = xy.split_once('+')
                && let (Ok(w), Ok(h), Ok(x), Ok(y)) = (
                    w_s.parse::<u32>(),
                    h_s.parse::<u32>(),
                    x_s.parse::<i32>(),
                    y_s.parse::<i32>(),
                )
            {
                let idx = displays.len();
                displays.push(DisplayInfo {
                    id: format!("Display {}", idx + 1),
                    friendly_label: format!("{name} ({w} × {h})"),
                    name,
                    width: w,
                    height: h,
                    x,
                    y,
                    refresh_hz: 60.0,
                    scale_factor: 1.0,
                    primary,
                    connected: true,
                    physical_width_mm: None,
                    physical_height_mm: None,
                });
                current_idx = Some(idx);
            } else {
                current_idx = None;
            }
        } else if let Some(idx) = current_idx
            && line.contains('*')
        {
            for tok in line.split_whitespace() {
                if tok.contains('*') {
                    let clean = tok.trim_end_matches(['*', '+']);
                    if let Ok(hz) = clean.parse::<f64>() {
                        displays[idx].refresh_hz = hz;
                    }
                    break;
                }
            }
        }
    }

    displays
}

#[cfg(target_os = "linux")]
fn detect_displays_wayland() -> Vec<DisplayInfo> {
    use wayland_client::protocol::{wl_output, wl_registry};
    use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};
    use wayland_protocols::xdg::xdg_output::zv1::client::{zxdg_output_manager_v1, zxdg_output_v1};

    #[derive(Default)]
    struct RawWlOutput {
        global_name: u32,
        name: Option<String>,
        description: Option<String>,
        width: u32,
        height: u32,
        logical_w: Option<u32>,
        logical_h: Option<u32>,
        x: i32,
        y: i32,
        refresh_hz: f64,
        scale: i32,
        phys_w: Option<u32>,
        phys_h: Option<u32>,
        wl_obj: Option<wl_output::WlOutput>,
    }

    struct WlState {
        xdg_mgr: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
        outputs: Vec<RawWlOutput>,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for WlState {
        fn event(
            state: &mut Self,
            reg: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            if let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
            {
                if interface == wl_output::WlOutput::interface().name {
                    let ver = version.min(4);
                    let out = reg.bind::<wl_output::WlOutput, u32, Self>(name, ver, qh, name);
                    let raw = RawWlOutput {
                        global_name: name,
                        refresh_hz: 60.0,
                        scale: 1,
                        wl_obj: Some(out.clone()),
                        ..Default::default()
                    };
                    if let Some(mgr) = &state.xdg_mgr {
                        mgr.get_xdg_output(&out, qh, name);
                    }
                    state.outputs.push(raw);
                    let _ = &mut state.outputs.last_mut().unwrap().global_name;
                } else if interface == zxdg_output_manager_v1::ZxdgOutputManagerV1::interface().name
                {
                    let ver = version.min(3);
                    let mgr = reg.bind::<zxdg_output_manager_v1::ZxdgOutputManagerV1, (), Self>(
                        name,
                        ver,
                        qh,
                        (),
                    );
                    for o in &state.outputs {
                        if let Some(wl_obj) = &o.wl_obj {
                            mgr.get_xdg_output(wl_obj, qh, o.global_name);
                        }
                    }
                    state.xdg_mgr = Some(mgr);
                }
            }
        }
    }

    impl Dispatch<wl_output::WlOutput, u32> for WlState {
        fn event(
            state: &mut Self,
            _: &wl_output::WlOutput,
            event: wl_output::Event,
            id: &u32,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let Some(out) = state.outputs.iter_mut().find(|o| o.global_name == *id) else {
                return;
            };
            match event {
                wl_output::Event::Geometry {
                    x,
                    y,
                    physical_width,
                    physical_height,
                    model,
                    ..
                } => {
                    out.x = x;
                    out.y = y;
                    if physical_width > 0 {
                        out.phys_w = Some(physical_width as u32);
                    }
                    if physical_height > 0 {
                        out.phys_h = Some(physical_height as u32);
                    }
                    if out.description.is_none() && !model.is_empty() {
                        out.description = Some(model);
                    }
                }
                wl_output::Event::Mode {
                    flags,
                    width,
                    height,
                    refresh,
                } => {
                    if let wayland_client::WEnum::Value(f) = flags
                        && f.contains(wl_output::Mode::Current)
                    {
                        out.width = width.max(0) as u32;
                        out.height = height.max(0) as u32;
                        if refresh > 0 {
                            out.refresh_hz = refresh as f64 / 1000.0;
                        }
                    } else if out.width == 0 {
                        out.width = width.max(0) as u32;
                        out.height = height.max(0) as u32;
                        if refresh > 0 {
                            out.refresh_hz = refresh as f64 / 1000.0;
                        }
                    }
                }
                wl_output::Event::Scale { factor } => {
                    out.scale = factor.max(1);
                }
                wl_output::Event::Name { name } => {
                    out.name = Some(name);
                }
                wl_output::Event::Description { description } => {
                    out.description = Some(description);
                }
                _ => {}
            }
        }
    }

    impl Dispatch<zxdg_output_v1::ZxdgOutputV1, u32> for WlState {
        fn event(
            state: &mut Self,
            _: &zxdg_output_v1::ZxdgOutputV1,
            event: zxdg_output_v1::Event,
            id: &u32,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let Some(out) = state.outputs.iter_mut().find(|o| o.global_name == *id) else {
                return;
            };
            match event {
                zxdg_output_v1::Event::LogicalPosition { x, y } => {
                    out.x = x;
                    out.y = y;
                }
                zxdg_output_v1::Event::LogicalSize { width, height } => {
                    out.logical_w = Some(width.max(0) as u32);
                    out.logical_h = Some(height.max(0) as u32);
                }
                zxdg_output_v1::Event::Name { name } => {
                    out.name = Some(name);
                }
                zxdg_output_v1::Event::Description { description } => {
                    out.description = Some(description);
                }
                _ => {}
            }
        }
    }

    delegate_noop!(WlState: ignore zxdg_output_manager_v1::ZxdgOutputManagerV1);

    let Ok(conn) = Connection::connect_to_env() else {
        return Vec::new();
    };
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();
    let _registry = display.get_registry(&qh, ());

    let mut state = WlState {
        xdg_mgr: None,
        outputs: Vec::new(),
    };
    let _ = event_queue.roundtrip(&mut state);
    let _ = event_queue.roundtrip(&mut state);

    state
        .outputs
        .into_iter()
        .enumerate()
        .filter(|(_, o)| o.width > 0 && o.height > 0)
        .map(|(idx, o)| {
            let name = o.name.unwrap_or_else(|| format!("WL-{}", idx + 1));
            let label_desc = o.description.unwrap_or_else(|| name.clone());
            let w = o.logical_w.unwrap_or(o.width);
            let h = o.logical_h.unwrap_or(o.height);
            DisplayInfo {
                id: format!("Display {}", idx + 1),
                friendly_label: format!("{name} — {label_desc}"),
                name,
                width: w,
                height: h,
                x: o.x,
                y: o.y,
                refresh_hz: o.refresh_hz,
                scale_factor: o.scale as f64,
                primary: idx == 0,
                connected: true,
                physical_width_mm: o.phys_w,
                physical_height_mm: o.phys_h,
            }
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn detect_displays_compositor_cli() -> Vec<DisplayInfo> {
    use std::process::Command;
    // Try wlr-randr for wlroots compositors
    if let Ok(out) = Command::new("wlr-randr").output()
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        let mut list = Vec::new();
        let mut cur_name: Option<String> = None;
        for line in text.lines() {
            if !line.starts_with(' ') && !line.is_empty() {
                cur_name = line.split_whitespace().next().map(|s| s.to_string());
            } else if line.contains("(current)")
                && let Some(name) = cur_name.take()
            {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some((w_s, h_s)) = parts.first().and_then(|s| s.split_once('x'))
                    && let (Ok(w), Ok(h)) = (w_s.parse::<u32>(), h_s.parse::<u32>())
                {
                    let idx = list.len();
                    list.push(DisplayInfo {
                        id: format!("Display {}", idx + 1),
                        friendly_label: format!("{name} ({w} × {h})"),
                        name,
                        width: w,
                        height: h,
                        x: 0,
                        y: 0,
                        refresh_hz: 60.0,
                        scale_factor: 1.0,
                        primary: idx == 0,
                        connected: true,
                        physical_width_mm: None,
                        physical_height_mm: None,
                    });
                }
            }
        }
        if !list.is_empty() {
            return list;
        }
    }
    Vec::new()
}

#[cfg(target_os = "linux")]
fn detect_displays_drm_sysfs() -> Vec<DisplayInfo> {
    use std::fs;
    let mut list = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return list;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.contains('-') {
            continue;
        }
        let status_path = entry.path().join("status");
        if fs::read_to_string(status_path)
            .map(|s| s.trim() == "connected")
            .unwrap_or(false)
        {
            let modes = fs::read_to_string(entry.path().join("modes")).unwrap_or_default();
            let first_mode = modes.lines().next().unwrap_or("1920x1080");
            let (w, h) = first_mode
                .split_once('x')
                .and_then(|(ws, hs)| Some((ws.parse().ok()?, hs.parse().ok()?)))
                .unwrap_or((1920, 1080));
            let idx = list.len();
            list.push(DisplayInfo {
                id: format!("Display {}", idx + 1),
                friendly_label: format!("{name} ({w} × {h})"),
                name,
                width: w,
                height: h,
                x: 0,
                y: 0,
                refresh_hz: 60.0,
                scale_factor: 1.0,
                primary: idx == 0,
                connected: true,
                physical_width_mm: None,
                physical_height_mm: None,
            });
        }
    }
    list
}

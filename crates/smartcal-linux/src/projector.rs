use smartcal_core::{DisplayServer, LogLevel, record_log};

#[cfg(target_os = "linux")]
use std::process::{Child, Command};

#[cfg(target_os = "linux")]
type X11ScreenSaverState = (u16, u16, u8, u8);

/// RAII Guard for Projector Mode:
/// - Prevents screen saver & sleep during calibration
/// - Captures & restores X11 DPMS / ScreenSaver settings on Drop
pub struct ProjectorGuard {
    active: bool,
    #[cfg(target_os = "linux")]
    inhibit_child: Option<Child>,
    #[cfg(target_os = "linux")]
    dbus_cookie: Option<u32>,
    #[cfg(target_os = "linux")]
    x11_prev_screensaver: Option<X11ScreenSaverState>,
    #[cfg(target_os = "linux")]
    x11_prev_dpms_enabled: Option<bool>,
}

impl ProjectorGuard {
    pub fn enter(projector_mode: bool, server: DisplayServer) -> Self {
        if !projector_mode {
            return Self {
                active: false,
                #[cfg(target_os = "linux")]
                inhibit_child: None,
                #[cfg(target_os = "linux")]
                dbus_cookie: None,
                #[cfg(target_os = "linux")]
                x11_prev_screensaver: None,
                #[cfg(target_os = "linux")]
                x11_prev_dpms_enabled: None,
            };
        }

        record_log(
            LogLevel::Info,
            "projector",
            "Activating Projector Mode guard (inhibiting sleep & screensaver)",
        );

        #[cfg(target_os = "linux")]
        {
            let inhibit_child = Command::new("systemd-inhibit")
                .args([
                    "--what=idle:sleep",
                    "--who=SmartCal",
                    "--why=Classroom Smart Board calibration in progress",
                    "sleep",
                    "3600",
                ])
                .spawn()
                .ok();

            let dbus_cookie = inhibit_screensaver_dbus();
            let (x11_prev_screensaver, x11_prev_dpms_enabled) = if server == DisplayServer::X11 {
                disable_x11_blanking()
            } else {
                (None, None)
            };

            Self {
                active: true,
                inhibit_child,
                dbus_cookie,
                x11_prev_screensaver,
                x11_prev_dpms_enabled,
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = server;
            Self { active: true }
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for ProjectorGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        record_log(
            LogLevel::Info,
            "projector",
            "Restoring system sleep and screensaver settings after Projector Mode",
        );

        #[cfg(target_os = "linux")]
        {
            if let Some(mut child) = self.inhibit_child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            if let Some(cookie) = self.dbus_cookie.take() {
                uninhibit_screensaver_dbus(cookie);
            }
            restore_x11_blanking(self.x11_prev_screensaver, self.x11_prev_dpms_enabled);
        }
    }
}

#[cfg(target_os = "linux")]
fn inhibit_screensaver_dbus() -> Option<u32> {
    let out = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.ScreenSaver",
            "--object-path",
            "/org/freedesktop/ScreenSaver",
            "--method",
            "org.freedesktop.ScreenSaver.Inhibit",
            "SmartCal",
            "Smart Board Calibration",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .trim_start_matches('(')
        .trim_end_matches([')', ',', ' ', '\n'])
        .split_whitespace()
        .last()
        .and_then(|s| s.parse::<u32>().ok())
}

#[cfg(target_os = "linux")]
fn uninhibit_screensaver_dbus(cookie: u32) {
    let _ = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.ScreenSaver",
            "--object-path",
            "/org/freedesktop/ScreenSaver",
            "--method",
            "org.freedesktop.ScreenSaver.UnInhibit",
            &cookie.to_string(),
        ])
        .output();
}

#[cfg(target_os = "linux")]
fn disable_x11_blanking() -> (Option<X11ScreenSaverState>, Option<bool>) {
    use x11rb::connection::Connection;
    use x11rb::protocol::dpms::ConnectionExt as DpmsExt;
    use x11rb::protocol::xproto::ConnectionExt as XprotoExt;

    let Ok((conn, _)) = x11rb::connect(None) else {
        return (None, None);
    };

    let prev_ss = conn
        .get_screen_saver()
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| {
            (
                r.timeout,
                r.interval,
                r.prefer_blanking.into(),
                r.allow_exposures.into(),
            )
        });

    if prev_ss.is_some() {
        let _ = conn.set_screen_saver(
            0,
            0,
            x11rb::protocol::xproto::Blanking::NOT_PREFERRED,
            x11rb::protocol::xproto::Exposures::ALLOWED,
        );
    }

    let prev_dpms = conn
        .dpms_info()
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.state);

    if prev_dpms == Some(true) {
        let _ = conn.dpms_disable();
    }
    let _ = conn.flush();

    (prev_ss, prev_dpms)
}

#[cfg(target_os = "linux")]
fn restore_x11_blanking(prev_ss: Option<(u16, u16, u8, u8)>, prev_dpms_enabled: Option<bool>) {
    use x11rb::connection::Connection;
    use x11rb::protocol::dpms::ConnectionExt as DpmsExt;
    use x11rb::protocol::xproto::{Blanking, ConnectionExt as XprotoExt, Exposures};

    let Ok((conn, _)) = x11rb::connect(None) else {
        return;
    };

    if let Some((timeout, interval, blanking, exposures)) = prev_ss {
        let b = match blanking {
            0 => Blanking::NOT_PREFERRED,
            1 => Blanking::PREFERRED,
            _ => Blanking::DEFAULT,
        };
        let e = match exposures {
            0 => Exposures::NOT_ALLOWED,
            1 => Exposures::ALLOWED,
            _ => Exposures::DEFAULT,
        };
        let _ = conn.set_screen_saver(timeout as i16, interval as i16, b, e);
    }

    if prev_dpms_enabled == Some(true) {
        let _ = conn.dpms_enable();
    }
    let _ = conn.flush();
}

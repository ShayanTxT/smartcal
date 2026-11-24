use smartcal_core::{InputDeviceInfo, Point2D};
#[cfg(target_os = "linux")]
use smartcal_core::{LogLevel, record_log};
#[cfg(target_os = "linux")]
use smartcal_math::SingleTouchTracker;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone)]
pub enum RawTouchEvent {
    Down(Point2D),
    Sample(Point2D),
    Up,
    Disconnected(String),
    PermissionDenied(String),
    HotplugChanged,
}

pub struct InputStreamHandle {
    stop_flag: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    pub rx: Receiver<RawTouchEvent>,
}

impl InputStreamHandle {
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for InputStreamHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts a resilient non-blocking evdev listener thread for the given device.
/// Tracks ONLY a single primary touch contact so multi-touch never corrupts calibration.
pub fn start_device_stream(device: &InputDeviceInfo) -> InputStreamHandle {
    let (tx, rx) = mpsc::channel();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_flag);
    let path = device.event_path.clone();
    let dev_name = device.name.clone();

    let handle = thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            use evdev::{AbsoluteAxisCode, Device, EventSummary, KeyCode};
            use std::os::unix::io::AsRawFd;

            let Ok(mut dev) = Device::open(&path) else {
                record_log(
                    LogLevel::Warning,
                    "input_stream",
                    format!("Permission denied opening {path}"),
                );
                let _ = tx.send(RawTouchEvent::PermissionDenied(path));
                return;
            };

            let fd = dev.as_raw_fd();
            // Set O_NONBLOCK cleanly so reads never hang during disconnect or stop.
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                if flags >= 0 {
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
            }

            let mut tracker = SingleTouchTracker::default();
            let mut was_down = false;
            let mut pending_move = false;

            while !stop_clone.load(Ordering::SeqCst) {
                let mut pfd = libc::pollfd {
                    fd,
                    events: libc::POLLIN,
                    revents: 0,
                };
                let poll_res = unsafe { libc::poll(&mut pfd, 1, 40) };
                if poll_res < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    record_log(
                        LogLevel::Warning,
                        "input_stream",
                        format!("Poll error on {dev_name}: {err}"),
                    );
                    let _ = tx.send(RawTouchEvent::Disconnected(path.clone()));
                    break;
                }
                if (pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)) != 0 {
                    record_log(
                        LogLevel::Warning,
                        "input_stream",
                        format!("Device disconnected: {dev_name} ({path})"),
                    );
                    let _ = tx.send(RawTouchEvent::Disconnected(path.clone()));
                    break;
                }
                if poll_res == 0 {
                    continue;
                }

                match dev.fetch_events() {
                    Ok(events) => {
                        for ev in events {
                            match ev.destructure() {
                                EventSummary::AbsoluteAxis(_, code, value) => match code {
                                    AbsoluteAxisCode::ABS_MT_SLOT => {
                                        tracker.set_current_slot(value);
                                    }
                                    AbsoluteAxisCode::ABS_MT_TRACKING_ID => {
                                        tracker.on_tracking_id(value);
                                    }
                                    AbsoluteAxisCode::ABS_MT_POSITION_X => {
                                        tracker.update_mt_x(value as f64);
                                        pending_move = true;
                                    }
                                    AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                                        tracker.update_mt_y(value as f64);
                                        pending_move = true;
                                    }
                                    AbsoluteAxisCode::ABS_X => {
                                        tracker.update_abs_x(value as f64);
                                        pending_move = true;
                                    }
                                    AbsoluteAxisCode::ABS_Y => {
                                        tracker.update_abs_y(value as f64);
                                        pending_move = true;
                                    }
                                    _ => {}
                                },
                                EventSummary::Key(_, key, value) => {
                                    if key == KeyCode::BTN_TOUCH || key == KeyCode::BTN_LEFT {
                                        tracker.on_btn_touch(value != 0);
                                    }
                                }
                                EventSummary::Synchronization(_, _, _) => {
                                    let is_down = tracker.is_touch_down();
                                    if is_down && !was_down {
                                        if let Some(pt) = tracker.current_point() {
                                            let _ = tx.send(RawTouchEvent::Down(pt));
                                        }
                                    } else if is_down && pending_move {
                                        if let Some(pt) = tracker.current_point() {
                                            let _ = tx.send(RawTouchEvent::Sample(pt));
                                        }
                                    } else if !is_down && was_down {
                                        let _ = tx.send(RawTouchEvent::Up);
                                    }
                                    was_down = is_down;
                                    pending_move = false;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => {
                        record_log(
                            LogLevel::Warning,
                            "input_stream",
                            format!("Device read stopped ({dev_name}): {e}"),
                        );
                        let _ = tx.send(RawTouchEvent::Disconnected(path.clone()));
                        break;
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (&tx, &path, &dev_name);
            while !stop_clone.load(Ordering::SeqCst) {
                thread::sleep(std::time::Duration::from_millis(80));
            }
        }
    });

    InputStreamHandle {
        stop_flag,
        handle: Some(handle),
        rx,
    }
}

/// Starts a lightweight udev/netlink hot-plug monitor notifying on USB input device attach/detach.
pub fn start_hotplug_monitor() -> InputStreamHandle {
    let (tx, rx) = mpsc::channel();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_flag);

    let handle = thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            let sock = unsafe {
                libc::socket(
                    libc::AF_NETLINK,
                    libc::SOCK_RAW | libc::SOCK_NONBLOCK,
                    libc::NETLINK_KOBJECT_UEVENT,
                )
            };
            if sock >= 0 {
                let mut addr: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
                addr.nl_family = libc::AF_NETLINK as u16;
                addr.nl_pid = 0;
                addr.nl_groups = 1; // kernel uevents

                let bound = unsafe {
                    libc::bind(
                        sock,
                        (&addr as *const libc::sockaddr_nl).cast::<libc::sockaddr>(),
                        std::mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
                    )
                };

                if bound == 0 {
                    let mut buf = [0u8; 4096];
                    while !stop_clone.load(Ordering::SeqCst) {
                        let mut pfd = libc::pollfd {
                            fd: sock,
                            events: libc::POLLIN,
                            revents: 0,
                        };
                        let res = unsafe { libc::poll(&mut pfd, 1, 250) };
                        if res > 0 {
                            let n =
                                unsafe { libc::recv(sock, buf.as_mut_ptr().cast(), buf.len(), 0) };
                            if n > 0 {
                                let msg = String::from_utf8_lossy(&buf[..n as usize]);
                                if msg.contains("SUBSYSTEM=input") || msg.contains("SUBSYSTEM=drm")
                                {
                                    let _ = tx.send(RawTouchEvent::HotplugChanged);
                                }
                            }
                        }
                    }
                }
                unsafe { libc::close(sock) };
                return;
            }
        }

        while !stop_clone.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(500));
        }
        let _ = &tx;
    });

    InputStreamHandle {
        stop_flag,
        handle: Some(handle),
        rx,
    }
}

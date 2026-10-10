//! Native macOS controller: translates Apple TV remote buttons and touch gestures
//! into macOS CoreGraphics keyboard/mouse events, volume adjustments, and display sleep/wake.

use std::ffi::c_void;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use atv_core::{AtvDelegate, EventKind, TouchPhase, TrackpadMode};
use tracing::{debug, info, warn};

/// Whether notifications are in Chinese. The macOS app passes its language in `ATV_LANG`; run
/// from a terminal, the usual locale variables decide. Without either, the text stays Chinese.
fn is_chinese() -> bool {
    static CHINESE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CHINESE.get_or_init(|| {
        ["ATV_LANG", "LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|key| std::env::var(key).ok())
            .find(|value| !value.is_empty())
            .map_or(true, |value| value.to_lowercase().starts_with("zh"))
    })
}

/// The Chinese or the English version of a notification.
fn tr(zh: &'static str, en: &'static str) -> &'static str {
    if is_chinese() { zh } else { en }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

pub type CGEventRef = *mut c_void;
pub type CGEventSourceRef = *mut c_void;

const K_CGHID_EVENT_TAP: u32 = 0;
const K_CG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
const K_CG_EVENT_LEFT_MOUSE_UP: u32 = 2;
const K_CG_EVENT_MOUSE_MOVED: u32 = 5;
const K_CG_MOUSE_BUTTON_LEFT: u32 = 0;

#[allow(dead_code)]
pub const NX_KEYTYPE_SOUND_UP: i32 = 0;
#[allow(dead_code)]
pub const NX_KEYTYPE_SOUND_DOWN: i32 = 1;
#[allow(dead_code)]
pub const NX_KEYTYPE_BRIGHTNESS_UP: i32 = 2;
#[allow(dead_code)]
pub const NX_KEYTYPE_BRIGHTNESS_DOWN: i32 = 3;
#[allow(dead_code)]
pub const NX_KEYTYPE_MUTE: i32 = 7;
#[allow(dead_code)]
pub const NX_KEYTYPE_PLAY: i32 = 16;
#[allow(dead_code)]
pub const NX_KEYTYPE_NEXT: i32 = 17;
#[allow(dead_code)]
pub const NX_KEYTYPE_PREVIOUS: i32 = 18;
#[allow(dead_code)]
pub const NX_KEYTYPE_FAST: i32 = 19;
#[allow(dead_code)]
pub const NX_KEYTYPE_REWIND: i32 = 20;

const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "objc")]
extern "C" {
    fn CGEventCreate(source: CGEventSourceRef) -> CGEventRef;
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGEventCreateKeyboardEvent(
        source: CGEventSourceRef,
        virtual_key: u16,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventCreateMouseEvent(
        source: CGEventSourceRef,
        mouse_type: u32,
        mouse_cursor_position: CGPoint,
        mouse_button: u32,
    ) -> CGEventRef;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CFRelease(cf: *const c_void);
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: *const c_void;
    static kCFBooleanTrue: *const c_void;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        num_values: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *const c_void;
    fn objc_getClass(name: *const u8) -> *mut c_void;
    fn sel_registerName(name: *const u8) -> *mut c_void;
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
    fn dlsym(handle: *mut c_void, symbol: *const u8) -> *mut c_void;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayPixelsWide(display: u32) -> usize;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
}

/// Send macOS system-defined media key (Play/Pause, Vol+/-, Mute, Next/Prev)
/// which triggers the native macOS Now Playing daemon and HUD volume bezel.
pub fn send_system_media_key(key: i32) {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let cls_name = b"NSEvent\0";
        let sel_name = b"otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:\0";
        let cg_event_sel_name = b"CGEvent\0";

        let nsevent_class = objc_getClass(cls_name.as_ptr());
        let other_event_sel = sel_registerName(sel_name.as_ptr());
        let cg_event_sel = sel_registerName(cg_event_sel_name.as_ptr());

        if nsevent_class.is_null() || other_event_sel.is_null() || cg_event_sel.is_null() {
            warn!("[macOS] Failed to get NSEvent class or selectors for media key");
            objc_autoreleasePoolPop(pool);
            return;
        }

        type OtherEventFn = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            CGPoint,
            usize,
            f64,
            isize,
            *mut c_void,
            i16,
            isize,
            isize,
        ) -> *mut c_void;

        type CgEventFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> CGEventRef;

        let msg_send = dlsym(RTLD_DEFAULT, b"objc_msgSend\0".as_ptr());
        if msg_send.is_null() {
            warn!("[macOS] Failed to locate objc_msgSend");
            objc_autoreleasePoolPop(pool);
            return;
        }
        let other_event_fn: OtherEventFn = std::mem::transmute(msg_send);
        let cg_event_fn: CgEventFn = std::mem::transmute(msg_send);

        for down in [true, false] {
            let state: i32 = if down { 0xa } else { 0xb };
            let flags: usize = (state as usize) << 8;
            let data1: isize = ((key as isize) << 16) | ((state as isize) << 8);

            let event = other_event_fn(
                nsevent_class,
                other_event_sel,
                14, // NSSystemDefined
                CGPoint { x: 0.0, y: 0.0 },
                flags,
                0.0,
                0,
                std::ptr::null_mut(),
                8, // NX_SUBTYPE_AUX_CONTROL_BUTTONS
                data1,
                -1,
            );

            if !event.is_null() {
                let cg_event = cg_event_fn(event, cg_event_sel);
                if !cg_event.is_null() {
                    CGEventPost(K_CGHID_EVENT_TAP, cg_event);
                }
            }
        }
        objc_autoreleasePoolPop(pool);
    }
}

/// Query actual system audio settings from macOS via AppleScript.
pub fn get_mac_audio_settings() -> (f64, bool) {
    if let Ok(output) = Command::new("/usr/bin/osascript")
        .args(["-e", "get {output volume of (get volume settings), output muted of (get volume settings)}"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = s.trim().split(',').map(str::trim).collect();
            if parts.len() == 2 {
                let vol_pct: f64 = parts[0].parse().unwrap_or(50.0);
                let muted: bool = parts[1].parse().unwrap_or(false);
                return (vol_pct / 100.0, muted);
            }
        }
    }
    (0.5, false)
}

/// Check if the process has macOS Accessibility permission.
pub fn is_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Request macOS Accessibility permission: triggers system modal dialog and opens System Settings.
pub fn request_accessibility_permission() -> bool {
    unsafe {
        if AXIsProcessTrusted() {
            return true;
        }

        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );

        let trusted = AXIsProcessTrustedWithOptions(dict);
        if !dict.is_null() {
            CFRelease(dict);
        }

        let _ = Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility")
            .spawn();

        trusted
    }
}

/// Virtual keycodes on macOS.
fn key_code_for_button(name: &str) -> Option<u16> {
    match name {
        "up" => Some(126),       // Arrow Up
        "down" => Some(125),     // Arrow Down
        "left" => Some(123),     // Arrow Left
        "right" => Some(124),    // Arrow Right
        "menu" => Some(53),      // Escape
        "home" => Some(115),     // Home
        "channel_up" | "page_up" => Some(116), // Page Up
        "channel_down" | "page_down" => Some(121), // Page Down
        "guide" => Some(48),     // Tab
        _ => None,
    }
}

struct PlayPauseState {
    last_click: Instant,
    click_count: u32,
    timer_generation: u32,
}

pub struct MacDelegate {
    mouse_mode: Arc<AtomicBool>,
    mouse_speed: AtomicU64,
    mouse_accel: Arc<AtomicBool>,
    verbose_events: Arc<AtomicBool>,
    display_asleep: Arc<AtomicBool>,
    cached_audio: std::sync::Mutex<Option<(f64, bool)>>,
    cursor_pos: std::sync::Mutex<Option<CGPoint>>,
    play_pause_state: Arc<std::sync::Mutex<PlayPauseState>>,
}

impl MacDelegate {
    #[allow(dead_code)]
    pub fn new(mouse_mode: Arc<AtomicBool>) -> Self {
        Self::new_with_full_settings(mouse_mode, 1.0, true, false, true)
    }

    #[allow(dead_code)]
    pub fn new_with_settings(
        mouse_mode: Arc<AtomicBool>,
        initial_speed: f64,
        initial_accel: bool,
    ) -> Self {
        Self::new_with_full_settings(mouse_mode, initial_speed, initial_accel, false, true)
    }

    pub fn new_with_full_settings(
        mouse_mode: Arc<AtomicBool>,
        initial_speed: f64,
        initial_accel: bool,
        verbose_events: bool,
        prompt_accessibility: bool,
    ) -> Self {
        if !is_accessibility_trusted() {
            if prompt_accessibility {
                eprintln!(
                    "\n================================================================================\n\
                     ⚠️  [macOS Accessibility Permission Required / 需要辅助功能权限]\n\
                     atv-cli requires Accessibility permission to simulate keyboard and mouse controls.\n\
                     \n\
                     👉 Note: In modern macOS, the settings pane is displayed as:\n\
                     \"Device Control and Data Access\" (设备控制与数据访问) > \"Accessibility\" (辅助功能).\n\
                     👉 Please ensure the toggle is ON for Terminal / iTerm (or Apple TV Remote.app).\n\
                     ================================================================================\n"
                );
                request_accessibility_permission();
            } else {
                warn!("[macOS] Child process started without direct Accessibility trust; relying on host application bundle.");
            }
        } else {
            info!("macOS Accessibility permission confirmed");
        }

        let initial_audio = get_mac_audio_settings();
        let speed = initial_speed.clamp(0.1, 10.0);

        Self {
            mouse_mode,
            mouse_speed: AtomicU64::new(speed.to_bits()),
            mouse_accel: Arc::new(AtomicBool::new(initial_accel)),
            verbose_events: Arc::new(AtomicBool::new(verbose_events)),
            display_asleep: Arc::new(AtomicBool::new(false)),
            cached_audio: std::sync::Mutex::new(Some(initial_audio)),
            cursor_pos: std::sync::Mutex::new(None),
            play_pause_state: Arc::new(std::sync::Mutex::new(PlayPauseState {
                last_click: Instant::now() - Duration::from_secs(10),
                click_count: 0,
                timer_generation: 0,
            })),
        }
    }

    pub fn is_mouse_mode(&self) -> bool {
        self.mouse_mode.load(Ordering::SeqCst)
    }

    pub fn set_mouse_mode(&self, enabled: bool) {
        self.mouse_mode.store(enabled, Ordering::SeqCst);
        self.notify_mode_changed(enabled);
    }

    pub fn toggle_mouse_mode(&self) -> bool {
        let current = self.is_mouse_mode();
        let new_mode = !current;
        self.set_mouse_mode(new_mode);
        new_mode
    }

    pub fn notify_trackpad_mode_changed(&self, mode: TrackpadMode) {
        let title = "Apple TV Remote";
        let message = match mode {
            TrackpadMode::Mouse => tr("已切换为: 🖱️ 鼠标光标模式 (Mouse Mode)", "Switched to: 🖱️ Mouse Pointer"),
            TrackpadMode::Direction => tr("已切换为: ◀▲▼▶ 方向键模式 (Arrow Keys 上下左右)", "Switched to: ◀▲▼▶ Arrow Keys"),
            TrackpadMode::Idle => tr("已切换为: 👁️ 触控板空置模式 (仅看轨迹实时预览)", "Switched to: 👁️ Idle (live preview only)"),
        };
        info!("[macOS] {message}");
        let script = format!("display notification \"{message}\" with title \"{title}\"");
        let _ = Command::new("/usr/bin/osascript").args(["-e", &script]).spawn();
    }

    pub fn notify_mode_changed(&self, mouse_mode: bool) {
        self.notify_trackpad_mode_changed(if mouse_mode {
            TrackpadMode::Mouse
        } else {
            TrackpadMode::Direction
        });
    }

    pub fn notify_media_action(&self, message: &str) {
        let title = "Apple TV Remote";
        info!("[macOS] {message}");
        let script = format!("display notification \"{message}\" with title \"{title}\"");
        let _ = Command::new("/usr/bin/osascript").args(["-e", &script]).spawn();
    }

    fn refresh_cached_audio(&self) {
        std::thread::spawn(|| {
            let _ = get_mac_audio_settings();
        });
    }

    fn display_size() -> (f64, f64) {
        unsafe {
            let main_disp = CGMainDisplayID();
            let w = CGDisplayPixelsWide(main_disp);
            let h = CGDisplayPixelsHigh(main_disp);
            if w > 0 && h > 0 {
                (w as f64 - 1.0, h as f64 - 1.0)
            } else {
                (3840.0, 2160.0)
            }
        }
    }

    fn send_key_press(virtual_key: u16) {
        unsafe {
            let event_down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), virtual_key, true);
            if !event_down.is_null() {
                CGEventPost(K_CGHID_EVENT_TAP, event_down);
                CFRelease(event_down as *const c_void);
            }
            let event_up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), virtual_key, false);
            if !event_up.is_null() {
                CGEventPost(K_CGHID_EVENT_TAP, event_up);
                CFRelease(event_up as *const c_void);
            }
        }
    }

    fn current_mouse_location() -> CGPoint {
        unsafe {
            let event = CGEventCreate(std::ptr::null_mut());
            if !event.is_null() {
                let loc = CGEventGetLocation(event);
                CFRelease(event as *const c_void);
                loc
            } else {
                CGPoint { x: 0.0, y: 0.0 }
            }
        }
    }

    fn mouse_move(&self, dx: f64, dy: f64) {
        let mut guard = match self.cursor_pos.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        let cur = match *guard {
            Some(pos) => pos,
            None => {
                let pos = Self::current_mouse_location();
                *guard = Some(pos);
                pos
            }
        };

        let dist = (dx * dx + dy * dy).sqrt();
        if dist == 0.0 {
            return;
        }

        let speed = f64::from_bits(self.mouse_speed.load(Ordering::Relaxed));
        let use_accel = self.mouse_accel.load(Ordering::Relaxed);

        // Non-linear trackpad acceleration (ballistics):
        // - Tiny movements (< 1.5 units): 1.15x precision mode
        // - Moderate motion (1.5 - 5.0 units): smooth 1.15x -> 2.375x
        // - Fast flick (> 5.0 units): accelerates up to 4.5x for effortless travel
        let accel = if use_accel {
            if dist < 1.5 {
                1.15
            } else if dist < 5.0 {
                1.15 + (dist - 1.5) * 0.35
            } else {
                2.375 + (dist - 5.0).min(10.0) * 0.22
            }
        } else {
            1.6 // Pure linear 1:1 mode
        };

        let base_scale = 1.6;
        let scale = base_scale * speed;
        let move_x = dx * accel * scale;
        let move_y = dy * accel * scale;

        let (screen_w, screen_h) = Self::display_size();
        let target = CGPoint {
            x: (cur.x + move_x).clamp(0.0, screen_w),
            y: (cur.y + move_y).clamp(0.0, screen_h),
        };

        *guard = Some(target);
        drop(guard);

        unsafe {
            let event = CGEventCreateMouseEvent(
                std::ptr::null_mut(),
                K_CG_EVENT_MOUSE_MOVED,
                target,
                K_CG_MOUSE_BUTTON_LEFT,
            );
            if !event.is_null() {
                CGEventPost(K_CGHID_EVENT_TAP, event);
                CFRelease(event as *const c_void);
            }
        }
    }

    fn mouse_click(&self) {
        let cur = if let Ok(guard) = self.cursor_pos.lock() {
            guard.unwrap_or_else(Self::current_mouse_location)
        } else {
            Self::current_mouse_location()
        };
        unsafe {
            let down = CGEventCreateMouseEvent(
                std::ptr::null_mut(),
                K_CG_EVENT_LEFT_MOUSE_DOWN,
                cur,
                K_CG_MOUSE_BUTTON_LEFT,
            );
            if !down.is_null() {
                CGEventPost(K_CGHID_EVENT_TAP, down);
                CFRelease(down as *const c_void);
            }
            let up = CGEventCreateMouseEvent(
                std::ptr::null_mut(),
                K_CG_EVENT_LEFT_MOUSE_UP,
                cur,
                K_CG_MOUSE_BUTTON_LEFT,
            );
            if !up.is_null() {
                CGEventPost(K_CGHID_EVENT_TAP, up);
                CFRelease(up as *const c_void);
            }
        }
    }

    fn handle_play_pause_click(&self) {
        let mut state = self.play_pause_state.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_click);

        state.last_click = now;
        state.timer_generation = state.timer_generation.wrapping_add(1);
        let current_gen = state.timer_generation;

        if elapsed <= Duration::from_millis(380) {
            state.click_count += 1;
        } else {
            state.click_count = 1;
        }

        let count = state.click_count;
        drop(state);

        // Triple click: instant Previous track
        if count >= 3 {
            info!("[macOS] media previous track (triple-click)");
            send_system_media_key(NX_KEYTYPE_PREVIOUS);
            self.notify_media_action(tr("⏮ 上一曲 (Previous Track)", "⏮ Previous Track"));
            if let Ok(mut s) = self.play_pause_state.lock() {
                s.click_count = 0;
            }
            return;
        }

        // Wait to see if user clicks again (double click window)
        let state_arc = self.play_pause_state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let mut s = state_arc.lock().unwrap();
            if s.timer_generation == current_gen {
                let final_count = s.click_count;
                s.click_count = 0;
                drop(s);
                match final_count {
                    1 => {
                        info!("[macOS] media play/pause (single-click)");
                        send_system_media_key(NX_KEYTYPE_PLAY);
                    }
                    2 => {
                        info!("[macOS] media next track (double-click)");
                        send_system_media_key(NX_KEYTYPE_NEXT);
                        let title = "Apple TV Remote";
                        let message = tr("⏭ 下一曲 (Next Track)", "⏭ Next Track");
                        info!("[macOS] {message}");
                        let script = format!("display notification \"{message}\" with title \"{title}\"");
                        let _ = Command::new("/usr/bin/osascript").args(["-e", &script]).spawn();
                    }
                    _ => {}
                }
            }
        });
    }

    fn wake_display(&self) {
        if self.display_asleep.swap(false, Ordering::SeqCst) {
            let _ = Command::new("/usr/bin/caffeinate")
                .args(["-u", "-t", "1"])
                .spawn();
        }
    }

    fn sleep_display(&self) {
        self.display_asleep.store(true, Ordering::SeqCst);
        let _ = Command::new("/usr/bin/pmset")
            .arg("displaysleepnow")
            .spawn();
    }
}

impl AtvDelegate for MacDelegate {
    fn on_button(&self, name: &str) {
        let verbose = self.verbose_events.load(Ordering::Relaxed);
        if verbose {
            info!("[macOS] button: {name}");
        } else {
            debug!("[macOS] button: {name}");
        }
        if name == "power" {
            if self.display_asleep.load(Ordering::SeqCst) {
                self.wake_display();
            } else {
                self.sleep_display();
            }
            return;
        }

        self.wake_display();

        if let Some(vk) = key_code_for_button(name) {
            Self::send_key_press(vk);
        } else {
            match name {
                "select" => {
                    if self.mouse_mode.load(Ordering::SeqCst) {
                        if verbose {
                            info!("[macOS] select in mouse mode -> simulating mouse click");
                        } else {
                            debug!("[macOS] select in mouse mode -> simulating mouse click");
                        }
                        self.mouse_click();
                    } else {
                        if verbose {
                            info!("[macOS] select in direction mode -> simulating Return key");
                        } else {
                            debug!("[macOS] select in direction mode -> simulating Return key");
                        }
                        Self::send_key_press(36);
                    }
                }
                "play_pause" => {
                    self.handle_play_pause_click();
                }
                "play" => {
                    if verbose { info!("[macOS] media play"); } else { debug!("[macOS] media play"); }
                    send_system_media_key(NX_KEYTYPE_PLAY);
                }
                "pause" => {
                    if verbose { info!("[macOS] media pause"); } else { debug!("[macOS] media pause"); }
                    send_system_media_key(NX_KEYTYPE_PLAY);
                }
                "next" | "next_track" => {
                    if verbose { info!("[macOS] media next track"); } else { debug!("[macOS] media next track"); }
                    send_system_media_key(NX_KEYTYPE_NEXT);
                }
                "prev" | "previous" | "prev_track" | "previous_track" => {
                    if verbose { info!("[macOS] media previous track"); } else { debug!("[macOS] media previous track"); }
                    send_system_media_key(NX_KEYTYPE_PREVIOUS);
                }
                "fast_forward" => {
                    if verbose { info!("[macOS] media fast forward"); } else { debug!("[macOS] media fast forward"); }
                    send_system_media_key(NX_KEYTYPE_FAST);
                }
                "rewind" => {
                    if verbose { info!("[macOS] media rewind"); } else { debug!("[macOS] media rewind"); }
                    send_system_media_key(NX_KEYTYPE_REWIND);
                }
                "volume_up" => {
                    if verbose { info!("[macOS] volume up"); } else { debug!("[macOS] volume up"); }
                    send_system_media_key(NX_KEYTYPE_SOUND_UP);
                    self.refresh_cached_audio();
                }
                "volume_down" => {
                    if verbose { info!("[macOS] volume down"); } else { debug!("[macOS] volume down"); }
                    send_system_media_key(NX_KEYTYPE_SOUND_DOWN);
                    self.refresh_cached_audio();
                }
                "mute" => {
                    if verbose { info!("[macOS] mute toggle"); } else { debug!("[macOS] mute toggle"); }
                    send_system_media_key(NX_KEYTYPE_MUTE);
                    self.refresh_cached_audio();
                }
                "siri" | "toggle_mode" => {
                    info!("[macOS] toggling input mode via remote button");
                    self.toggle_mouse_mode();
                }
                "screensaver" => {
                    let _ = Command::new("/usr/bin/open")
                        .args(["-a", "ScreenSaverEngine"])
                        .spawn();
                }
                "sleep" => self.sleep_display(),
                "wake" => self.wake_display(),
                _ => warn!("[macOS] no handler for button: {name}"),
            }
        }
    }

    fn on_touch(&self, dx: f64, dy: f64, phase: TouchPhase) {
        if !self.mouse_mode.load(Ordering::SeqCst) {
            return;
        }
        match phase {
            TouchPhase::Began => {
                // When a new touch begins, sync with the latest OS mouse location
                if let Ok(mut guard) = self.cursor_pos.lock() {
                    *guard = Some(Self::current_mouse_location());
                }
            }
            TouchPhase::Moved => {
                self.mouse_move(dx, dy);
            }
            _ => {}
        }
    }

    fn on_audio(&self, volume: f64, muted: bool) {
        info!("[macOS] audio sync: vol={volume:.2} muted={muted}");
        if let Ok(mut guard) = self.cached_audio.lock() {
            *guard = Some((volume, muted));
        }
        let percent = (volume * 100.0).round() as u32;
        let script = format!("set volume output volume {percent} with output muted {muted}");
        std::thread::spawn(move || {
            let _ = Command::new("/usr/bin/osascript")
                .args(["-e", &script])
                .status();
        });
    }

    fn on_trackpad_mode_changed(&self, mode: TrackpadMode) {
        self.mouse_mode.store(mode == TrackpadMode::Mouse, Ordering::SeqCst);
        self.notify_trackpad_mode_changed(mode);
    }

    fn on_mode_changed(&self, mouse_mode: bool) {
        self.mouse_mode.store(mouse_mode, Ordering::SeqCst);
        self.notify_mode_changed(mouse_mode);
    }

    fn on_touchpad_settings_changed(&self, speed: f64, acceleration: bool, verbose_events: bool) {
        let clamped = speed.clamp(0.1, 10.0);
        self.mouse_speed.store(clamped.to_bits(), Ordering::Relaxed);
        self.mouse_accel.store(acceleration, Ordering::Relaxed);
        self.verbose_events.store(verbose_events, Ordering::Relaxed);
        info!(
            "[macOS] touchpad settings updated: speed={clamped:.2}x, accel={acceleration}, verbose_events={verbose_events}"
        );
    }

    fn get_touchpad_settings(&self) -> (f64, bool, bool) {
        let speed = f64::from_bits(self.mouse_speed.load(Ordering::Relaxed));
        let accel = self.mouse_accel.load(Ordering::Relaxed);
        let verbose = self.verbose_events.load(Ordering::Relaxed);
        (speed, accel, verbose)
    }

    fn get_screen_size(&self) -> Option<(f64, f64)> {
        Some(Self::display_size())
    }

    fn get_audio_state(&self) -> Option<(f64, bool)> {
        if let Ok(guard) = self.cached_audio.lock() {
            if let Some(state) = *guard {
                return Some(state);
            }
        }
        Some(get_mac_audio_settings())
    }

    fn on_event(&self, kind: EventKind, detail: &str) {
        let verbose = self.verbose_events.load(Ordering::Relaxed);
        match kind {
            EventKind::MouseClick => {
                if verbose {
                    info!("[macOS] mouse click");
                } else {
                    debug!("[macOS] mouse click");
                }
                self.mouse_click();
            }
            EventKind::ClientConnected => {
                info!("[macOS] iPhone connected: {detail}");
            }
            EventKind::ClientDisconnected => {
                info!("[macOS] iPhone disconnected: {detail}");
            }
            _ => {
                if verbose {
                    info!("[macOS] event: {kind:?} {detail}");
                } else {
                    debug!("[macOS] event: {kind:?} {detail}");
                }
            }
        }
    }
}

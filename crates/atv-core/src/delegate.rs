//! Delegate callbacks and event types.
//!
//! The API surface intentionally uses only plain data (strings, f64, small
//! enums) and an `Arc<dyn AtvDelegate>` callback interface so it can be
//! exposed through UniFFI later without shape changes.

/// Phase of a touch-sequence on the phone's touch surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    /// Finger touched down (`_tPh` = 1).
    Began,
    /// Finger moved (`_tPh` = 2/3 intermediate updates).
    Moved,
    /// Finger lifted (`_tPh` = 4).
    Ended,
}

/// Lifecycle and state events emitted by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A client connected to the Companion port. Detail: peer address.
    ClientConnected,
    /// A client disconnected. Detail: peer address and reason.
    ClientDisconnected,
    /// SRP pair-setup completed successfully.
    Paired,
    /// Pair-verify completed; the encrypted channel is up.
    Verified,
    /// `_sessionStart` received. Detail: local/remote session ids.
    SessionStarted,
    /// `_sessionStop` received.
    SessionStopped,
    /// A TV Remote Client session started (`TVRCSessionStart`).
    RemoteReady,
    /// Power/display state toggled via the power button. Detail: "on"/"off".
    PowerState,
    /// `_launchApp` requested. Detail: bundle id.
    LaunchApp,
    /// Volume capabilities were advertised to the client.
    Capabilities,
    /// Any other control message. Detail: the decoded OPACK content.
    ControlMessage,
    /// Mouse mode: a short tap on the touch surface (click).
    MouseClick,
}

/// Operating mode for the Apple TV Remote touch surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrackpadMode {
    /// Directional arrow keys (Up, Down, Left, Right) and Select.
    Direction = 0,
    /// Mouse pointer control and click.
    Mouse = 1,
    /// Idle mode: purely broadcast real-time touch trajectory for preview; no mouse move or arrow key actions triggered.
    Idle = 2,
}

impl TrackpadMode {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Direction,
            1 => Self::Mouse,
            2 => Self::Idle,
            _ => Self::Direction,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Direction => "direction",
            Self::Mouse => "mouse",
            Self::Idle => "idle",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        let trimmed = s.trim().to_ascii_lowercase();
        if trimmed == "direction" || trimmed == "dpad" || trimmed == "arrow" || trimmed == "false" {
            Some(Self::Direction)
        } else if trimmed == "mouse" || trimmed == "true" {
            Some(Self::Mouse)
        } else if trimmed == "idle" || trimmed == "preview" || trimmed == "noop" || trimmed == "dummy" {
            Some(Self::Idle)
        } else {
            None
        }
    }

    pub fn is_mouse(&self) -> bool {
        *self == Self::Mouse
    }

    pub fn is_idle(&self) -> bool {
        *self == Self::Idle
    }

    pub fn is_direction(&self) -> bool {
        *self == Self::Direction
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Direction => Self::Mouse,
            Self::Mouse => Self::Idle,
            Self::Idle => Self::Direction,
        }
    }
}

/// Callbacks invoked (synchronously, from tokio tasks) when the phone
/// interacts with the simulated Apple TV.
///
/// All methods have default no-op implementations; override what you need.
pub trait AtvDelegate: Send + Sync + 'static {
    /// A completed remote button action (fired on button-up), e.g. "up",
    /// "down", "select", "menu", "home", "play_pause", "volume_up",
    /// "volume_down", "mute", "power", "siri". Swipe gestures on the touch
    /// surface also arrive here as directional buttons.
    fn on_button(&self, _name: &str) {}

    /// Touch surface activity. `dx`/`dy` are deltas in phone touchpad units
    /// (the surface is 1000x1000). On `Began` the deltas are zero; on `Ended`
    /// they are the total displacement from the touch origin.
    fn on_touch(&self, _dx: f64, _dy: f64, _phase: TouchPhase) {}

    /// Audio state changed: `volume` in 0.0-1.0 (16 steps), `muted` flag.
    fn on_audio(&self, _volume: f64, _muted: bool) {}

    /// Lifecycle/state event with a human-readable detail string.
    fn on_event(&self, _kind: EventKind, _detail: &str) {}

    /// Random-PIN pairing: a phone started pair-setup; show `pin` (four
    /// digits) to the user until `hide_pairing_pin` is called.
    fn show_pairing_pin(&self, _pin: &str) {}

    /// The pairing attempt that showed a PIN ended: it completed, failed, or
    /// its connection closed.
    fn hide_pairing_pin(&self) {}

    /// Called when the input mode changes (trackpad mode: Direction, Mouse, or Idle).
    fn on_trackpad_mode_changed(&self, mode: TrackpadMode) {
        self.on_mode_changed(mode.is_mouse());
    }

    /// Called when the input mode (directional vs mouse) changes (legacy boolean method).
    fn on_mode_changed(&self, _mouse_mode: bool) {}

    /// Set touchpad settings: speed factor, acceleration toggle, and verbose event logging toggle.
    fn on_touchpad_settings_changed(
        &self,
        _speed: f64,
        _acceleration: bool,
        _verbose_events: bool,
    ) {
    }

    /// Query the current touchpad settings: (speed_factor, acceleration_enabled, verbose_events_enabled).
    fn get_touchpad_settings(&self) -> (f64, bool, bool) {
        (crate::settings::DEFAULT_MOUSE_SPEED, true, false)
    }

    /// Set mouse pointer speed factor and acceleration toggle (legacy helper).
    fn on_mouse_settings_changed(&self, speed: f64, acceleration: bool) {
        let (_, _, verbose) = self.get_touchpad_settings();
        self.on_touchpad_settings_changed(speed, acceleration, verbose);
    }

    /// Query the current mouse settings: (speed_factor, acceleration_enabled).
    fn get_mouse_settings(&self) -> (f64, bool) {
        let (s, a, _) = self.get_touchpad_settings();
        (s, a)
    }

    /// Query current screen bounds: (width, height) in pixels.
    fn get_screen_size(&self) -> Option<(f64, f64)> {
        None
    }

    /// Query the current system audio state: volume in [0.0, 1.0] and muted flag.
    /// Returns None if system volume querying is unsupported or unavailable.
    fn get_audio_state(&self) -> Option<(f64, bool)> {
        None
    }
}

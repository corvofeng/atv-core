//! Server: TCP listeners and Bonjour (mDNS) advertisement.
//!
//! Ported from `fake_atv.py`'s `main()`: three services with the exact TXT
//! records iOS expects (`rpFl=0x36782` etc. gate features like the volume
//! keys), the MRP port answering minimal framing, and the AirPlay port
//! answering HTTP "OK".

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_os = "macos"))]
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use sha2::{Digest, Sha256};

use crate::delegate::{AtvDelegate, EventKind, TrackpadMode};
use crate::error::{Error, Result};
use crate::identity::{self, DeviceIdentity};
use crate::inspector::InspectorHub;
use crate::registry::{ActiveConnectionTracker, DeviceRegistry};
use crate::session::{CompanionSession, DEVICE_MODEL};
use crate::FrameType;

pub const MRP_PORT: u16 = 49152;
pub const COMPANION_PORT: u16 = 49153;
pub const SOURCE_VERSION: &str = "715.2";
pub const DEFAULT_DEVICE_NAME: &str = "Mac Remote";

/// Query the local Mac's computer name via `scutil --get ComputerName`.
pub fn detect_mac_computer_name() -> Option<String> {
    let name = identity::detect_computer_name();
    if name == DEFAULT_DEVICE_NAME {
        None
    } else {
        Some(name)
    }
}

/// Default device name: uses local Mac computer name on macOS, otherwise "Mac Remote".
pub fn default_device_name() -> String {
    identity::detect_computer_name()
}

/// Server configuration.
#[derive(Debug, Clone)]
pub struct AtvConfig {
    /// Device name shown in Apple TV Remote.
    pub name: String,
    /// Four-digit pairing PIN (0-9999).
    pub pin: u32,
    /// LAN IPv4 address to advertise; auto-detected when None.
    pub ip: Option<Ipv4Addr>,
    /// Trackpad mode: touch drives pointer deltas instead of direction keys (legacy boolean).
    pub mouse_mode: bool,
    /// Operating mode: Direction, Mouse, or Idle (preview only).
    pub trackpad_mode: TrackpadMode,
    /// Port for Debug Web UI & Inspector (None to disable, default Some(8765)).
    pub ui_port: Option<u16>,
    /// Initial mouse speed multiplier (default 0.5).
    pub mouse_speed: f64,
    /// Hardware / MAC-like identifier (e.g. "84:2F:57:2E:6C:EE").
    pub device_id: Option<String>,
    /// Unique UUID (e.g. "F232C260-003A-5396-99D1-F8164BB03DF2").
    pub server_identifier: Option<String>,
    /// Persistent private key seed (32 bytes).
    pub private_key: Option<[u8; 32]>,
}

impl Default for AtvConfig {
    fn default() -> Self {
        let identity = DeviceIdentity::detect();
        let settings = crate::settings::UserSettings::load();
        let initial_mode = settings
            .trackpad_mode
            .as_deref()
            .and_then(TrackpadMode::from_str_opt)
            .unwrap_or_else(|| {
                if settings.mouse_mode.unwrap_or(false) {
                    TrackpadMode::Mouse
                } else {
                    TrackpadMode::Direction
                }
            });
        Self {
            name: identity.name,
            pin: 1111,
            ip: None,
            mouse_mode: initial_mode.is_mouse(),
            trackpad_mode: initial_mode,
            ui_port: Some(8765),
            mouse_speed: settings.mouse_speed,
            device_id: Some(identity.device_id),
            server_identifier: Some(identity.server_identifier),
            private_key: Some(identity.private_key),
        }
    }
}

/// A running Apple TV simulator. Drop or call `stop()` to shut down.
pub struct AtvServer {
    config: AtvConfig,
    identity: DeviceIdentity,
    ip: Ipv4Addr,
    mrp_port: u16,
    companion_port: u16,
    airplay_port: u16,
    ui_port: Option<u16>,
    inspector: Arc<InspectorHub>,
    mouse_mode: Arc<AtomicBool>,
    trackpad_mode: Arc<AtomicU8>,
    delegate: Arc<dyn AtvDelegate>,
    shutdown_tx: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    #[cfg(target_os = "macos")]
    dns_sd_children: Vec<tokio::process::Child>,
    #[cfg(not(target_os = "macos"))]
    mdns: ServiceDaemon,
    #[cfg(not(target_os = "macos"))]
    mdns_fullnames: Vec<String>,
}

impl AtvServer {
    /// Start listeners and advertise the three Bonjour services.
    pub async fn start(config: AtvConfig, delegate: Arc<dyn AtvDelegate>) -> Result<Self> {
        let trackpad_mode = Arc::new(AtomicU8::new(config.trackpad_mode as u8));
        let mouse_mode = Arc::new(AtomicBool::new(config.trackpad_mode.is_mouse()));
        Self::start_with_trackpad_mode(config, delegate, trackpad_mode, mouse_mode).await
    }

    /// Start listeners with a shared atomic mouse_mode boolean for live mode switching (legacy helper).
    pub async fn start_with_mouse_mode(
        config: AtvConfig,
        delegate: Arc<dyn AtvDelegate>,
        mouse_mode: Arc<AtomicBool>,
    ) -> Result<Self> {
        let initial_u8 = if mouse_mode.load(Ordering::SeqCst) {
            TrackpadMode::Mouse as u8
        } else {
            config.trackpad_mode as u8
        };
        let trackpad_mode = Arc::new(AtomicU8::new(initial_u8));
        Self::start_with_trackpad_mode(config, delegate, trackpad_mode, mouse_mode).await
    }

    /// Start listeners with shared atomic mode trackers.
    pub async fn start_with_trackpad_mode(
        config: AtvConfig,
        delegate: Arc<dyn AtvDelegate>,
        trackpad_mode: Arc<AtomicU8>,
        mouse_mode: Arc<AtomicBool>,
    ) -> Result<Self> {
        if config.pin > 9999 {
            return Err(Error::Protocol("pin must be between 0000 and 9999".into()));
        }
        let identity = DeviceIdentity::from_config(&config);
        let mut config = config;
        config.name = identity.name.clone();
        config.device_id = Some(identity.device_id.clone());
        config.server_identifier = Some(identity.server_identifier.clone());
        config.private_key = Some(identity.private_key);

        let ip = match config.ip {
            Some(ip) => ip,
            None => detect_local_ip()?,
        };

        let (mrp_listener, mrp_port) = bind_with_fallback(MRP_PORT, "MRP").await?;
        let (companion_listener, companion_port) =
            bind_with_fallback(COMPANION_PORT, "Companion").await?;
        // Port 7000 is normally occupied when macOS AirPlay Receiver is on;
        // we only need this endpoint for discovery, so let the OS choose.
        let airplay_listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
        let airplay_port = airplay_listener.local_addr()?.port();

        let inspector = InspectorHub::new();

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let mut tasks = Vec::new();

        tasks.push(tokio::spawn(mrp_loop(mrp_listener, shutdown_rx.clone())));
        tasks.push(tokio::spawn(airplay_loop(
            airplay_listener,
            shutdown_rx.clone(),
        )));
        tasks.push(tokio::spawn(companion_loop(
            companion_listener,
            identity.clone(),
            config.pin,
            delegate.clone(),
            inspector.clone(),
            trackpad_mode.clone(),
            mouse_mode.clone(),
            shutdown_rx.clone(),
        )));

        let mut actual_ui_port = None;
        if let Some(ui_p) = config.ui_port {
            // The inspector has no authentication and can press buttons on this computer, so it
            // only answers on this computer (http://127.0.0.1), not to other devices on the network.
            match TcpListener::bind((Ipv4Addr::LOCALHOST, ui_p)).await {
                Ok(ui_listener) => {
                    actual_ui_port = Some(ui_p);
                    tasks.push(tokio::spawn(ui_loop(
                        ui_listener,
                        inspector.clone(),
                        delegate.clone(),
                        trackpad_mode.clone(),
                        mouse_mode.clone(),
                        identity.name.clone(),
                        shutdown_rx.clone(),
                    )));
                }
                Err(e) => {
                    warn!("failed to bind Debug UI on port {ui_p}: {e}");
                }
            }
        }

        let rpha = identity.device_id.replace(':', "").to_uppercase();
        let airplay_id = {
            let mut h = Sha256::new();
            h.update(b"airplay-identity:");
            h.update(identity.server_identifier.as_bytes());
            hex::encode(&h.finalize()[..8]).to_uppercase()
        };
        let rphn = {
            let mut h = Sha256::new();
            h.update(b"rpHN:");
            h.update(identity.server_identifier.as_bytes());
            hex::encode(&h.finalize()[..6]).to_uppercase()
        };
        let rpad = {
            let mut h = Sha256::new();
            h.update(b"rpAD:");
            h.update(identity.server_identifier.as_bytes());
            hex::encode(&h.finalize()[..6]).to_uppercase()
        };
        let rphi = {
            let mut h = Sha256::new();
            h.update(b"rpHI:");
            h.update(identity.server_identifier.as_bytes());
            hex::encode(&h.finalize()[..6]).to_uppercase()
        };
        let pk = {
            let mut h = Sha256::new();
            h.update(b"pk:");
            h.update(identity.server_identifier.as_bytes());
            hex::encode(&h.finalize()[..16])
        };

        let services: Vec<(&str, u16, Vec<(&str, &str)>)> = vec![
            (
                "_mediaremotetv._tcp.local.",
                mrp_port,
                vec![
                    ("Name", identity.name.as_str()),
                    ("UniqueIdentifier", identity.unique_id.as_str()),
                    ("SystemBuildVersion", "22K160"),
                    ("LocalAirPlayReceiverPairingIdentity", airplay_id.as_str()),
                    ("ModelName", "Apple TV"),
                    ("AllowPairing", "YES"),
                ],
            ),
            (
                "_companion-link._tcp.local.",
                companion_port,
                vec![
                    ("rpMac", "1"),
                    ("rpHA", rpha.as_str()),
                    ("rpHN", rphn.as_str()),
                    ("rpVr", SOURCE_VERSION),
                    ("rpMd", DEVICE_MODEL),
                    ("rpFl", "0x36782"),
                    ("rpAD", rpad.as_str()),
                    ("rpHI", rphi.as_str()),
                    ("rpBA", identity.device_id.as_str()),
                ],
            ),
            (
                "_airplay._tcp.local.",
                airplay_port,
                vec![
                    ("deviceid", identity.device_id.as_str()),
                    ("features", "0x5A7FFFF7,0x1E"),
                    ("flags", "0x44"),
                    ("model", DEVICE_MODEL),
                    ("srcvers", SOURCE_VERSION),
                    ("vv", "2"),
                    ("pi", identity.server_identifier.as_str()),
                    ("pk", pk.as_str()),
                    ("name", identity.name.as_str()),
                ],
            ),
        ];

        #[cfg(target_os = "macos")]
        let mut dns_sd_children = Vec::new();
        #[cfg(target_os = "macos")]
        {
            for (service_type, port, properties) in &services {
                let dns_sd_type = service_type
                    .trim_end_matches(".local.")
                    .trim_end_matches(".local");
                let mut cmd = tokio::process::Command::new("/usr/bin/dns-sd");
                cmd.arg("-R")
                    .arg(&identity.name)
                    .arg(dns_sd_type)
                    .arg("local.")
                    .arg(port.to_string());
                for (k, v) in properties {
                    cmd.arg(format!("{k}={v}"));
                }
                cmd.stdout(std::process::Stdio::null());
                cmd.stderr(std::process::Stdio::null());
                match cmd.spawn() {
                    Ok(child) => {
                        dns_sd_children.push(child);
                        info!("registered {}.{service_type} on {ip}:{port} (via macOS mDNSResponder)", identity.name);
                    }
                    Err(e) => {
                        warn!("failed to spawn /usr/bin/dns-sd for {service_type}: {e}");
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        #[cfg(not(target_os = "macos"))]
        let mdns = ServiceDaemon::new().map_err(|e| Error::Mdns(e.to_string()))?;
        #[cfg(not(target_os = "macos"))]
        let mut mdns_fullnames = Vec::new();
        #[cfg(not(target_os = "macos"))]
        {
            let host_name = format!("{}.local.", identity.name.replace(' ', "-"));
            for (service_type, port, properties) in services {
                let info = ServiceInfo::new(
                    service_type,
                    &identity.name,
                    &host_name,
                    ip.to_string(),
                    port,
                    &properties[..],
                )
                .map_err(|e| Error::Mdns(e.to_string()))?;
                mdns_fullnames.push(info.get_fullname().to_string());
                mdns.register(info).map_err(|e| Error::Mdns(e.to_string()))?;
                info!("registered {}.{service_type} on {ip}:{port}", identity.name);
            }
        }

        info!("fake Apple TV started");
        info!("Name      : \"{}\"", identity.name);
        info!("Device ID : {}", identity.device_id);
        info!("Server ID : {}", identity.server_identifier);
        info!("PIN       : {:04}", config.pin);
        info!("MRP       : {ip}:{mrp_port}");
        info!("Companion : {ip}:{companion_port}");
        info!("AirPlay   : {ip}:{airplay_port}");
        if let Some(port) = actual_ui_port {
            info!("Debug UI  : http://127.0.0.1:{port}");
        }

        Ok(Self {
            config,
            identity,
            ip,
            mrp_port,
            companion_port,
            airplay_port,
            ui_port: actual_ui_port,
            inspector,
            mouse_mode,
            trackpad_mode,
            delegate,
            shutdown_tx,
            tasks,
            #[cfg(target_os = "macos")]
            dns_sd_children,
            #[cfg(not(target_os = "macos"))]
            mdns,
            #[cfg(not(target_os = "macos"))]
            mdns_fullnames,
        })
    }

    pub fn config(&self) -> &AtvConfig {
        &self.config
    }

    pub fn identity(&self) -> &DeviceIdentity {
        &self.identity
    }

    /// The LAN address the services advertise.
    pub fn ip(&self) -> Ipv4Addr {
        self.ip
    }

    pub fn mrp_port(&self) -> u16 {
        self.mrp_port
    }

    pub fn companion_port(&self) -> u16 {
        self.companion_port
    }

    pub fn airplay_port(&self) -> u16 {
        self.airplay_port
    }

    pub fn ui_port(&self) -> Option<u16> {
        self.ui_port
    }

    pub fn inspector(&self) -> Arc<InspectorHub> {
        self.inspector.clone()
    }

    pub fn trackpad_mode_arc(&self) -> Arc<AtomicU8> {
        self.trackpad_mode.clone()
    }

    pub fn trackpad_mode(&self) -> TrackpadMode {
        TrackpadMode::from_u8(self.trackpad_mode.load(Ordering::SeqCst))
    }

    pub fn set_trackpad_mode(&self, mode: TrackpadMode) {
        self.trackpad_mode.store(mode as u8, Ordering::SeqCst);
        self.mouse_mode.store(mode.is_mouse(), Ordering::SeqCst);
        self.delegate.on_trackpad_mode_changed(mode);
        self.inspector.emit(
            "mode_changed",
            &format!("{{\"mode\":\"{}\",\"mouse_mode\":{}}}", mode.as_str(), mode.is_mouse()),
        );
        crate::settings::UserSettings::update_trackpad_mode(mode.as_str());
    }

    pub fn cycle_trackpad_mode(&self) -> TrackpadMode {
        let current = self.trackpad_mode();
        let new_mode = current.next();
        self.set_trackpad_mode(new_mode);
        new_mode
    }

    pub fn mouse_mode_arc(&self) -> Arc<AtomicBool> {
        self.mouse_mode.clone()
    }

    pub fn is_mouse_mode(&self) -> bool {
        self.mouse_mode.load(Ordering::SeqCst)
    }

    pub fn is_idle_mode(&self) -> bool {
        self.trackpad_mode.load(Ordering::SeqCst) == TrackpadMode::Idle as u8
    }

    pub fn set_mouse_mode(&self, enabled: bool) {
        self.set_trackpad_mode(if enabled { TrackpadMode::Mouse } else { TrackpadMode::Direction });
    }

    pub fn toggle_mouse_mode(&self) -> bool {
        let new_mode = self.cycle_trackpad_mode();
        new_mode.is_mouse()
    }

    /// Unregister Bonjour services and stop all listeners.
    pub async fn stop(self) {
        let _ = self.shutdown_tx.send(true);

        #[cfg(target_os = "macos")]
        for mut child in self.dns_sd_children {
            let _ = child.kill().await;
        }

        #[cfg(not(target_os = "macos"))]
        {
            for fullname in &self.mdns_fullnames {
                if let Err(e) = self.mdns.unregister(fullname) {
                    warn!("mDNS unregister failed for {fullname}: {e}");
                }
            }
            if let Err(e) = self.mdns.shutdown() {
                warn!("mDNS shutdown failed: {e}");
            }
        }

        for task in &self.tasks {
            task.abort();
        }
        for task in self.tasks {
            let _ = task.await;
        }
    }
}

/// Bind `preferred`, falling back to an OS-assigned port when it is occupied.
/// The chosen port is advertised via Bonjour, so iOS finds the service either way.
async fn bind_with_fallback(preferred: u16, label: &str) -> Result<(TcpListener, u16)> {
    match TcpListener::bind((Ipv4Addr::UNSPECIFIED, preferred)).await {
        Ok(listener) => Ok((listener, preferred)),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
            let port = listener.local_addr()?.port();
            warn!(
                "{label} port {preferred} is already in use ({e}); \
                 falling back to ephemeral port {port}"
            );
            Ok((listener, port))
        }
        Err(e) => Err(e.into()),
    }
}

/// Return the IPv4 address used to reach the local network (UDP-connect
/// trick, like `fake_atv.detect_local_ip`).
pub fn detect_local_ip() -> Result<Ipv4Addr> {
    let sock = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    // A UDP connect sends nothing; it asks the kernel which route it would use.
    if let Ok(()) = sock.connect((Ipv4Addr::new(192, 0, 2, 1), 9)) {
        if let Ok(addr) = sock.local_addr() {
            if let std::net::IpAddr::V4(ip) = addr.ip() {
                if !ip.is_unspecified() && !ip.is_loopback() {
                    return Ok(ip);
                }
            }
        }
    }
    if let Ok(()) = sock.connect((Ipv4Addr::new(8, 8, 8, 8), 53)) {
        if let Ok(addr) = sock.local_addr() {
            if let std::net::IpAddr::V4(ip) = addr.ip() {
                if !ip.is_unspecified() && !ip.is_loopback() {
                    return Ok(ip);
                }
            }
        }
    }
    Err(Error::Protocol("unable to detect local IPv4 address".into()))
}

async fn companion_loop(
    listener: TcpListener,
    identity: DeviceIdentity,
    pin: u32,
    delegate: Arc<dyn AtvDelegate>,
    inspector: Arc<InspectorHub>,
    trackpad_mode: Arc<AtomicU8>,
    mouse_mode: Arc<AtomicBool>,
    mut shutdown: watch::Receiver<bool>,
) {
    let registry = Arc::new(Mutex::new(DeviceRegistry::new(Duration::from_secs(24 * 3600))));
    let active_tracker = Arc::new(Mutex::new(ActiveConnectionTracker::new()));

    loop {
        let stream = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(e) => {
                    warn!("companion accept failed: {e}");
                    continue;
                }
            },
            _ = shutdown.changed() => break,
        };
        let identity = identity.clone();
        let delegate = delegate.clone();
        let inspector = inspector.clone();
        let trackpad_mode = trackpad_mode.clone();
        let mouse_mode = mouse_mode.clone();
        let registry = registry.clone();
        let active_tracker = active_tracker.clone();
        let mut client_shutdown = shutdown.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = handle_companion(stream, identity, pin, delegate, inspector, trackpad_mode, mouse_mode, registry, active_tracker) => {},
                _ = client_shutdown.changed() => {},
            }
        });
    }
}

async fn handle_companion(
    stream: TcpStream,
    identity: DeviceIdentity,
    pin: u32,
    delegate: Arc<dyn AtvDelegate>,
    inspector: Arc<InspectorHub>,
    trackpad_mode: Arc<AtomicU8>,
    mouse_mode: Arc<AtomicBool>,
    registry: Arc<Mutex<DeviceRegistry>>,
    active_tracker: Arc<Mutex<ActiveConnectionTracker>>,
) {
    let peer_addr = stream.peer_addr().ok();
    let peer = peer_addr
        .as_ref()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "?".into());
    let peer_ip = peer_addr
        .map(|a| a.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    info!("companion client connected: {peer}");
    inspector.add_client(&peer);
    delegate.on_event(EventKind::ClientConnected, &peer);
    inspector.emit("client_connected", &format!("{{\"peer\":\"{peer}\"}}"));

    let session_established = Arc::new(AtomicBool::new(false));
    let (conn_id, mut abort_rx) = {
        let mut tracker = active_tracker.lock().await;
        tracker.register(peer_ip, session_established.clone())
    };

    let (mut reader, writer) = stream.into_split();
    let writer = Arc::new(Mutex::new(writer));
    let mut session = CompanionSession::new(
        writer,
        delegate.clone(),
        &identity,
        pin,
        mouse_mode,
        trackpad_mode,
        inspector.clone(),
        registry,
        active_tracker.clone(),
        conn_id,
        peer_ip.to_string(),
        session_established,
    );
    let timeout_deadline = session.timeout_deadline();
    let mut disconnect_reason = "peer_closed".to_string();

    const MAX_FRAME_LENGTH: usize = 2 * 1024 * 1024; // 2MB protection against malformed frames

    loop {
        let deadline = *timeout_deadline.lock().await;
        let timeout_fut = async {
            if let Some(dl) = deadline {
                tokio::time::sleep_until(dl).await;
                true
            } else {
                std::future::pending::<bool>().await
            }
        };

        let mut header = [0u8; 4];
        tokio::select! {
            res = reader.read_exact(&mut header) => {
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::UnexpectedEof {
                            disconnect_reason = format!("error:{e}");
                        }
                        break;
                    }
                }
            }
            _ = abort_rx.changed() => {
                disconnect_reason = "superseded".to_string();
                info!("companion connection from {peer} superseded by new connection");
                break;
            }
            _ = timeout_fut => {
                if !session.is_established() {
                    disconnect_reason = "sessionless_timeout".to_string();
                    info!("companion connection from {peer} timed out without remote session; closing to free channel");
                    break;
                }
            }
        }
        let Some(frame_type) = FrameType::from_u8(header[0]) else {
            disconnect_reason = format!("error:unknown frame type {}", header[0]);
            warn!("unknown frame type {}", header[0]);
            break;
        };
        let length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
        if length > MAX_FRAME_LENGTH {
            disconnect_reason = format!("error:frame too large ({length} bytes)");
            warn!("companion frame exceeds limit: {length} bytes from {peer}");
            break;
        }
        let mut payload = vec![0u8; length];
        if let Err(e) = reader.read_exact(&mut payload).await {
            disconnect_reason = format!("incomplete_frame:{e}");
            break;
        }
        debug!("companion frame={:?} length={length}", frame_type);
        if let Err(e) = session.handle_frame(frame_type, &payload, &header).await {
            disconnect_reason = format!("error:{e}");
            warn!("companion error: {e}");
            break;
        }
    }

    {
        let mut tracker = active_tracker.lock().await;
        tracker.unregister(conn_id);
    }

    info!("companion client disconnected: {peer}");
    inspector.remove_client(&peer);
    delegate.on_event(
        EventKind::ClientDisconnected,
        &format!("{peer} reason={disconnect_reason}"),
    );
    inspector.emit(
        "client_disconnected",
        &format!("{{\"peer\":\"{peer}\",\"reason\":\"{disconnect_reason}\"}}"),
    );
}

const DASHBOARD_HTML: &str = include_str!("dashboard.html");

async fn ui_loop(
    listener: TcpListener,
    inspector: Arc<InspectorHub>,
    delegate: Arc<dyn AtvDelegate>,
    trackpad_mode: Arc<AtomicU8>,
    mouse_mode: Arc<AtomicBool>,
    device_name: String,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        let stream = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(e) => {
                    warn!("UI accept failed: {e}");
                    continue;
                }
            },
            _ = shutdown.changed() => break,
        };
        let inspector = inspector.clone();
        let delegate = delegate.clone();
        let trackpad_mode = trackpad_mode.clone();
        let mouse_mode = mouse_mode.clone();
        let device_name = device_name.clone();
        tokio::spawn(handle_ui_client(stream, inspector, delegate, trackpad_mode, mouse_mode, device_name));
    }
}

async fn handle_ui_client(
    mut stream: TcpStream,
    inspector: Arc<InspectorHub>,
    delegate: Arc<dyn AtvDelegate>,
    trackpad_mode: Arc<AtomicU8>,
    mouse_mode: Arc<AtomicBool>,
    device_name: String,
) {
    use tokio::io::AsyncWriteExt;

    let mut buf = vec![0u8; 4096];
    let n = match stream.read(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let raw_path = parts[1];
    let path = raw_path.split('?').next().unwrap_or(raw_path);

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            let body = DASHBOARD_HTML.as_bytes();
            let header = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/html; charset=utf-8\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes()).await;
            let _ = stream.write_all(body).await;
            let _ = stream.flush().await;
        }
        ("GET", "/debug") | ("GET", "/debug/") | ("GET", "/logs") => {
            let resp = "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:8766/\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes()).await;
        }
        ("GET", "/events") => {
            let header = "HTTP/1.1 200 OK\r\n\
                          Content-Type: text/event-stream\r\n\
                          Cache-Control: no-cache\r\n\
                          Connection: keep-alive\r\n\
                          Access-Control-Allow-Origin: *\r\n\
                          \r\n";
            if stream.write_all(header.as_bytes()).await.is_err() {
                return;
            }
            if stream.flush().await.is_err() {
                return;
            }

            let (history, mut rx) = inspector.subscribe();
            for item in history {
                let msg = format!("data: {item}\n\n");
                if stream.write_all(msg.as_bytes()).await.is_err() {
                    return;
                }
            }
            let _ = stream.flush().await;

            let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Ok(packet) => {
                                let sse = format!("data: {packet}\n\n");
                                if stream.write_all(sse.as_bytes()).await.is_err() {
                                    break;
                                }
                                if stream.flush().await.is_err() {
                                    break;
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(_) => break,
                        }
                    }
                    _ = ping_interval.tick() => {
                        if stream.write_all(b": ping\n\n").await.is_err() {
                            break;
                        }
                        if stream.flush().await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
        ("POST", "/api/action") => {
            if let Some(pos) = req.find("\r\n\r\n") {
                let body = &req[pos + 4..];
                if let Some(button) = extract_button_from_json(body) {
                    if button == "toggle_mode" {
                        let cur_mode = TrackpadMode::from_u8(trackpad_mode.load(Ordering::SeqCst));
                        let new_mode = cur_mode.next();
                        trackpad_mode.store(new_mode as u8, Ordering::SeqCst);
                        mouse_mode.store(new_mode.is_mouse(), Ordering::SeqCst);
                        delegate.on_trackpad_mode_changed(new_mode);
                        inspector.emit(
                            "mode_changed",
                            &format!("{{\"mode\":\"{}\",\"mouse_mode\":{}}}", new_mode.as_str(), new_mode.is_mouse()),
                        );
                        crate::settings::UserSettings::update_trackpad_mode(new_mode.as_str());
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{{\"status\":\"ok\",\"mode\":\"{}\",\"mouse_mode\":{}}}",
                            new_mode.as_str(),
                            new_mode.is_mouse()
                        );
                        let _ = stream.write_all(resp.as_bytes()).await;
                        return;
                    }
                    delegate.on_button(&button);
                    inspector.emit("button", &format!("{{\"name\":\"{button}\"}}"));
                    let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}";
                    let _ = stream.write_all(resp.as_bytes()).await;
                    return;
                }
            }
            let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes()).await;
        }
        ("POST", "/api/mode") => {
            if let Some(pos) = req.find("\r\n\r\n") {
                let body = &req[pos + 4..];
                let cur_mode = TrackpadMode::from_u8(trackpad_mode.load(Ordering::SeqCst));
                let new_mode = if body.contains("\"idle\"") || body.contains("\"preview\"") {
                    TrackpadMode::Idle
                } else if body.contains("\"mouse\"") || body.contains("true") {
                    TrackpadMode::Mouse
                } else if body.contains("\"direction\"") || body.contains("false") {
                    TrackpadMode::Direction
                } else {
                    cur_mode.next()
                };
                trackpad_mode.store(new_mode as u8, Ordering::SeqCst);
                mouse_mode.store(new_mode.is_mouse(), Ordering::SeqCst);
                delegate.on_trackpad_mode_changed(new_mode);
                inspector.emit(
                    "mode_changed",
                    &format!("{{\"mode\":\"{}\",\"mouse_mode\":{}}}", new_mode.as_str(), new_mode.is_mouse()),
                );
                crate::settings::UserSettings::update_trackpad_mode(new_mode.as_str());
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{{\"status\":\"ok\",\"mode\":\"{}\",\"mouse_mode\":{}}}",
                    new_mode.as_str(),
                    new_mode.is_mouse()
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                return;
            }
            let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes()).await;
        }
        ("POST", "/api/toggle_mode") => {
            let cur_mode = TrackpadMode::from_u8(trackpad_mode.load(Ordering::SeqCst));
            let new_mode = cur_mode.next();
            trackpad_mode.store(new_mode as u8, Ordering::SeqCst);
            mouse_mode.store(new_mode.is_mouse(), Ordering::SeqCst);
            delegate.on_trackpad_mode_changed(new_mode);
            inspector.emit(
                "mode_changed",
                &format!("{{\"mode\":\"{}\",\"mouse_mode\":{}}}", new_mode.as_str(), new_mode.is_mouse()),
            );
            crate::settings::UserSettings::update_trackpad_mode(new_mode.as_str());
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{{\"status\":\"ok\",\"mode\":\"{}\",\"mouse_mode\":{}}}",
                new_mode.as_str(),
                new_mode.is_mouse()
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            return;
        }
        ("POST", "/api/touchpad_settings") | ("POST", "/api/mouse_settings") | ("POST", "/api/speed") => {
            if let Some(pos) = req.find("\r\n\r\n") {
                let body = &req[pos + 4..];
                let (cur_speed, cur_accel, cur_verbose) = delegate.get_touchpad_settings();
                let new_speed = extract_float_from_json(body, "speed")
                    .unwrap_or(cur_speed)
                    .clamp(0.1, 10.0);
                let new_accel = extract_bool_from_json(body, "accel").unwrap_or(cur_accel);
                let new_verbose = extract_bool_from_json(body, "verbose_events")
                    .or_else(|| extract_bool_from_json(body, "verbose"))
                    .unwrap_or(cur_verbose);

                let mut cur_mode = TrackpadMode::from_u8(trackpad_mode.load(Ordering::SeqCst));
                if let Some(m_str) = extract_string_from_json(body, "mode")
                    .or_else(|| extract_string_from_json(body, "trackpad_mode"))
                {
                    if let Some(m) = TrackpadMode::from_str_opt(&m_str) {
                        cur_mode = m;
                        trackpad_mode.store(m as u8, Ordering::SeqCst);
                        mouse_mode.store(m.is_mouse(), Ordering::SeqCst);
                        delegate.on_trackpad_mode_changed(m);
                        inspector.emit(
                            "mode_changed",
                            &format!("{{\"mode\":\"{}\",\"mouse_mode\":{}}}", m.as_str(), m.is_mouse()),
                        );
                    }
                } else if let Some(new_mouse) = extract_bool_from_json(body, "mouse_mode") {
                    let m = if new_mouse { TrackpadMode::Mouse } else { TrackpadMode::Direction };
                    cur_mode = m;
                    trackpad_mode.store(m as u8, Ordering::SeqCst);
                    mouse_mode.store(m.is_mouse(), Ordering::SeqCst);
                    delegate.on_trackpad_mode_changed(m);
                    inspector.emit(
                        "mode_changed",
                        &format!("{{\"mode\":\"{}\",\"mouse_mode\":{}}}", m.as_str(), m.is_mouse()),
                    );
                }

                delegate.on_touchpad_settings_changed(new_speed, new_accel, new_verbose);
                crate::settings::UserSettings::update_all_with_mode(
                    new_speed,
                    new_accel,
                    new_verbose,
                    Some(cur_mode.as_str()),
                    Some(cur_mode.is_mouse()),
                );
                inspector.emit(
                    "touchpad_settings",
                    &format!(
                        "{{\"mode\":\"{}\",\"mouse_mode\":{},\"speed\":{new_speed:.2},\"accel\":{new_accel},\"verbose_events\":{new_verbose}}}",
                        cur_mode.as_str(),
                        cur_mode.is_mouse()
                    ),
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{{\"status\":\"ok\",\"mode\":\"{}\",\"mouse_mode\":{},\"speed\":{:.2},\"accel\":{},\"verbose_events\":{}}}",
                    cur_mode.as_str(), cur_mode.is_mouse(), new_speed, new_accel, new_verbose
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                return;
            }
            let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes()).await;
        }
        ("GET", "/api/status") | ("GET", "/api/mode") => {
            let cur_mode = TrackpadMode::from_u8(trackpad_mode.load(Ordering::SeqCst));
            let is_mouse = cur_mode.is_mouse();
            let (speed, accel, verbose_events) = delegate.get_touchpad_settings();
            let screen_json = match delegate.get_screen_size() {
                Some((w, h)) => format!("[{},{}]", w.round() as u32, h.round() as u32),
                None => "null".to_string(),
            };
            let (client_conn, client_peer, session_ready) = inspector.get_client_info();
            let peer_json = match client_peer {
                Some(p) => format!("\"{p}\""),
                None => "null".to_string(),
            };
            let audio_json = match delegate.get_audio_state() {
                Some((vol, muted)) => format!("{{\"volume\":{vol:.2},\"muted\":{muted}}}"),
                None => "null".to_string(),
            };
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{{\"status\":\"ok\",\"server_online\":true,\"mode\":\"{}\",\"mouse_mode\":{},\"device_name\":\"{}\",\"mouse_speed\":{:.2},\"mouse_accel\":{},\"verbose_events\":{},\"screen_size\":{},\"client_connected\":{},\"client_peer\":{},\"session_ready\":{},\"audio\":{}}}",
                cur_mode.as_str(), is_mouse, device_name, speed, accel, verbose_events, screen_json, client_conn, peer_json, session_ready, audio_json
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            return;
        }
        ("GET", "/favicon.ico") => {
            let resp = "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes()).await;
        }
        _ => {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes()).await;
        }
    }
}

fn extract_string_from_json(json: &str, key: &str) -> Option<String> {
    let quoted = format!("\"{key}\"");
    let idx = json.find(&quoted)?;
    let rest = &json[idx + quoted.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_button_from_json(json: &str) -> Option<String> {
    let key = "\"button\"";
    let idx = json.find(key)?;
    let rest = &json[idx + key.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_float_from_json(json: &str, key: &str) -> Option<f64> {
    let quoted = format!("\"{key}\"");
    let idx = json.find(&quoted)?;
    let rest = &json[idx + quoted.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let num_str: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .collect();
    num_str.parse().ok()
}

fn extract_bool_from_json(json: &str, key: &str) -> Option<bool> {
    let quoted = format!("\"{key}\"");
    let idx = json.find(&quoted)?;
    let rest = &json[idx + quoted.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

async fn mrp_loop(listener: TcpListener, mut shutdown: watch::Receiver<bool>) {
    loop {
        let stream = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(e) => {
                    warn!("MRP accept failed: {e}");
                    continue;
                }
            },
            _ = shutdown.changed() => break,
        };
        tokio::spawn(handle_mrp(stream));
    }
}

async fn handle_mrp(mut stream: TcpStream) {
    let peer = peer_string(&stream);
    info!("MRP client connected: {peer}");
    loop {
        let Some(size) = read_varint(&mut stream).await else {
            break;
        };
        let mut body = vec![0u8; size];
        if stream.read_exact(&mut body).await.is_err() {
            break;
        }
        debug!("MRP message size={size}");
    }
    info!("MRP client disconnected: {peer}");
}

async fn read_varint(stream: &mut TcpStream) -> Option<usize> {
    let mut shift = 0u32;
    let mut result: usize = 0;
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).await.ok()?;
        result |= ((byte[0] & 0x7F) as usize) << shift;
        if byte[0] & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift > 63 {
            warn!("varint too long");
            return None;
        }
    }
}

async fn airplay_loop(listener: TcpListener, mut shutdown: watch::Receiver<bool>) {
    loop {
        let stream = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(e) => {
                    warn!("AirPlay accept failed: {e}");
                    continue;
                }
            },
            _ = shutdown.changed() => break,
        };
        tokio::spawn(handle_airplay(stream));
    }
}

async fn handle_airplay(mut stream: TcpStream) {
    let peer = peer_string(&stream);
    info!("AirPlay client connected: {peer}");
    let mut buf = [0u8; 4096];
    let _ = stream.read(&mut buf).await;
    use tokio::io::AsyncWriteExt;
    let _ = stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK")
        .await;
}

fn peer_string(stream: &TcpStream) -> String {
    stream
        .peer_addr()
        .map(|a: SocketAddr| a.to_string())
        .unwrap_or_else(|_| "?".into())
}


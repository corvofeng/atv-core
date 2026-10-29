//! atv-cli: Apple TV Remote simulator and controller for macOS, Windows and Android TV.

// On Windows run as a background tray app (no console window).
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::process::exit;
use std::sync::Arc;

use atv_core::{AtvConfig, AtvDelegate, AtvServer, TrackpadMode};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod windows;
mod android_adb;
mod print;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    Mac,
    Android,
    Print,
}

fn usage() -> ! {
    eprintln!(
        "usage: atv-cli [OPTIONS]\n\
         \n\
         Starts a fake Apple TV server (Bonjour, SRP PIN pairing, Companion Link)\n\
         that accepts control from iPhone's Control Center \"Apple TV Remote\".\n\
         \n\
         Options:\n\
           --target <mac|windows|android|print>  Target to control (default: this computer on macOS/Windows, else print)\n\
           --android-host <IP>            Android TV IP address (required when --target android)\n\
           --android-port <PORT>          Android TV ADB port (default: 5555)\n\
           --name <NAME>                  Device name shown in iOS Remote (default: Computer Name on macOS, else \"Mac Remote\")\n\
           --device-id <MAC>              Hardware / MAC device ID (default: auto-detected hardware MAC)\n\
           --server-id <UUID>             Unique Server UUID (default: auto-detected hardware UUID)\n\
           --pin <PIN>                    4-digit pairing PIN (default: 1111)\n\
           --random-pin                   Print a new random PIN for every pairing attempt instead of --pin\n\
           --require-pairing              Admit only phones that paired with this server; use with --no-ui,\n\
                                          the debug web UI presses buttons without pairing\n\
           --pairing-dir <DIR>            Directory for the paired-phone list (default: memory only)\n\
           --ip <IPV4>                    Local IPv4 to advertise (auto-detected if omitted)\n\
           --mouse                        Initial mode: Trackpad mouse (default on macOS)\n\
           --direction, --dpad            Initial mode: Direction keys (D-pad)\n\
           --idle, --preview              Initial mode: Trackpad idle (preview trajectory only)\n\
           --mode <mouse|direction|idle>  Initial trackpad mode\n\
           --speed <FLOAT>                Initial mouse speed multiplier (default: 0.5)\n\
           --verbose-events               Enable verbose terminal logging for buttons and events (default: quiet debug)\n\
           --debug, -d                    Enable debug-level logging for all protocol packets\n\
           --ui-port <PORT>               Debug Web UI port (default: 8765)\n\
           --no-ui                        Disable Debug Web UI\n\
           --open                         Automatically open Debug UI in default browser (default on Windows)\n\
           --no-open                      Do not open Debug UI automatically\n\
           -h, --help                     Show this help message"
    );
    exit(2);
}

struct CliArgs {
    config: AtvConfig,
    target: Target,
    android_host: Option<String>,
    android_port: u16,
    open_browser: bool,
    verbose_events: bool,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    prompt_accessibility: bool,
}

fn parse_args() -> CliArgs {
    let mut config = AtvConfig::default();
    #[cfg(any(target_os = "macos", windows))]
    let mut target = Target::Mac;
    #[cfg(not(any(target_os = "macos", windows)))]
    let mut target = Target::Print;

    let mut android_host = None;
    let mut android_port = 5555;
    let mut open_browser = cfg!(windows);
    let mut verbose_events = false;
    let mut mouse_mode_explicit = false;
    let mut mouse_speed_explicit = false;
    let mut prompt_accessibility = true;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--target" => {
                let t = args.next().unwrap_or_else(|| usage());
                match t.as_str() {
                    "mac" | "windows" | "win" | "native" => target = Target::Mac,
                    "android" => target = Target::Android,
                    "print" | "observe" => target = Target::Print,
                    _ => usage(),
                }
            }
            "--android-host" => {
                android_host = Some(args.next().unwrap_or_else(|| usage()));
            }
            "--android-port" => {
                android_port = args
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or_else(|| usage());
            }
            "--name" => config.name = args.next().unwrap_or_else(|| usage()),
            "--device-id" => config.device_id = Some(args.next().unwrap_or_else(|| usage())),
            "--server-id" => config.server_identifier = Some(args.next().unwrap_or_else(|| usage())),
            "--pin" => {
                let pin: u32 = args
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or_else(|| usage());
                if pin > 9999 {
                    usage();
                }
                config.pin = pin;
            }
            "--random-pin" => config.random_pin = true,
            "--require-pairing" => config.require_pairing = true,
            "--pairing-dir" => {
                config.pairing_dir = Some(args.next().unwrap_or_else(|| usage()).into());
            }
            "--ip" => {
                config.ip = Some(
                    args.next()
                        .and_then(|ip| ip.parse().ok())
                        .unwrap_or_else(|| usage()),
                );
            }
            "--mouse" => {
                config.mouse_mode = true;
                config.trackpad_mode = TrackpadMode::Mouse;
                mouse_mode_explicit = true;
            }
            "--direction" | "--dpad" => {
                config.mouse_mode = false;
                config.trackpad_mode = TrackpadMode::Direction;
                mouse_mode_explicit = true;
            }
            "--idle" | "--preview" => {
                config.mouse_mode = false;
                config.trackpad_mode = TrackpadMode::Idle;
                mouse_mode_explicit = true;
            }
            "--mode" => {
                let m = args.next().unwrap_or_else(|| usage());
                if let Some(tm) = TrackpadMode::from_str_opt(&m) {
                    config.trackpad_mode = tm;
                    config.mouse_mode = tm.is_mouse();
                    mouse_mode_explicit = true;
                } else {
                    usage();
                }
            }
            "--speed" => {
                let sp: f64 = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| usage());
                config.mouse_speed = sp;
                mouse_speed_explicit = true;
            }
            "--verbose-events" | "--verbose" => verbose_events = true,
            "--debug" | "-d" => {}
            "--no-prompt" => prompt_accessibility = false,
            "--ui-port" => {
                let port: u16 = args
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or_else(|| usage());
                config.ui_port = Some(port);
            }
            "--no-ui" => config.ui_port = None,
            "--open" => open_browser = true,
            "--no-open" => open_browser = false,
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }

    if mouse_speed_explicit {
        atv_core::UserSettings::update_speed(config.mouse_speed);
    }

    if !mouse_mode_explicit {
        #[cfg(any(target_os = "macos", windows))]
        if target == Target::Mac {
            let settings = atv_core::UserSettings::load();
            let initial_mode = settings
                .trackpad_mode
                .as_deref()
                .and_then(TrackpadMode::from_str_opt)
                .unwrap_or_else(|| {
                    if settings.mouse_mode.unwrap_or(true) {
                        TrackpadMode::Mouse
                    } else {
                        TrackpadMode::Direction
                    }
                });
            config.trackpad_mode = initial_mode;
            config.mouse_mode = initial_mode.is_mouse();
        }
    }

    if target == Target::Android && android_host.is_none() {
        eprintln!("Error: --target android requires --android-host <IP>");
        usage();
    }

    CliArgs {
        config,
        target,
        android_host,
        android_port,
        open_browser,
        verbose_events,
        prompt_accessibility,
    }
}

#[tokio::main]
async fn main() {
    let has_debug_flag = std::env::args().any(|a| a == "--debug" || a == "-d");
    let default_filter = if has_debug_flag {
        "atv_core=debug,atv_cli=debug"
    } else {
        "atv_core=info,atv_cli=info"
    };

    use std::io::IsTerminal;
    let is_terminal = std::io::stdout().is_terminal();

    let log = tracing_subscriber::fmt()
        .with_ansi(is_terminal)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        );
    // No console on Windows: write logs to a file next to settings.json.
    #[cfg(windows)]
    {
        if tray::already_running() {
            tray::message_box("Apple TV Remote 已在后台运行，请在任务栏托盘中查看。");
            exit(0);
        }
        let path = tray::log_path();
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        match std::fs::File::create(&path) {
            Ok(f) => log.with_ansi(false).with_writer(std::sync::Mutex::new(f)).init(),
            Err(_) => log.init(),
        }
    }
    #[cfg(not(windows))]
    log.init();

    let args = parse_args();
    let trackpad_mode = Arc::new(std::sync::atomic::AtomicU8::new(args.config.trackpad_mode as u8));
    let mouse_mode = Arc::new(std::sync::atomic::AtomicBool::new(args.config.mouse_mode));
    let open_browser = args.open_browser;

    let delegate: Arc<dyn AtvDelegate> = match args.target {
        Target::Mac => {
            #[cfg(target_os = "macos")]
            {
                let settings = atv_core::UserSettings::load();
                let accel = settings.mouse_accel.unwrap_or(true);
                let verbose = if args.verbose_events {
                    true
                } else {
                    settings.verbose_events.unwrap_or(false)
                };
                Arc::new(macos::MacDelegate::new_with_full_settings(
                    mouse_mode.clone(),
                    args.config.mouse_speed,
                    accel,
                    verbose,
                    args.prompt_accessibility,
                ))
            }
            #[cfg(windows)]
            {
                let settings = atv_core::UserSettings::load();
                Arc::new(windows::WinDelegate::new(
                    mouse_mode.clone(),
                    args.config.mouse_speed,
                    settings.mouse_accel.unwrap_or(true),
                    args.verbose_events || settings.verbose_events.unwrap_or(false),
                ))
            }
            #[cfg(not(any(target_os = "macos", windows)))]
            {
                eprintln!("Warning: Target 'mac' is only supported on macOS/Windows; falling back to print");
                Arc::new(print::PrintDelegate)
            }
        }
        Target::Android => {
            let host = args.android_host.expect("checked above");
            println!("Target      : Android TV at {host}:{}", args.android_port);
            Arc::new(android_adb::AndroidAdbDelegate::new(
                host,
                args.android_port,
                mouse_mode.load(std::sync::atomic::Ordering::SeqCst),
            ))
        }
        Target::Print => {
            println!("Target      : observe-only (printing events to stdout)");
            Arc::new(print::PrintDelegate)
        }
    };

    let server = match AtvServer::start_with_trackpad_mode(args.config.clone(), delegate.clone(), trackpad_mode.clone(), mouse_mode.clone()).await {
        Ok(server) => server,
        Err(e) => {
            eprintln!("failed to start server: {e}");
            #[cfg(windows)]
            tray::message_box(&format!("启动失败 (failed to start server): {e}"));
            exit(1);
        }
    };

    let identity = server.identity();
    println!("Device Name : \"{}\"", identity.name);
    println!("Device ID   : {}", identity.device_id);
    println!("Server UUID : {}", identity.server_identifier);
    if server.config().random_pin {
        println!("PIN Code    : random, logged when a phone starts pairing");
    } else {
        println!("PIN Code    : {:04}", server.config().pin);
    }

    let (init_speed, init_accel, init_verbose) = delegate.get_touchpad_settings();
    let initial_mode_str = match TrackpadMode::from_u8(trackpad_mode.load(std::sync::atomic::Ordering::SeqCst)) {
        TrackpadMode::Mouse => "🖱️  Mouse Cursor (鼠标光标模式)",
        TrackpadMode::Direction => "◀▲▼▶ Direction Keys (上下左右模式)",
        TrackpadMode::Idle => "👁️  Idle Preview (触控板空置/仅看轨迹实时预览)",
    };
    println!("Input Mode  : {initial_mode_str}");
    println!(
        "Touchpad    : Speed={:.2}x, Accel={}, VerboseEvents={}",
        init_speed,
        if init_accel { "Enabled" } else { "Disabled" },
        if init_verbose { "Enabled (Info)" } else { "Quiet (Debug)" }
    );
    println!("Interactive : 'm' toggle mode, 's <speed>' adjust speed, 'v' toggle verbose events");

    // Spawn terminal input listener for dynamic mode, speed, and verbose logging switching
    let trackpad_mode_terminal = trackpad_mode.clone();
    let mouse_mode_terminal = mouse_mode.clone();
    let delegate_terminal = delegate.clone();
    let inspector_terminal = server.inspector();
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let trimmed = line.trim();
            if trimmed.eq_ignore_ascii_case("m") || trimmed.eq_ignore_ascii_case("mode") {
                let current_mode = TrackpadMode::from_u8(trackpad_mode_terminal.load(std::sync::atomic::Ordering::SeqCst));
                let new_mode = current_mode.next();
                trackpad_mode_terminal.store(new_mode as u8, std::sync::atomic::Ordering::SeqCst);
                mouse_mode_terminal.store(new_mode.is_mouse(), std::sync::atomic::Ordering::SeqCst);
                delegate_terminal.on_trackpad_mode_changed(new_mode);
                inspector_terminal.emit(
                    "mode_changed",
                    &format!("{{\"mode\":\"{}\",\"mouse_mode\":{}}}", new_mode.as_str(), new_mode.is_mouse()),
                );
                atv_core::UserSettings::update_trackpad_mode(new_mode.as_str());
                let desc = match new_mode {
                    TrackpadMode::Mouse => "🖱️  Mouse Cursor (鼠标光标模式)",
                    TrackpadMode::Direction => "◀▲▼▶ Direction Keys (上下左右模式)",
                    TrackpadMode::Idle => "👁️  Idle / Preview Only (触控板空置/仅看轨迹实时预览)",
                };
                println!("\n🔄 [模式切换] 当前已切换为: {desc}\n");
            } else if trimmed.starts_with("s ") || trimmed.starts_with("speed ") {
                let val_str = trimmed.split_whitespace().nth(1).unwrap_or("");
                if let Ok(val) = val_str.parse::<f64>() {
                    let (_, accel, verbose) = delegate_terminal.get_touchpad_settings();
                    let new_speed = val.clamp(0.1, 10.0);
                    delegate_terminal.on_touchpad_settings_changed(new_speed, accel, verbose);
                    let cur_mode = TrackpadMode::from_u8(trackpad_mode_terminal.load(std::sync::atomic::Ordering::SeqCst));
                    inspector_terminal.emit(
                        "touchpad_settings",
                        &format!(
                            "{{\"mode\":\"{}\",\"mouse_mode\":{},\"speed\":{new_speed:.2},\"accel\":{accel},\"verbose_events\":{verbose}}}",
                            cur_mode.as_str(),
                            cur_mode.is_mouse()
                        ),
                    );
                    println!("\n⚡ [驱动灵敏度] 鼠标速度已调整为: {new_speed:.2}x\n");
                } else {
                    println!("\n⚠️ 用法: s <速度数值>，例如: s 1.5\n");
                }
            } else if trimmed.eq_ignore_ascii_case("v") || trimmed.eq_ignore_ascii_case("verbose") {
                let (speed, accel, cur_verbose) = delegate_terminal.get_touchpad_settings();
                let new_verbose = !cur_verbose;
                delegate_terminal.on_touchpad_settings_changed(speed, accel, new_verbose);
                let cur_mode = TrackpadMode::from_u8(trackpad_mode_terminal.load(std::sync::atomic::Ordering::SeqCst));
                inspector_terminal.emit(
                    "touchpad_settings",
                    &format!(
                        "{{\"mode\":\"{}\",\"mouse_mode\":{},\"speed\":{speed:.2},\"accel\":{accel},\"verbose_events\":{new_verbose}}}",
                        cur_mode.as_str(),
                        cur_mode.is_mouse()
                    ),
                );
                let desc = if new_verbose { "开启 (Info 级别)" } else { "静默 (Debug 级别)" };
                println!("\n📢 [事件日志输出] 已切换为: {desc}\n");
            }
        }
    });

    if let Some(port) = server.ui_port() {
        let url = format!("http://127.0.0.1:{port}");
        println!("Debug UI    : {url}");
        if open_browser {
            #[cfg(target_os = "macos")]
            let _ = tokio::process::Command::new("/usr/bin/open")
                .arg(&url)
                .spawn();
            #[cfg(windows)]
            tray::open_url(&url);
            #[cfg(not(any(target_os = "macos", windows)))]
            let _ = tokio::process::Command::new("xdg-open")
                .arg(&url)
                .spawn();
        }
    }

    println!("atv-cli ready (press Ctrl+C to stop)");

    #[cfg(windows)]
    {
        let is_running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let title = format!("Apple TV Remote · {} · PIN {:04}", server.identity().name, server.config().pin);
        let url = server.ui_port().map(|p| format!("http://127.0.0.1:{p}"));
        let mut rx = tray::spawn(title, url, trackpad_mode.clone(), is_running.clone());

        let mut current_server: Option<AtvServer> = Some(server);
        let config = args.config;
        let delegate = delegate.clone();

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                cmd = rx.recv() => match cmd {
                    Some(tray::TrayCmd::Mode(m)) => {
                        if let Some(srv) = &current_server {
                            srv.set_trackpad_mode(m);
                        }
                    }
                    Some(tray::TrayCmd::ToggleService) => {
                        if let Some(srv) = current_server.take() {
                            println!("Stopping simulator service via tray...");
                            srv.stop().await;
                            is_running.store(false, std::sync::atomic::Ordering::SeqCst);
                            tray::update_status(false, "");
                            println!("Simulator service stopped.");
                        } else {
                            println!("Starting simulator service via tray...");
                            match AtvServer::start_with_trackpad_mode(
                                config.clone(),
                                delegate.clone(),
                                trackpad_mode.clone(),
                                mouse_mode.clone(),
                            ).await {
                                Ok(srv) => {
                                    println!("Simulator service started.");
                                    let info = format!("{} · PIN {:04}", srv.identity().name, srv.config().pin);
                                    tray::update_status(true, &info);
                                    current_server = Some(srv);
                                    is_running.store(true, std::sync::atomic::Ordering::SeqCst);
                                }
                                Err(e) => {
                                    eprintln!("Failed to restart server: {e}");
                                    tray::message_box(&format!("启动服务失败: {e}"));
                                }
                            }
                        }
                    }
                    Some(tray::TrayCmd::Quit) | None => break,
                },
            }
        }
        if let Some(srv) = current_server {
            srv.stop().await;
        }
        tray::remove();
    }
    #[cfg(not(windows))]
    {
        tokio::signal::ctrl_c().await.ok();
        println!("shutting down...");
        server.stop().await;
    }
}

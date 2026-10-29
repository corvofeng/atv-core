//! JNI bindings for Android TV: runs AtvServer and dispatches callbacks into Kotlin/Java.

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jint, JNI_FALSE, JNI_TRUE};
use jni::{JNIEnv, JavaVM};
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;
use tracing::{error, info, warn};

use atv_core::{AtvConfig, AtvDelegate, AtvServer, EventKind, TextOp, TouchPhase, TrackpadMode};

struct JniDelegate {
    jvm: JavaVM,
    callback: GlobalRef,
}

impl JniDelegate {
    fn new(jvm: JavaVM, callback: GlobalRef) -> Self {
        Self { jvm, callback }
    }

    fn call_void_string(&self, method: &str, arg: &str) {
        if let Ok(mut env) = self.jvm.attach_current_thread() {
            let jstr = match env.new_string(arg) {
                Ok(s) => s,
                Err(e) => {
                    warn!("failed to create JString: {e}");
                    return;
                }
            };
            let obj = self.callback.as_obj();
            let _ = env.call_method(
                obj,
                method,
                "(Ljava/lang/String;)V",
                &[JValue::Object(&jstr)],
            );
        }
    }
}

impl AtvDelegate for JniDelegate {
    fn on_button(&self, name: &str) {
        info!("[Android JNI] on_button: {name}");
        self.call_void_string("onButton", name);
    }

    fn on_touch(&self, dx: f64, dy: f64, phase: TouchPhase) {
        if let Ok(mut env) = self.jvm.attach_current_thread() {
            let phase_int = match phase {
                TouchPhase::Began => 1,
                TouchPhase::Moved => 2,
                TouchPhase::Ended => 4,
            };
            let obj = self.callback.as_obj();
            let _ = env.call_method(
                obj,
                "onTouch",
                "(DDI)V",
                &[
                    JValue::Double(dx),
                    JValue::Double(dy),
                    JValue::Int(phase_int),
                ],
            );
        }
    }

    fn on_audio(&self, volume: f64, muted: bool) {
        if let Ok(mut env) = self.jvm.attach_current_thread() {
            let obj = self.callback.as_obj();
            let _ = env.call_method(
                obj,
                "onAudio",
                "(DZ)V",
                &[JValue::Double(volume), JValue::Bool(muted as u8)],
            );
        }
    }

    fn on_text(&self, op: TextOp) {
        // Kotlin side: onText(kind, text, count) with kind "insert" | "delete" | "replace".
        let (kind, text, count) = match &op {
            TextOp::Insert(text) => ("insert", text.as_str(), 0),
            TextOp::Delete(count) => ("delete", "", i32::try_from(*count).unwrap_or(i32::MAX)),
            TextOp::Replace(text) => ("replace", text.as_str(), 0),
        };
        if let Ok(mut env) = self.jvm.attach_current_thread() {
            let (Ok(kind_str), Ok(text_str)) = (env.new_string(kind), env.new_string(text)) else {
                warn!("failed to create JString for on_text");
                return;
            };
            let obj = self.callback.as_obj();
            let _ = env.call_method(
                obj,
                "onText",
                "(Ljava/lang/String;Ljava/lang/String;I)V",
                &[
                    JValue::Object(&kind_str),
                    JValue::Object(&text_str),
                    JValue::Int(count),
                ],
            );
        }
    }

    fn on_event(&self, kind: EventKind, detail: &str) {
        if let Ok(mut env) = self.jvm.attach_current_thread() {
            let kind_str = match env.new_string(format!("{kind:?}")) {
                Ok(s) => s,
                Err(_) => return,
            };
            let detail_str = match env.new_string(detail) {
                Ok(s) => s,
                Err(_) => return,
            };
            let obj = self.callback.as_obj();
            let _ = env.call_method(
                obj,
                "onEvent",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[JValue::Object(&kind_str), JValue::Object(&detail_str)],
            );
        }
    }

    fn on_trackpad_mode_changed(&self, mode: TrackpadMode) {
        info!("[Android JNI] trackpad mode: {}", mode.as_str());
        if let Ok(mut env) = self.jvm.attach_current_thread() {
            let obj = self.callback.as_obj();
            let _ = env.call_method(
                obj,
                "onTrackpadModeChanged",
                "(I)V",
                &[JValue::Int(mode as u8 as jint)],
            );
        }
    }

    fn show_pairing_pin(&self, pin: &str) {
        self.call_void_string("onShowPin", pin);
    }

    fn hide_pairing_pin(&self) {
        if let Ok(mut env) = self.jvm.attach_current_thread() {
            let _ = env.call_method(self.callback.as_obj(), "onHidePin", "()V", &[]);
        }
    }
}

struct RunningServer {
    runtime: Runtime,
    server: Option<AtvServer>,
}

static SERVER_STATE: Lazy<Mutex<Option<RunningServer>>> = Lazy::new(|| Mutex::new(None));
static IS_RUNNING: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "system" fn Java_com_corvofeng_fakeatv_AtvNative_nativeStartServer(
    mut env: JNIEnv,
    _class: JClass,
    name: JString,
    pin: jint,
    device_id: JString,
    server_id: JString,
    local_ip: JString,
    files_dir: JString,
    callback: JObject,
) -> jboolean {
    let mut state_guard = match SERVER_STATE.lock() {
        Ok(guard) => guard,
        Err(_) => return JNI_FALSE,
    };

    if state_guard.is_some() {
        warn!("server is already running");
        return JNI_TRUE;
    }

    let device_name: String = match env.get_string(&name) {
        Ok(s) => s.into(),
        Err(_) => "Android TV".to_string(),
    };

    let device_id_str: Option<String> = env
        .get_string(&device_id)
        .ok()
        .map(|s| s.into())
        .filter(|s: &String| !s.is_empty());

    let server_id_str: Option<String> = env
        .get_string(&server_id)
        .ok()
        .map(|s| s.into())
        .filter(|s: &String| !s.is_empty());

    let local_ip_str: Option<String> = env
        .get_string(&local_ip)
        .ok()
        .map(|s| s.into())
        .filter(|s: &String| !s.is_empty());

    let files_dir_str: Option<String> = env
        .get_string(&files_dir)
        .ok()
        .map(|s| s.into())
        .filter(|s: &String| !s.is_empty());

    let jvm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(e) => {
            error!("failed to get JavaVM: {e}");
            return JNI_FALSE;
        }
    };

    let global_callback = match env.new_global_ref(callback) {
        Ok(g) => g,
        Err(e) => {
            error!("failed to create GlobalRef for callback: {e}");
            return JNI_FALSE;
        }
    };

    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("failed to create Tokio runtime: {e}");
            return JNI_FALSE;
        }
    };

    let delegate = Arc::new(JniDelegate::new(jvm, global_callback));
    let mut config = AtvConfig::default();
    config.name = device_name;
    config.pin = pin as u32;
    config.device_id = device_id_str;
    config.server_identifier = server_id_str;
    // Derive the long-term key from the server identifier the app persists. The default key
    // comes from a UUID file under $HOME, which an Android app process does not have, so it was
    // new on every start and the iPhone's stored pairing stopped verifying after a reboot.
    config.private_key = None;
    // The TV shows a new code for every pairing attempt, and only phones that
    // paired (kept in the app's files dir) may control it.
    config.random_pin = true;
    config.require_pairing = true;
    config.pairing_dir = files_dir_str.map(PathBuf::from);
    // The debug web UI listens on every address and its /api/action presses buttons without
    // pairing; the app does not use it, so it stays off on Android.
    config.ui_port = None;
    if let Some(ref s) = local_ip_str {
        if let Ok(ip) = s.parse::<Ipv4Addr>() {
            info!("using explicit local IP from Android: {ip}");
            config.ip = Some(ip);
        }
    }

    let server_res = runtime.block_on(async {
        AtvServer::start(config, delegate).await
    });

    match server_res {
        Ok(server) => {
            *state_guard = Some(RunningServer {
                runtime,
                server: Some(server),
            });
            IS_RUNNING.store(true, Ordering::SeqCst);
            info!("AtvServer successfully started on Android TV");
            JNI_TRUE
        }
        Err(e) => {
            error!("failed to start AtvServer: {e}");
            JNI_FALSE
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_corvofeng_fakeatv_AtvNative_nativeStopServer(
    _env: JNIEnv,
    _class: JClass,
) {
    let mut state_guard = match SERVER_STATE.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    if let Some(mut running) = state_guard.take() {
        if let Some(server) = running.server.take() {
            running.runtime.block_on(async {
                server.stop().await;
            });
        }
        IS_RUNNING.store(false, Ordering::SeqCst);
        info!("AtvServer stopped on Android TV");
    }
}

/// Sets the trackpad mode (0 direction, 1 mouse, 2 idle) of the running server; false when no
/// server is running. The server calls `onTrackpadModeChanged` back on this thread.
#[no_mangle]
pub extern "system" fn Java_com_corvofeng_fakeatv_AtvNative_nativeSetTrackpadMode(
    _env: JNIEnv,
    _class: JClass,
    mode: jint,
) -> jboolean {
    // try_lock: this is called from button callbacks, and nativeStopServer holds the lock while
    // it drops the runtime, which waits for the very worker thread that is making this call.
    let state_guard = match SERVER_STATE.try_lock() {
        Ok(guard) => guard,
        Err(_) => return JNI_FALSE,
    };
    match state_guard.as_ref().and_then(|running| running.server.as_ref()) {
        Some(server) => {
            server.set_trackpad_mode(TrackpadMode::from_u8(mode as u8));
            JNI_TRUE
        }
        None => JNI_FALSE,
    }
}

/// Returns false when no server is running, so the caller reports the state again later.
#[no_mangle]
pub extern "system" fn Java_com_corvofeng_fakeatv_AtvNative_nativeSetTextFocus(
    mut env: JNIEnv,
    _class: JClass,
    focused: jboolean,
    field: JString,
    text: JString,
    from_phone: jboolean,
) -> jboolean {
    let field: String = env.get_string(&field).map(Into::into).unwrap_or_default();
    let text: String = env.get_string(&text).map(Into::into).unwrap_or_default();
    let state_guard = match SERVER_STATE.lock() {
        Ok(guard) => guard,
        Err(_) => return JNI_FALSE,
    };
    match state_guard.as_ref().and_then(|running| running.server.as_ref()) {
        Some(server) => {
            server.set_text_focus(focused != JNI_FALSE, &field, &text, from_phone != JNI_FALSE);
            JNI_TRUE
        }
        None => JNI_FALSE,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_corvofeng_fakeatv_AtvNative_nativeIsRunning(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    if IS_RUNNING.load(Ordering::SeqCst) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

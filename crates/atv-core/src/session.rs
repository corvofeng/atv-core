//! Companion Link session: pairing, verification and the encrypted control
//! channel. Ported from `fake_atv.py`'s `CompanionSession`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use ed25519_dalek::{Signer, SigningKey};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::crypto::{hkdf_expand, CompanionCipher};
use crate::delegate::{AtvDelegate, EventKind, TouchPhase, TrackpadMode};
use crate::error::{Error, Result};
use crate::identity::DeviceIdentity;
use crate::inspector::InspectorHub;
use crate::opack::{self, Value};
use crate::pairing::{self, PairingPolicy};
use crate::registry::{ActiveConnectionTracker, DeviceRegistry};
use crate::srp::SrpServer;
use crate::tlv::{self, tags};
use crate::FrameType;

pub const SERVER_IDENTIFIER: &str = "5D797FD3-3538-427E-A47B-A32FC6CF3A6A";
pub const SERVER_PRIVATE_KEY: [u8; 32] = [0xaa; 32];
pub const DEVICE_MODEL: &str = "AppleTV14,1";
pub const MEDIA_CONTROL_VOLUME: u64 = 0x0100;

const SWIPE_MIN_DISTANCE: f64 = 150.0; // touchpad units; the surface is 1000x1000
const TAP_MAX_DISTANCE: f64 = 60.0; // below this a touch sequence counts as a tap

/// Fixed `altIRK` value from `fake_atv._pair_setup_m5`.
const ALT_IRK: [u8; 16] = [
    0x2d, 0x54, 0xe0, 0x7a, 0x88, 0x2a, 0x65, 0x6e, 0x11, 0xab, 0x82, 0x76, 0x2d, 0x27, 0x25,
    0xc5,
];

pub type SharedWriter = Arc<Mutex<OwnedWriteHalf>>;

/// Keys of a pair-verify between M1 and M3; the control channel is enabled
/// only once M3 is accepted.
struct PendingVerify {
    session_key: [u8; 32],
    client_pub: [u8; 32],
    server_pub: [u8; 32],
    output_key: [u8; 32],
    input_key: [u8; 32],
}

fn hid_command_name(command: u64) -> String {
    match command {
        1 => "up".into(),
        2 => "down".into(),
        3 => "left".into(),
        4 => "right".into(),
        5 => "menu".into(),
        6 => "select".into(),
        7 => "home".into(),
        8 => "volume_up".into(),
        9 => "volume_down".into(),
        10 => "siri".into(),
        11 => "screensaver".into(),
        12 => "sleep".into(),
        13 => "wake".into(),
        14 => "play_pause".into(),
        15 => "channel_up".into(),
        16 => "channel_down".into(),
        17 => "guide".into(),
        // iOS 26/27 repurposes the old PageUp/PageDown wire codes for the
        // Control Center remote's Mute and Power buttons.
        18 => "mute".into(),
        19 => "power".into(),
        other => format!("unknown:{other}"),
    }
}

/// One Companion connection's state machine.
pub struct CompanionSession {
    writer: SharedWriter,
    delegate: Arc<dyn AtvDelegate>,
    device_name: String,
    device_mac: [u8; 6],
    pin: u32,
    pairing: PairingPolicy,
    /// Random PIN of the pair-setup attempt in progress; Some while shown.
    active_pin: Option<u32>,
    pending_verify: Option<PendingVerify>,
    _mouse_mode: Arc<AtomicBool>,
    trackpad_mode: Arc<AtomicU8>,
    unique_id: Vec<u8>,
    private_key: [u8; 32],
    signing_key: SigningKey,
    auth_pub: [u8; 32],
    verify_private: StaticSecret,
    verify_pub: X25519PublicKey,
    cipher: Option<CompanionCipher>,
    current_identifier: Option<String>,
    srp: SrpServer,
    remote_session_id: u32,
    session_established: Arc<AtomicBool>,
    touch_origin: Option<(f64, f64)>,
    touch_last: Option<(f64, f64)>,
    touch_moved: bool,
    volume: f64,
    muted: bool,
    system_status: u64,
    display_power_on: bool,
    inspector: Arc<InspectorHub>,
    registry: Arc<Mutex<DeviceRegistry>>,
    active_tracker: Arc<Mutex<ActiveConnectionTracker>>,
    conn_id: u64,
    peer_ip: String,
    device_id: Option<String>,
    timeout_deadline: Arc<Mutex<Option<tokio::time::Instant>>>,
}

impl CompanionSession {
    pub fn new(
        writer: SharedWriter,
        delegate: Arc<dyn AtvDelegate>,
        identity: &DeviceIdentity,
        pin: u32,
        pairing: PairingPolicy,
        mouse_mode: Arc<AtomicBool>,
        trackpad_mode: Arc<AtomicU8>,
        inspector: Arc<InspectorHub>,
        registry: Arc<Mutex<DeviceRegistry>>,
        active_tracker: Arc<Mutex<ActiveConnectionTracker>>,
        conn_id: u64,
        peer_ip: String,
        session_established: Arc<AtomicBool>,
    ) -> Self {
        let signing_key = SigningKey::from_bytes(&identity.private_key);
        let auth_pub = signing_key.verifying_key().to_bytes();
        let verify_private = StaticSecret::from(identity.private_key);
        let verify_pub = X25519PublicKey::from(&verify_private);
        let remote_session_id: u32 = rand::random();
        let (initial_volume, initial_muted) = delegate.get_audio_state().unwrap_or((0.5, false));
        Self {
            writer,
            delegate,
            device_name: identity.name.clone(),
            device_mac: identity.mac_bytes,
            pin,
            pairing,
            active_pin: None,
            pending_verify: None,
            _mouse_mode: mouse_mode,
            trackpad_mode,
            unique_id: identity.server_identifier.as_bytes().to_vec(),
            private_key: identity.private_key,
            signing_key,
            auth_pub,
            verify_private,
            verify_pub,
            cipher: None,
            current_identifier: None,
            srp: SrpServer::new(pin, &identity.private_key),
            remote_session_id,
            session_established,
            touch_origin: None,
            touch_last: None,
            touch_moved: false,
            volume: initial_volume,
            muted: initial_muted,
            system_status: 3, // Awake
            display_power_on: true,
            inspector,
            registry,
            active_tracker,
            conn_id,
            peer_ip,
            device_id: None,
            timeout_deadline: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_established(&self) -> bool {
        self.session_established.load(Ordering::SeqCst)
    }

    pub fn timeout_deadline(&self) -> Arc<Mutex<Option<tokio::time::Instant>>> {
        self.timeout_deadline.clone()
    }

    pub fn device_id(&self) -> Option<String> {
        self.device_id.clone()
    }

    async fn write_raw(&self, bytes: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock().await;
        writer.write_all(bytes).await?;
        writer.flush().await?;
        Ok(())
    }

    async fn send_frame(&self, frame_type: FrameType, payload: &[u8]) -> Result<()> {
        let mut out = Vec::with_capacity(4 + payload.len());
        out.push(frame_type as u8);
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes()[1..]);
        out.extend_from_slice(payload);
        self.write_raw(&out).await
    }

    async fn send_opack(&self, frame_type: FrameType, data: &Value) -> Result<()> {
        let payload = opack::pack(data);
        debug!(">> {:?} {}", frame_type, data);
        self.send_frame(frame_type, &payload).await
    }

    async fn send_encrypted_opack(&mut self, data: &Value) -> Result<()> {
        let cipher = self
            .cipher
            .as_mut()
            .ok_or_else(|| Error::Protocol("encryption is not enabled".into()))?;
        let plain = opack::pack(data);
        let mut header = Vec::with_capacity(4);
        header.push(FrameType::EOpack as u8);
        header.extend_from_slice(&((plain.len() + 16) as u32).to_be_bytes()[1..]);
        let encrypted = cipher.encrypt(&plain, Some(&header), None)?;
        debug!(">> E_OPACK {}", data);
        let mut out = header;
        out.extend_from_slice(&encrypted);
        self.write_raw(&out).await
    }

    async fn send_response(
        &mut self,
        xid: Option<u64>,
        content: Option<Value>,
        result_type: u64,
    ) -> Result<()> {
        let mut response = vec![
            Value::kv("_c", content.unwrap_or(Value::dict(vec![]))),
            Value::kv("_rT", Value::Int(result_type)),
            Value::kv("_t", Value::Int(3)),
        ];
        if let Some(identifier) = &self.current_identifier {
            response.push(Value::kv("_i", Value::str(identifier)));
        }
        if let Some(xid) = xid {
            response.push(Value::kv("_x", Value::Int(xid)));
        }
        self.send_encrypted_opack(&Value::Dict(response)).await
    }

    async fn send_event(
        &mut self,
        identifier: &str,
        xid: Option<u64>,
        content: Option<Value>,
    ) -> Result<()> {
        let mut event = vec![
            Value::kv("_i", Value::str(identifier)),
            Value::kv("_t", Value::Int(1)),
            Value::kv("_c", content.unwrap_or(Value::dict(vec![]))),
        ];
        if let Some(xid) = xid {
            event.push(Value::kv("_x", Value::Int(xid)));
        }
        self.send_encrypted_opack(&Value::Dict(event)).await
    }

    /// Advertise volume support using both modern and legacy wire formats.
    async fn send_media_capabilities(&mut self, xid: Option<u64>) -> Result<()> {
        self.send_event(
            "MediaControlStatus",
            xid,
            Some(Value::dict(vec![Value::kv(
                "MediaControlFlags",
                Value::Int(MEDIA_CONTROL_VOLUME),
            )])),
        )
        .await?;
        self.send_event(
            "_iMC",
            xid,
            Some(Value::dict(vec![Value::kv(
                "_mcF",
                Value::Int(MEDIA_CONTROL_VOLUME),
            )])),
        )
        .await?;
        self.delegate.on_event(EventKind::Capabilities, "volume,mute,power");
        Ok(())
    }

    /// Re-publish the volume capability flag after a state change so iOS
    /// refreshes its volume HUD (it issues GetVolume in response).
    async fn push_media_state(&mut self, xid: Option<u64>) -> Result<()> {
        self.send_event(
            "MediaControlStatus",
            xid,
            Some(Value::dict(vec![Value::kv(
                "MediaControlFlags",
                Value::Int(MEDIA_CONTROL_VOLUME),
            )])),
        )
        .await?;
        self.send_event(
            "_iMC",
            xid,
            Some(Value::dict(vec![Value::kv(
                "_mcF",
                Value::Int(MEDIA_CONTROL_VOLUME),
            )])),
        )
        .await
    }

    /// Publish current power state using both known Companion event names.
    async fn send_power_state(&mut self, xid: Option<u64>) -> Result<()> {
        let content = Value::dict(vec![Value::kv("state", Value::Int(self.system_status))]);
        self.send_event("SystemStatus", xid, Some(content.clone()))
            .await?;
        self.send_event("TVSystemStatus", xid, Some(content)).await?;
        Ok(())
    }

    pub async fn handle_frame(
        &mut self,
        frame_type: FrameType,
        payload: &[u8],
        header: &[u8],
    ) -> Result<()> {
        let mut payload = payload.to_vec();
        if frame_type == FrameType::EOpack {
            match self.cipher.as_mut() {
                Some(cipher) => {
                    payload = cipher.decrypt(&payload, Some(header), None)?;
                }
                None => {
                    warn!("encrypted frame before verification");
                    return Ok(());
                }
            }
        }

        if payload.is_empty() {
            return Ok(());
        }

        let (data, remaining) = opack::unpack(&payload)?;
        if !remaining.is_empty() {
            warn!("trailing OPACK bytes: {:02x?}", remaining);
        }

        match frame_type {
            FrameType::PsStart | FrameType::PsNext | FrameType::PvStart | FrameType::PvNext => {
                self.handle_auth(frame_type, &data).await
            }
            FrameType::EOpack => {
                debug!("<< E_OPACK {}", data);
                self.handle_control(&data).await
            }
            other => {
                debug!("{:?} {}", other, data);
                Ok(())
            }
        }
    }

    async fn handle_auth(&mut self, frame_type: FrameType, data: &Value) -> Result<()> {
        let pairing_data = tlv::read(data.get("_pd").and_then(Value::as_bytes).unwrap_or(b""));
        let seq = pairing_data
            .get(&tags::SEQ_NO)
            .map(|v| {
                v.iter()
                    .enumerate()
                    .fold(0u64, |acc, (i, b)| acc | ((*b as u64) << (8 * i)))
            })
            .unwrap_or(0);
        info!("<< {:?} seq={}", frame_type, seq);

        match (frame_type, seq) {
            (FrameType::PsStart, 1) => self.pair_setup_m1().await,
            (FrameType::PsNext, 3) => self.pair_setup_m3(&pairing_data).await,
            (FrameType::PsNext, 5) => self.pair_setup_m5(&pairing_data).await,
            (FrameType::PvStart, 1) => self.pair_verify_m1(&pairing_data).await,
            (FrameType::PvNext, 3) => self.pair_verify_m3(&pairing_data).await,
            _ => {
                warn!("unsupported auth step: frame={:?} seq={}", frame_type, seq);
                Ok(())
            }
        }
    }

    /// Withdraw the PIN this connection asked the delegate to show, if any.
    fn hide_pin(&mut self) {
        if self.active_pin.take().is_some() {
            info!("pairing PIN withdrawn");
            self.delegate.hide_pairing_pin();
            self.pairing.release_code(self.conn_id);
        }
    }

    async fn pair_setup_m1(&mut self) -> Result<()> {
        if self.pairing.random_pin {
            if !self.pairing.claim_code(self.conn_id) {
                // One screen: another phone's code is up until that attempt ends.
                warn!("pair-setup refused: another connection has a pairing code on screen");
                self.inspector
                    .emit("auth", "{\"step\":\"pair_setup_m1\",\"busy\":true}");
                return self
                    .send_opack(
                        FrameType::PsNext,
                        &Value::dict(vec![Value::kv(
                            "_pd",
                            Value::Bytes(tlv::write(&[
                                (tags::SEQ_NO, &[0x02]),
                                (tags::ERROR, &[0x07]),
                            ])),
                        )]),
                    )
                    .await;
            }
            // A new PIN, and an SRP verifier for it, for every attempt.
            let pin = pairing::random_pin();
            self.srp = SrpServer::new(pin, &self.private_key);
            self.active_pin = Some(pin);
            info!("pairing PIN is {pin:04}");
            // No PIN here: the inspector stream is served to the network.
            self.inspector.emit("auth", "{\"step\":\"pair_setup_m1\"}");
            self.delegate.show_pairing_pin(&format!("{pin:04}"));
        } else {
            info!("pairing PIN is {:04}", self.pin);
            self.inspector.emit(
                "auth",
                &format!("{{\"step\":\"pair_setup_m1\",\"pin\":{:04}}}", self.pin),
            );
        }
        let tlv_data = tlv::write(&[
            (tags::SEQ_NO, &[0x02]),
            (tags::SALT, self.srp.salt()),
            (tags::PUBLIC_KEY, &self.srp.public_key()),
            (27, &[0x01]),
        ]);
        self.send_opack(
            FrameType::PsNext,
            &Value::dict(vec![
                Value::kv("_pd", Value::Bytes(tlv_data)),
                Value::kv("_pwTy", Value::Int(1)),
            ]),
        )
        .await
    }

    async fn pair_setup_m3(
        &mut self,
        pairing_data: &std::collections::BTreeMap<u8, Vec<u8>>,
    ) -> Result<()> {
        let public_key = pairing_data
            .get(&tags::PUBLIC_KEY)
            .ok_or_else(|| Error::Protocol("missing client public key".into()))?;
        let proof_ok = if self.pairing.random_pin && self.active_pin.is_none() {
            // The PIN of a failed or finished attempt is gone; start over at M1.
            warn!("pair-setup M3 without a PIN on screen");
            false
        } else {
            self.srp.process(public_key)?;
            pairing_data
                .get(&tags::PROOF)
                .map(|p| self.srp.verify_proof(p))
                .unwrap_or(false)
        };
        if !proof_ok {
            self.hide_pin();
        }
        self.inspector.emit(
            "auth",
            &format!("{{\"step\":\"pair_setup_m3\",\"verified\":{proof_ok}}}"),
        );
        let tlv_data = if proof_ok {
            tlv::write(&[
                (tags::SEQ_NO, &[0x04]),
                (tags::PROOF, &self.srp.key_proof_hash()?),
            ])
        } else {
            tlv::write(&[(tags::SEQ_NO, &[0x04]), (tags::ERROR, &[0x02])])
        };
        self.send_opack(
            FrameType::PsNext,
            &Value::dict(vec![Value::kv("_pd", Value::Bytes(tlv_data))]),
        )
        .await
    }

    async fn pair_setup_m5(
        &mut self,
        pairing_data: &std::collections::BTreeMap<u8, Vec<u8>>,
    ) -> Result<()> {
        let shared = self.srp.session_key()?.to_vec();
        let session_key = hkdf_expand("Pair-Setup-Encrypt-Salt", "Pair-Setup-Encrypt-Info", &shared);
        let mut chacha = CompanionCipher::new(&session_key, &session_key, 8)?;
        let encrypted = pairing_data
            .get(&tags::ENCRYPTED_DATA)
            .ok_or_else(|| Error::Protocol("missing encrypted data".into()))?;
        let decrypted = chacha.decrypt(encrypted, None, Some(b"PS-Msg05"))?;
        let client_tlv = tlv::read(&decrypted);
        debug!("paired client: {} TLV entries", client_tlv.len());
        match pairing::verify_setup_client(&client_tlv, &shared) {
            Ok((identifier, ltpk)) => {
                let label = String::from_utf8_lossy(&identifier).into_owned();
                let saved = {
                    let mut clients = self
                        .pairing
                        .clients
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    clients.insert(&identifier, ltpk)
                };
                match saved {
                    Ok(()) => info!("paired client {label} stored"),
                    Err(e) => warn!("paired client {label} kept in memory only: {e}"),
                }
            }
            Err(e) if self.pairing.require_pairing => {
                warn!("pair-setup rejected: {e}");
                self.inspector
                    .emit("auth", "{\"step\":\"pair_setup_m5\",\"verified\":false}");
                self.hide_pin();
                self.send_opack(
                    FrameType::PsNext,
                    &Value::dict(vec![Value::kv(
                        "_pd",
                        Value::Bytes(tlv::write(&[
                            (tags::SEQ_NO, &[0x06]),
                            (tags::ERROR, &[0x02]),
                        ])),
                    )]),
                )
                .await?;
                return Err(e);
            }
            Err(e) => warn!("pair-setup: client signature not verified ({e}); not stored"),
        }

        let accessory_x = hkdf_expand(
            "Pair-Setup-Accessory-Sign-Salt",
            "Pair-Setup-Accessory-Sign-Info",
            &shared,
        );
        let mut device_info = Vec::with_capacity(32 + self.unique_id.len() + 32);
        device_info.extend_from_slice(&accessory_x);
        device_info.extend_from_slice(&self.unique_id);
        device_info.extend_from_slice(&self.auth_pub);
        let signature = self.signing_key.sign(&device_info);

        let extra = Value::dict(vec![
            Value::kv("altIRK", Value::Bytes(ALT_IRK.to_vec())),
            Value::kv(
                "accountID",
                Value::str(&uuid::Uuid::new_v4().to_string().to_uppercase()),
            ),
            Value::kv("model", Value::str(DEVICE_MODEL)),
            Value::kv(
                "wifiMAC",
                Value::Bytes(self.device_mac.to_vec()),
            ),
            Value::kv("name", Value::str(&self.device_name)),
            Value::kv("mac", Value::Bytes(self.device_mac.to_vec())),
        ]);
        let tlv_data = tlv::write(&[
            (tags::IDENTIFIER, &self.unique_id),
            (tags::PUBLIC_KEY, &self.auth_pub),
            (tags::SIGNATURE, &signature.to_bytes()),
            (17, &opack::pack(&extra)),
        ]);
        let mut chacha = CompanionCipher::new(&session_key, &session_key, 8)?;
        let encrypted = chacha.encrypt(&tlv_data, None, Some(b"PS-Msg06"))?;
        self.send_opack(
            FrameType::PsNext,
            &Value::dict(vec![Value::kv(
                "_pd",
                Value::Bytes(tlv::write(&[
                    (tags::SEQ_NO, &[0x06]),
                    (tags::ENCRYPTED_DATA, &encrypted),
                ])),
            )]),
        )
        .await?;
        info!("pairing complete");
        self.hide_pin();
        self.delegate.on_event(EventKind::Paired, "");
        self.inspector.emit("paired", "{}");
        // Fresh salt/verifier for the next pairing attempt.
        self.srp = SrpServer::new(self.pin, &self.private_key);
        Ok(())
    }

    async fn pair_verify_m1(
        &mut self,
        pairing_data: &std::collections::BTreeMap<u8, Vec<u8>>,
    ) -> Result<()> {
        let server_pub = self.verify_pub.to_bytes();
        let client_pub_bytes = pairing_data
            .get(&tags::PUBLIC_KEY)
            .ok_or_else(|| Error::Protocol("missing client public key".into()))?;
        let client_pub: [u8; 32] = client_pub_bytes
            .as_slice()
            .try_into()
            .map_err(|_| Error::Protocol("client public key must be 32 bytes".into()))?;
        let shared = self
            .verify_private
            .diffie_hellman(&X25519PublicKey::from(client_pub));
        let session_key = hkdf_expand(
            "Pair-Verify-Encrypt-Salt",
            "Pair-Verify-Encrypt-Info",
            shared.as_bytes(),
        );

        let mut info = Vec::with_capacity(32 + self.unique_id.len() + 32);
        info.extend_from_slice(&server_pub);
        info.extend_from_slice(&self.unique_id);
        info.extend_from_slice(&client_pub);
        let signature = self.signing_key.sign(&info);

        let mut chacha = CompanionCipher::new(&session_key, &session_key, 8)?;
        let encrypted = chacha.encrypt(
            &tlv::write(&[
                (tags::IDENTIFIER, &self.unique_id),
                (tags::SIGNATURE, &signature.to_bytes()),
            ]),
            None,
            Some(b"PV-Msg02"),
        )?;

        self.pending_verify = Some(PendingVerify {
            session_key,
            client_pub,
            server_pub,
            output_key: hkdf_expand("", "ServerEncrypt-main", shared.as_bytes()),
            input_key: hkdf_expand("", "ClientEncrypt-main", shared.as_bytes()),
        });

        self.inspector.emit("auth", "{\"step\":\"pair_verify_m1\"}");

        self.send_opack(
            FrameType::PvNext,
            &Value::dict(vec![Value::kv(
                "_pd",
                Value::Bytes(tlv::write(&[
                    (tags::SEQ_NO, &[0x02]),
                    (tags::PUBLIC_KEY, &server_pub),
                    (tags::ENCRYPTED_DATA, &encrypted),
                ])),
            )]),
        )
        .await
    }

    /// When pairing is required, decrypt the client's M3 and check its
    /// signature against the long-term key stored at pair-setup.
    fn check_verify_m3(
        &self,
        pending: &PendingVerify,
        pairing_data: &BTreeMap<u8, Vec<u8>>,
    ) -> Result<()> {
        if !self.pairing.require_pairing {
            return Ok(());
        }
        let encrypted = pairing_data
            .get(&tags::ENCRYPTED_DATA)
            .ok_or_else(|| Error::Protocol("missing encrypted data".into()))?;
        let mut chacha = CompanionCipher::new(&pending.session_key, &pending.session_key, 8)?;
        let decrypted = chacha.decrypt(encrypted, None, Some(b"PV-Msg03"))?;
        let clients = self
            .pairing
            .clients
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let identifier = pairing::verify_known_client(
            &tlv::read(&decrypted),
            &pending.client_pub,
            &pending.server_pub,
            &clients,
        )?;
        info!("pair-verify: paired client {}", String::from_utf8_lossy(&identifier));
        Ok(())
    }

    async fn pair_verify_m3(&mut self, pairing_data: &BTreeMap<u8, Vec<u8>>) -> Result<()> {
        let checked = match self.pending_verify.take() {
            Some(pending) => self.check_verify_m3(&pending, pairing_data).map(|()| pending),
            None => Err(Error::Protocol("pair-verify M3 before M1".into())),
        };
        let pending = match checked {
            Ok(pending) => pending,
            Err(e) => {
                warn!("pair-verify rejected: {e}");
                self.inspector
                    .emit("auth", "{\"step\":\"pair_verify_m3\",\"verified\":false}");
                self.send_opack(
                    FrameType::PvNext,
                    &Value::dict(vec![Value::kv(
                        "_pd",
                        Value::Bytes(tlv::write(&[
                            (tags::SEQ_NO, &[0x04]),
                            (tags::ERROR, &[0x02]),
                        ])),
                    )]),
                )
                .await?;
                return Err(e);
            }
        };
        self.cipher = Some(CompanionCipher::new(
            &pending.output_key,
            &pending.input_key,
            12,
        )?);
        self.send_opack(
            FrameType::PvNext,
            &Value::dict(vec![Value::kv(
                "_pd",
                Value::Bytes(tlv::write(&[(tags::SEQ_NO, &[0x04])])),
            )]),
        )
        .await?;
        info!("verification complete, encrypted control channel enabled");
        self.delegate.on_event(EventKind::Verified, "");
        self.inspector.emit("verified", "{}");

        // Kick connections that do not send _sessionStart within 3.0s
        *self.timeout_deadline.lock().await =
            Some(tokio::time::Instant::now() + std::time::Duration::from_millis(3000));

        Ok(())
    }

    async fn handle_control(&mut self, data: &Value) -> Result<()> {
        self.delegate
            .on_event(EventKind::ControlMessage, &data.to_string());
        // iOS 27 sends `_systemInfo` with the identifier in the `model` key
        // instead of `_i`; accept both so the `_systemInfo` arm below fires.
        let identifier = data
            .get("_i")
            .or_else(|| data.get("model"))
            .and_then(Value::as_str)
            .map(str::to_string);
        self.current_identifier = identifier.clone();
        let xid = data.get("_x").and_then(Value::as_int);
        let content = data.get("_c").cloned().unwrap_or(Value::dict(vec![]));

        match identifier.as_deref() {
            Some("_sessionStart") => {
                self.session_established.store(true, Ordering::SeqCst);
                *self.timeout_deadline.lock().await = None;
                self.inspector.session_started();
                let local_sid = content
                    .get("_sid")
                    .or_else(|| content.get("sid"))
                    .and_then(Value::as_int)
                    .unwrap_or_else(|| rand::random::<u32>() as u64);
                let remote_sid = self.remote_session_id;
                self.send_response(xid, Some(Value::dict(vec![Value::kv("_sid", Value::Int(remote_sid as u64))])), 0)
                    .await?;
                info!("<< _sessionStart: session established (local_sid={local_sid}, remote_sid={remote_sid})");
                self.delegate.on_event(
                    EventKind::SessionStarted,
                    &format!(
                        "local={} remote={} combined={}",
                        local_sid,
                        remote_sid,
                        ((remote_sid as u64) << 32) | local_sid
                    ),
                );
                self.inspector.emit(
                    "session_started",
                    &format!("{{\"local_sid\":{local_sid},\"remote_sid\":{remote_sid}}}"),
                );
            }
            Some("TVRCSessionStart") => {
                self.session_established.store(true, Ordering::SeqCst);
                *self.timeout_deadline.lock().await = None;
                self.inspector.session_started();
                let version = content
                    .get("ProtocolVersionKey")
                    .and_then(Value::as_str)
                    .unwrap_or("1.2")
                    .to_string();
                self.send_response(
                    xid,
                    Some(Value::dict(vec![Value::kv(
                        "ProtocolVersionKey",
                        Value::str(&version),
                    )])),
                    0,
                )
                .await?;
                // Current iOS versions make the volume/mute decision
                // immediately after session startup, so publish the flag
                // before subscriptions have necessarily arrived.
                self.send_media_capabilities(xid).await?;
                info!("<< TVRCSessionStart: remote ready (version={version})");
                self.delegate.on_event(EventKind::RemoteReady, "");
                self.inspector.emit("remote_ready", &format!("{{\"version\":\"{version}\"}}"));
            }
            Some("_sessionStop") => {
                self.session_established.store(false, Ordering::SeqCst);
                self.inspector.session_stopped();
                self.send_response(xid, None, 0).await?;
                info!("<< _sessionStop: remote session closed");
                self.delegate
                    .on_event(EventKind::SessionStopped, &content.to_string());
                self.inspector.emit("session_stopped", "{}");
            }
            Some("_hidC") => {
                self.session_established.store(true, Ordering::SeqCst);
                *self.timeout_deadline.lock().await = None;
                self.handle_hid_command(&content, xid).await?;
            }
            Some("_touchStart") => {
                self.session_established.store(true, Ordering::SeqCst);
                *self.timeout_deadline.lock().await = None;
                self.send_response(xid, Some(Value::dict(vec![Value::kv("_i", Value::Int(1))])), 0)
                    .await?;
            }
            Some("_hidT") => {
                self.session_established.store(true, Ordering::SeqCst);
                *self.timeout_deadline.lock().await = None;
                self.send_response(xid, None, 0).await?;
                self.handle_touch(&content);
            }
            Some("_touchStop") | Some("_touchMove") | Some("_tiStop") => {
                self.session_established.store(true, Ordering::SeqCst);
                *self.timeout_deadline.lock().await = None;
                self.send_response(xid, None, 0).await?;
            }
            Some("_interest") => {
                // _interest itself is an event and does not need a response.
                // A real Apple TV pushes the current state for each newly
                // registered event.
                if let Some(Value::Array(events)) = content.get("_regEvents") {
                    let names: Vec<String> = events
                        .iter()
                        .filter_map(|e| e.as_str().map(str::to_string))
                        .collect();
                    for event_name in names {
                        match event_name.as_str() {
                            "MediaControlStatus" => {
                                self.send_event(
                                    &event_name,
                                    xid,
                                    Some(Value::dict(vec![Value::kv(
                                        "MediaControlFlags",
                                        Value::Int(MEDIA_CONTROL_VOLUME),
                                    )])),
                                )
                                .await?;
                            }
                            "NowPlayingInfo" => {
                                self.send_event(&event_name, xid, None).await?;
                            }
                            "SystemStatus" | "TVSystemStatus" => {
                                self.send_event(
                                    &event_name,
                                    xid,
                                    Some(Value::dict(vec![Value::kv(
                                        "state",
                                        Value::Int(self.system_status),
                                    )])),
                                )
                                .await?;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Some("FetchMediaControlStatus") => {
                self.send_response(
                    xid,
                    Some(Value::dict(vec![Value::kv(
                        "MediaControlFlags",
                        Value::Int(MEDIA_CONTROL_VOLUME),
                    )])),
                    0,
                )
                .await?;
            }
            Some("FetchSiriRemoteInfo") | Some("FetchCurrentNowPlayingInfoEvent") => {
                self.send_response(xid, None, 0).await?;
            }
            Some("MediaControlCommand") | Some("_mcc") => {
                self.handle_media_control(&content, xid).await?;
            }
            Some("_tiStart") => {
                self.send_response(
                    xid,
                    Some(Value::dict(vec![Value::kv("_tiE", Value::Bool(false))])),
                    0,
                )
                .await?;
            }
            Some("FetchLaunchableApplicationsEvent") => {
                self.send_response(
                    xid,
                    Some(Value::dict(vec![
                        Value::kv("com.apple.TVAppStore", Value::str("App Store")),
                        Value::kv("com.apple.TVSettings", Value::str("Settings")),
                        Value::kv("com.apple.TVMusic", Value::str("Music")),
                        Value::kv("com.apple.TVMovies", Value::str("Movies")),
                        Value::kv("com.apple.TVWatchList", Value::str("TV")),
                        Value::kv("com.google.ios.youtube", Value::str("YouTube")),
                        Value::kv("com.netflix.Netflix", Value::str("Netflix")),
                    ])),
                    2,
                )
                .await?;
            }
            Some("FetchAttentionState") => {
                self.send_response(
                    xid,
                    Some(Value::dict(vec![Value::kv(
                        "state",
                        Value::Int(self.system_status),
                    )])),
                    0,
                )
                .await?;
            }
            Some("_launchApp") => {
                let bundle_id = content
                    .get("_bundleID")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                self.delegate.on_event(EventKind::LaunchApp, &bundle_id);
                self.inspector.emit(
                    "launch_app",
                    &format!("{{\"bundle_id\":\"{bundle_id}\"}}"),
                );
                self.send_response(xid, None, 0).await?;
            }
            Some("_systemInfo") | Some("SystemInfo") => {
                let ids_id = content.get("_idsID").and_then(Value::as_str);
                let pub_id = content.get("_pubID").and_then(Value::as_str);
                let name = content.get("name").and_then(Value::as_str).unwrap_or("");
                let model = content.get("model").and_then(Value::as_str).unwrap_or("");

                let primary_id = ids_id.or(pub_id).unwrap_or(&self.peer_ip);
                let alias_id = if ids_id.is_some() { pub_id } else { None };

                let (session_id, is_renewed) = {
                    let mut reg = self.registry.lock().await;
                    reg.get_or_register(primary_id, alias_id, name, model)
                };
                self.remote_session_id = session_id;
                self.device_id = Some(primary_id.to_string());

                {
                    let mut tracker = self.active_tracker.lock().await;
                    tracker.set_device_id(self.conn_id, primary_id);
                }

                self.send_response(xid, None, 0).await?;
                info!("<< _systemInfo: replied and refreshed power state (session_id={session_id}, renewed={is_renewed})");
                self.send_power_state(None).await?;

                // Background probes from sharingd/rapportd have high bit set in xid (>= 0x80000000)
                // and never send _sessionStart. Real remote control (tvremoted) sends _sessionStart
                // within 10-20ms. If probe, set a short 1.0s timeout to release the channel.
                let is_background_probe = xid.map_or(false, |x| (x & 0x8000_0000) != 0);
                let timeout = if is_background_probe {
                    std::time::Duration::from_millis(1000)
                } else {
                    std::time::Duration::from_millis(2500)
                };
                *self.timeout_deadline.lock().await =
                    Some(tokio::time::Instant::now() + timeout);
            }
            _ => {
                if let Some(id) = &identifier {
                    debug!("<< unhandled control message: {id} content={content}");
                }
                if xid.is_some() {
                    self.send_response(xid, None, 0).await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_hid_command(&mut self, content: &Value, xid: Option<u64>) -> Result<()> {
        let command = content.get("_hidC").and_then(Value::as_int).unwrap_or(0);
        let state = content.get("_hBtS").and_then(Value::as_int).unwrap_or(0);
        let command_name = hid_command_name(command);
        self.send_response(xid, None, 0).await?;

        if state == 2 {
            match command_name.as_str() {
                "volume_up" | "volume_down" => {
                    self.delegate.on_button(&command_name);
                    if let Some((vol, muted)) = self.delegate.get_audio_state() {
                        self.volume = vol;
                        self.muted = muted;
                    } else {
                        let step = 1.0 / 16.0;
                        if command_name == "volume_up" {
                            self.volume = (self.volume + step).min(1.0);
                        } else {
                            self.volume = (self.volume - step).max(0.0);
                        }
                        self.muted = false;
                        self.delegate.on_audio(self.volume, self.muted);
                    }
                    self.inspector.emit(
                        "audio_state",
                        &format!("{{\"volume\":{:.3},\"muted\":{}}}", self.volume, self.muted),
                    );
                    self.push_media_state(xid).await?;
                }
                "mute" => {
                    self.delegate.on_button(&command_name);
                    if let Some((vol, muted)) = self.delegate.get_audio_state() {
                        self.volume = vol;
                        self.muted = muted;
                    } else {
                        self.muted = !self.muted;
                        self.delegate.on_audio(self.volume, self.muted);
                    }
                    self.inspector.emit(
                        "audio_state",
                        &format!("{{\"volume\":{:.3},\"muted\":{}}}", self.volume, self.muted),
                    );
                    self.send_event(
                        "MediaControlStatus",
                        xid,
                        Some(Value::dict(vec![
                            Value::kv("MediaControlFlags", Value::Int(MEDIA_CONTROL_VOLUME)),
                            Value::kv("Muted", Value::Bool(self.muted)),
                        ])),
                    )
                    .await?;
                    self.send_event(
                        "_iMC",
                        xid,
                        Some(Value::dict(vec![Value::kv(
                            "_mcF",
                            Value::Int(MEDIA_CONTROL_VOLUME),
                        )])),
                    )
                    .await?;
                }
                "power" => {
                    self.display_power_on = !self.display_power_on;
                    self.delegate.on_event(
                        EventKind::PowerState,
                        if self.display_power_on { "on" } else { "off" },
                    );
                    self.inspector.emit(
                        "power_state",
                        &format!("{{\"display_on\":{}}}", self.display_power_on),
                    );
                    self.delegate.on_button(&command_name);
                }
                _ => {
                    self.delegate.on_button(&command_name);
                }
            }
            self.inspector.emit(
                "button",
                &format!("{{\"name\":\"{command_name}\"}}"),
            );
        }
        Ok(())
    }

    async fn handle_media_control(&mut self, content: &Value, xid: Option<u64>) -> Result<()> {
        let command = content
            .get("MediaControlCommand")
            .or_else(|| content.get("_mcc"))
            .and_then(Value::as_int);
        let mut response = Value::dict(vec![]);
        match command {
            // GetVolume: report 0 while muted so the phone's volume HUD shows
            // an empty bar — the only visual mute feedback this protocol has.
            Some(5) => {
                if let Some((vol, muted)) = self.delegate.get_audio_state() {
                    self.volume = vol;
                    self.muted = muted;
                }
                response = Value::dict(vec![Value::kv(
                    "_vol",
                    Value::Double(if self.muted { 0.0 } else { self.volume }),
                )]);
            }
            // SetVolume
            Some(6) => {
                if let Some(requested) = content.get("_vol").and_then(Value::as_float) {
                    self.volume = requested.clamp(0.0, 1.0);
                    // Raising the level from the phone lifts the mute.
                    if self.volume > 0.0 {
                        self.muted = false;
                    }
                    self.delegate.on_audio(self.volume, self.muted);
                    self.inspector.emit(
                        "audio_state",
                        &format!("{{\"volume\":{:.3},\"muted\":{}}}", self.volume, self.muted),
                    );
                }
            }
            // GetCaptionSettings
            Some(12) => {
                response = Value::dict(vec![Value::kv("_cse", Value::Bool(false))]);
            }
            _ => {}
        }
        self.send_response(xid, Some(response), 0).await
    }

    /// Translate touchpad gestures into directional button presses (direction mode),
    /// pointer-style deltas (mouse mode), or trajectory preview only (idle mode).
    fn handle_touch(&mut self, content: &Value) {
        let phase = content.get("_tPh").and_then(Value::as_int).unwrap_or(0);
        let x = content.get("_cx").and_then(Value::as_float);
        let y = content.get("_cy").and_then(Value::as_float);

        let mode_u8 = self.trackpad_mode.load(Ordering::SeqCst);
        let mode = TrackpadMode::from_u8(mode_u8);

        if phase == 1 {
            // touch began
            if let (Some(x), Some(y)) = (x, y) {
                self.touch_origin = Some((x, y));
                self.touch_last = Some((x, y));
                self.touch_moved = false;
                if mode == TrackpadMode::Mouse {
                    self.delegate.on_touch(0.0, 0.0, TouchPhase::Began);
                }
                self.inspector.emit("touch", "{\"phase\":1,\"dx\":0.0,\"dy\":0.0}");
            }
            return;
        }
        let (Some(origin), Some(x), Some(y)) = (self.touch_origin, x, y) else {
            return;
        };
        let position = (x, y);

        if phase != 4 {
            // intermediate move
            if let Some(last) = self.touch_last {
                let dx = position.0 - last.0;
                let dy = position.1 - last.1;
                if dx != 0.0 || dy != 0.0 {
                    if mode == TrackpadMode::Mouse {
                        self.touch_moved = true;
                        self.delegate.on_touch(dx, dy, TouchPhase::Moved);
                    }
                    self.inspector.emit(
                        "touch",
                        &format!("{{\"phase\":2,\"dx\":{:.1},\"dy\":{:.1}}}", dx, dy),
                    );
                }
            }
            self.touch_last = Some(position);
            return;
        }

        // phase == 4: finger lifted
        self.touch_origin = None;
        self.touch_last = None;
        let dx = position.0 - origin.0;
        let dy = position.1 - origin.1;
        let distance = dx.abs().max(dy.abs());

        if mode == TrackpadMode::Mouse {
            self.delegate.on_touch(dx, dy, TouchPhase::Ended);
        }
        self.inspector.emit(
            "touch",
            &format!("{{\"phase\":4,\"dx\":{:.1},\"dy\":{:.1}}}", dx, dy),
        );

        // In Idle mode: only emit touch trajectory for real-time preview, do not trigger any actions
        if mode == TrackpadMode::Idle {
            return;
        }

        if mode == TrackpadMode::Mouse {
            if distance < TAP_MAX_DISTANCE {
                self.delegate.on_event(EventKind::MouseClick, "");
                self.inspector.emit("button", "{\"name\":\"click\"}");
            }
            return;
        }

        if distance < TAP_MAX_DISTANCE {
            self.delegate.on_button("select");
            self.inspector.emit("button", "{\"name\":\"select\"}");
            return;
        }

        if distance < SWIPE_MIN_DISTANCE {
            return;
        }
        let direction = if dx.abs() > dy.abs() {
            if dx > 0.0 {
                "right"
            } else {
                "left"
            }
        } else if dy > 0.0 {
            "down"
        } else {
            "up"
        };
        self.delegate.on_button(direction);
        self.inspector.emit(
            "button",
            &format!("{{\"name\":\"{direction}\"}}"),
        );
    }
}

impl Drop for CompanionSession {
    fn drop(&mut self) {
        self.hide_pin();
        if self.session_established.load(Ordering::SeqCst) {
            self.inspector.session_stopped();
        }
    }
}

#[cfg(test)]
mod tests {
    //! Pairing over a real socket: the test plays the phone, the session the TV.

    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::Mutex as StdMutex;

    use num_bigint::BigUint;
    use sha2::{Digest, Sha512};
    use tokio::io::AsyncReadExt;
    use tokio::net::tcp::OwnedReadHalf;
    use tokio::net::{TcpListener, TcpStream};

    use crate::pairing::PairedClients;
    use crate::srp::{int_to_bytes, PRIME_3072_HEX};

    #[derive(Default)]
    struct Recorder {
        calls: StdMutex<Vec<String>>,
    }

    impl Recorder {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        fn last_shown_pin(&self) -> u32 {
            self.calls()
                .iter()
                .rev()
                .find_map(|c| c.strip_prefix("show:").map(|p| p.parse().unwrap()))
                .expect("a PIN was shown")
        }
    }

    impl AtvDelegate for Recorder {
        fn show_pairing_pin(&self, pin: &str) {
            self.calls.lock().unwrap().push(format!("show:{pin}"));
        }

        fn hide_pairing_pin(&self) {
            self.calls.lock().unwrap().push("hide".into());
        }
    }

    struct Phone {
        id: Vec<u8>,
        key: SigningKey,
    }

    impl Phone {
        fn new(id: &str, seed: u8) -> Self {
            Self {
                id: id.as_bytes().to_vec(),
                key: SigningKey::from_bytes(&[seed; 32]),
            }
        }

        fn ltpk(&self) -> [u8; 32] {
            self.key.verifying_key().to_bytes()
        }
    }

    static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

    struct Peer {
        session: CompanionSession,
        socket: TcpStream,
        _server_reader: OwnedReadHalf,
    }

    impl Peer {
        async fn connect(pairing: PairingPolicy, delegate: Arc<Recorder>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let socket = TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
            let (server, peer) = listener.accept().await.unwrap();
            let (reader, writer) = server.into_split();
            let identity = DeviceIdentity {
                name: "Test TV".into(),
                device_id: "02:00:00:00:00:01".into(),
                mac_bytes: [2, 0, 0, 0, 0, 1],
                server_identifier: SERVER_IDENTIFIER.into(),
                unique_id: SERVER_IDENTIFIER.replace('-', ""),
                private_key: [0x11; 32],
            };
            let delegate: Arc<dyn AtvDelegate> = delegate;
            let session = CompanionSession::new(
                Arc::new(Mutex::new(writer)),
                delegate,
                &identity,
                1111,
                pairing,
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicU8::new(0)),
                InspectorHub::new(),
                Arc::new(Mutex::new(DeviceRegistry::new(std::time::Duration::from_secs(60)))),
                Arc::new(Mutex::new(ActiveConnectionTracker::new())),
                NEXT_CONN_ID.fetch_add(1, Ordering::SeqCst),
                peer.ip().to_string(),
                Arc::new(AtomicBool::new(false)),
            );
            Self {
                session,
                socket,
                _server_reader: reader,
            }
        }

        /// Hand one auth frame to the session as if it came off the wire.
        async fn send(&mut self, frame_type: FrameType, seq: u8, entries: &[(u8, &[u8])]) -> Result<()> {
            let seq = [seq];
            let mut items = vec![(tags::SEQ_NO, &seq[..])];
            items.extend_from_slice(entries);
            let payload = opack::pack(&Value::dict(vec![Value::kv(
                "_pd",
                Value::Bytes(tlv::write(&items)),
            )]));
            let mut header = vec![frame_type as u8];
            header.extend_from_slice(&(payload.len() as u32).to_be_bytes()[1..]);
            self.session.handle_frame(frame_type, &payload, &header).await
        }

        /// Read the next frame the session wrote and return its pairing TLV.
        async fn recv(&mut self) -> BTreeMap<u8, Vec<u8>> {
            let mut header = [0u8; 4];
            self.socket.read_exact(&mut header).await.unwrap();
            let length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
            let mut payload = vec![0u8; length];
            self.socket.read_exact(&mut payload).await.unwrap();
            let (value, _) = opack::unpack(&payload).unwrap();
            tlv::read(value.get("_pd").and_then(Value::as_bytes).unwrap())
        }

        /// Pair-setup M1: returns M2.
        async fn m1(&mut self) -> BTreeMap<u8, Vec<u8>> {
            self.send(FrameType::PsStart, 1, &[(tags::METHOD, &[0])]).await.unwrap();
            self.recv().await
        }

        /// Pair-setup M1/M2: returns the PIN shown on screen, salt and B.
        async fn setup_m1(&mut self, delegate: &Recorder) -> (u32, Vec<u8>, Vec<u8>) {
            let m2 = self.m1().await;
            assert!(!m2.contains_key(&tags::ERROR), "M2 {m2:?}");
            (delegate.last_shown_pin(), m2[&tags::SALT].clone(), m2[&tags::PUBLIC_KEY].clone())
        }

        /// Pair-setup M3/M4 with `pin`: returns M4 and the client's SRP key.
        async fn setup_m3(&mut self, pin: u32, salt: &[u8], b: &[u8]) -> (BTreeMap<u8, Vec<u8>>, Vec<u8>) {
            let (a, proof, key) = srp_client(pin, salt, b);
            self.send(FrameType::PsNext, 3, &[(tags::PUBLIC_KEY, &a), (tags::PROOF, &proof)])
                .await
                .unwrap();
            (self.recv().await, key)
        }

        /// Pair-setup M5/M6: presents `phone`'s identifier and LTPK, signed by `signer`.
        async fn setup_m5(&mut self, phone: &Phone, signer: &SigningKey, srp_key: &[u8]) -> (Result<()>, BTreeMap<u8, Vec<u8>>) {
            let ltpk = phone.ltpk();
            let device_x = hkdf_expand(
                "Pair-Setup-Controller-Sign-Salt",
                "Pair-Setup-Controller-Sign-Info",
                srp_key,
            );
            let signature = signer.sign(&[&device_x[..], &phone.id[..], &ltpk[..]].concat());
            let sub_tlv = tlv::write(&[
                (tags::IDENTIFIER, &phone.id),
                (tags::PUBLIC_KEY, &ltpk),
                (tags::SIGNATURE, &signature.to_bytes()),
            ]);
            let key = hkdf_expand("Pair-Setup-Encrypt-Salt", "Pair-Setup-Encrypt-Info", srp_key);
            let encrypted = CompanionCipher::new(&key, &key, 8)
                .unwrap()
                .encrypt(&sub_tlv, None, Some(b"PS-Msg05"))
                .unwrap();
            let result = self.send(FrameType::PsNext, 5, &[(tags::ENCRYPTED_DATA, &encrypted)]).await;
            (result, self.recv().await)
        }

        /// Full pair-verify as `id`, signing M3 with `signer`: returns the
        /// session's verdict and M4.
        async fn verify(&mut self, id: &[u8], signer: &SigningKey) -> (Result<()>, BTreeMap<u8, Vec<u8>>) {
            let secret = StaticSecret::from([0x33; 32]);
            let client_pub = X25519PublicKey::from(&secret).to_bytes();
            self.send(FrameType::PvStart, 1, &[(tags::PUBLIC_KEY, &client_pub)]).await.unwrap();
            let m2 = self.recv().await;
            let server_pub: [u8; 32] = m2[&tags::PUBLIC_KEY].as_slice().try_into().unwrap();
            let shared = secret.diffie_hellman(&X25519PublicKey::from(server_pub));
            let key = hkdf_expand(
                "Pair-Verify-Encrypt-Salt",
                "Pair-Verify-Encrypt-Info",
                shared.as_bytes(),
            );
            let signature = signer.sign(&[&client_pub[..], id, &server_pub[..]].concat());
            let sub_tlv = tlv::write(&[(tags::IDENTIFIER, id), (tags::SIGNATURE, &signature.to_bytes())]);
            let encrypted = CompanionCipher::new(&key, &key, 8)
                .unwrap()
                .encrypt(&sub_tlv, None, Some(b"PV-Msg03"))
                .unwrap();
            let result = self.send(FrameType::PvNext, 3, &[(tags::ENCRYPTED_DATA, &encrypted)]).await;
            (result, self.recv().await)
        }
    }

    fn sha512(parts: &[&[u8]]) -> Vec<u8> {
        let mut hasher = Sha512::new();
        for part in parts {
            hasher.update(part);
        }
        hasher.finalize().to_vec()
    }

    fn pad(value: &BigUint, n: &BigUint) -> Vec<u8> {
        let bytes = int_to_bytes(value);
        [vec![0; int_to_bytes(n).len() - bytes.len()], bytes].concat()
    }

    /// SRP-6a client (srptools / pyatv): returns A, the proof M1 and the session key K.
    fn srp_client(pin: u32, salt: &[u8], server_public: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let n = BigUint::parse_bytes(PRIME_3072_HEX.as_bytes(), 16).unwrap();
        let g = BigUint::from(5u32);
        let k = BigUint::from_bytes_be(&sha512(&[&int_to_bytes(&n), &pad(&g, &n)]));
        let inner = sha512(&[format!("Pair-Setup:{pin}").as_bytes()]);
        let x = BigUint::from_bytes_be(&sha512(&[salt, &inner]));
        let a = BigUint::from_bytes_be(&[0x42; 32]);
        let a_pub = g.modpow(&a, &n);
        let b_pub = BigUint::from_bytes_be(server_public);
        let u = BigUint::from_bytes_be(&sha512(&[&pad(&a_pub, &n), &pad(&b_pub, &n)]));
        let base = (&b_pub + &n - (&k * g.modpow(&x, &n)) % &n) % &n;
        let premaster = base.modpow(&(&a + &u * &x), &n);
        let key = sha512(&[&int_to_bytes(&premaster)]);
        let hn = BigUint::from_bytes_be(&sha512(&[&int_to_bytes(&n)]));
        let hg = BigUint::from_bytes_be(&sha512(&[&int_to_bytes(&g)]));
        let proof = sha512(&[
            &int_to_bytes(&(hn ^ hg)),
            &sha512(&[b"Pair-Setup"]),
            salt,
            &int_to_bytes(&a_pub),
            &int_to_bytes(&b_pub),
            &key,
        ]);
        (int_to_bytes(&a_pub), proof, key)
    }

    fn enforcing(clients: PairedClients) -> PairingPolicy {
        PairingPolicy::new(true, true, clients)
    }

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "atv-session-{}-{}",
            std::process::id(),
            rand::random::<u32>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn stored_client_is_accepted() {
        let dir = temp_dir();
        let delegate = Arc::new(Recorder::default());
        let phone = Phone::new("phone-1", 1);

        let mut peer = Peer::connect(enforcing(PairedClients::open(Some(&dir))), delegate.clone()).await;
        let (pin, salt, b) = peer.setup_m1(&delegate).await;
        let (m4, srp_key) = peer.setup_m3(pin, &salt, &b).await;
        assert!(m4.contains_key(&tags::PROOF), "M4 {m4:?}");
        let (result, m6) = peer.setup_m5(&phone, &phone.key, &srp_key).await;
        result.unwrap();
        assert!(m6.contains_key(&tags::ENCRYPTED_DATA) && !m6.contains_key(&tags::ERROR));
        assert_eq!(delegate.calls(), vec![format!("show:{pin}"), "hide".to_string()]);
        drop(peer);

        // A new server instance reads the pairing back from disk.
        let mut peer = Peer::connect(enforcing(PairedClients::open(Some(&dir))), delegate.clone()).await;
        let (result, m4) = peer.verify(&phone.id, &phone.key).await;
        result.unwrap();
        assert!(!m4.contains_key(&tags::ERROR), "M4 {m4:?}");
        assert!(peer.session.cipher.is_some(), "control channel enabled");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn unknown_identifier_is_refused() {
        let paired = Phone::new("phone-1", 1);
        let stranger = Phone::new("phone-2", 2);
        let mut clients = PairedClients::open(None);
        clients.insert(&paired.id, paired.ltpk()).unwrap();

        let mut peer = Peer::connect(enforcing(clients), Arc::new(Recorder::default())).await;
        let (result, m4) = peer.verify(&stranger.id, &stranger.key).await;
        assert!(result.is_err(), "the connection must close");
        assert_eq!(m4.get(&tags::ERROR), Some(&vec![0x02]), "kTLVError_Authentication");
        assert!(peer.session.cipher.is_none(), "no control channel");
    }

    #[tokio::test]
    async fn bad_signature_is_refused() {
        let paired = Phone::new("phone-1", 1);
        let impostor = SigningKey::from_bytes(&[2; 32]);
        let mut clients = PairedClients::open(None);
        clients.insert(&paired.id, paired.ltpk()).unwrap();
        let pairing = enforcing(clients);
        let delegate = Arc::new(Recorder::default());

        // Pair-verify: a paired identifier, signed with another key.
        let mut peer = Peer::connect(pairing.clone(), delegate.clone()).await;
        let (result, m4) = peer.verify(&paired.id, &impostor).await;
        assert!(result.is_err(), "the connection must close");
        assert_eq!(m4.get(&tags::ERROR), Some(&vec![0x02]));
        assert!(peer.session.cipher.is_none());

        // Pair-setup M5: the right PIN, but the LTPK does not sign the request.
        let newcomer = Phone::new("phone-3", 3);
        let mut peer = Peer::connect(pairing.clone(), delegate.clone()).await;
        let (pin, salt, b) = peer.setup_m1(&delegate).await;
        let (_, srp_key) = peer.setup_m3(pin, &salt, &b).await;
        let (result, m6) = peer.setup_m5(&newcomer, &impostor, &srp_key).await;
        assert!(result.is_err());
        assert_eq!(m6.get(&tags::ERROR), Some(&vec![0x02]));
        assert_eq!(pairing.clients.lock().unwrap().get(&newcomer.id), None, "not stored");
        assert_eq!(delegate.calls().last().map(String::as_str), Some("hide"));
    }

    #[tokio::test]
    async fn every_attempt_gets_a_new_pin() {
        let delegate = Arc::new(Recorder::default());
        let mut peer = Peer::connect(enforcing(PairedClients::open(None)), delegate.clone()).await;

        let (first, _, _) = peer.setup_m1(&delegate).await;
        let (second, salt, b) = loop {
            let attempt = peer.setup_m1(&delegate).await;
            if attempt.0 != first {
                break attempt;
            }
        };
        // The previous attempt's PIN does not pair against the new verifier.
        let (m4, _) = peer.setup_m3(first, &salt, &b).await;
        assert_eq!(m4.get(&tags::ERROR), Some(&vec![0x02]));
        assert_eq!(delegate.calls().last().map(String::as_str), Some("hide"));

        // After a failure the PIN is gone: M3 needs a new M1 first.
        let (m4, _) = peer.setup_m3(second, &salt, &b).await;
        assert_eq!(m4.get(&tags::ERROR), Some(&vec![0x02]));

        let (third, salt, b) = peer.setup_m1(&delegate).await;
        let (m4, _) = peer.setup_m3(third, &salt, &b).await;
        assert!(m4.contains_key(&tags::PROOF), "M4 {m4:?}");

        // Closing the connection mid-pairing hides the PIN.
        drop(peer);
        assert_eq!(delegate.calls().last().map(String::as_str), Some("hide"));
        for call in delegate.calls() {
            if let Some(pin) = call.strip_prefix("show:") {
                assert!(pin.len() == 4 && (1000..=9999).contains(&pin.parse::<u32>().unwrap()));
            }
        }
    }

    #[tokio::test]
    async fn without_enforcement_any_client_is_admitted() {
        let delegate = Arc::new(Recorder::default());
        let pairing = PairingPolicy::new(false, false, PairedClients::open(None));
        let mut peer = Peer::connect(pairing, delegate.clone()).await;
        let stranger = Phone::new("phone-2", 2);
        let (result, m4) = peer.verify(&stranger.id, &stranger.key).await;
        result.unwrap();
        assert!(!m4.contains_key(&tags::ERROR));
        assert!(peer.session.cipher.is_some());

        // The fixed PIN still pairs, and nothing is shown.
        peer.send(FrameType::PsStart, 1, &[(tags::METHOD, &[0])]).await.unwrap();
        let m2 = peer.recv().await;
        let (m4, _) = peer.setup_m3(1111, &m2[&tags::SALT], &m2[&tags::PUBLIC_KEY]).await;
        assert!(m4.contains_key(&tags::PROOF), "M4 {m4:?}");
        assert!(delegate.calls().is_empty());
    }

    #[tokio::test]
    async fn second_pairing_is_busy_while_a_code_is_shown() {
        let delegate = Arc::new(Recorder::default());
        let pairing = enforcing(PairedClients::open(None));
        let mut first = Peer::connect(pairing.clone(), delegate.clone()).await;
        let mut second = Peer::connect(pairing.clone(), delegate.clone()).await;

        let (pin, _, _) = first.setup_m1(&delegate).await;
        let m2 = second.m1().await;
        assert_eq!(m2.get(&tags::ERROR), Some(&vec![0x07]), "kTLVError_Busy");
        assert!(!m2.contains_key(&tags::SALT));
        assert_eq!(delegate.calls(), vec![format!("show:{pin}")], "the first code stays up");

        // The phone that holds the screen may restart its own attempt.
        let (again, _, _) = first.setup_m1(&delegate).await;
        assert_eq!(delegate.calls().last(), Some(&format!("show:{again}")));
    }

    #[tokio::test]
    async fn screen_is_released_after_success_failure_and_close() {
        let delegate = Arc::new(Recorder::default());
        let pairing = enforcing(PairedClients::open(None));
        let phone = Phone::new("phone-1", 1);
        let mut a = Peer::connect(pairing.clone(), delegate.clone()).await;
        let mut b = Peer::connect(pairing.clone(), delegate.clone()).await;

        // Success: A pairs, then B may start.
        let (pin, salt, srp_b) = a.setup_m1(&delegate).await;
        assert_eq!(b.m1().await.get(&tags::ERROR), Some(&vec![0x07]));
        let (_, srp_key) = a.setup_m3(pin, &salt, &srp_b).await;
        a.setup_m5(&phone, &phone.key, &srp_key).await.0.unwrap();
        let (b_pin, salt, srp_b) = b.setup_m1(&delegate).await;

        // Failure: B types a wrong code, then A may start.
        assert_eq!(a.m1().await.get(&tags::ERROR), Some(&vec![0x07]));
        let wrong = if b_pin == 9999 { 1000 } else { b_pin + 1 };
        let (m4, _) = b.setup_m3(wrong, &salt, &srp_b).await;
        assert_eq!(m4.get(&tags::ERROR), Some(&vec![0x02]));
        a.setup_m1(&delegate).await;

        // Close: A's connection drops mid-pairing, then B may start.
        assert_eq!(b.m1().await.get(&tags::ERROR), Some(&vec![0x07]));
        drop(a);
        assert_eq!(delegate.calls().last().map(String::as_str), Some("hide"));
        b.setup_m1(&delegate).await;
    }
}

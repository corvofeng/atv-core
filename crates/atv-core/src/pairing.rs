//! Pairing policy and the list of paired clients.
//!
//! Pair-setup ends with the client handing over its pairing identifier and
//! long-term Ed25519 public key (LTPK), signed with that key (HAP M5). The
//! server keeps identifier -> LTPK; when pairing is required, pair-verify
//! admits only a client whose M3 signature checks out against the stored key.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ed25519_dalek::{Signature, VerifyingKey};
use rand::Rng;
use tracing::warn;

use crate::crypto::hkdf_expand;
use crate::error::{Error, Result};
use crate::tlv::tags;

/// File name of the paired-client list inside `AtvConfig::pairing_dir`.
pub const PAIRED_CLIENTS_FILE: &str = "paired_clients.txt";

/// How the server pairs and admits clients; shared by all connections.
#[derive(Clone)]
pub struct PairingPolicy {
    /// Draw a new PIN for every pair-setup attempt and show it via the delegate.
    pub random_pin: bool,
    /// Pair-verify admits only clients found in `clients` with a valid signature.
    pub require_pairing: bool,
    pub clients: Arc<Mutex<PairedClients>>,
    /// Connection whose pairing code is on screen. There is one screen, so a
    /// second pair-setup waits until that code is hidden (HAP kTLVError_Busy).
    code_owner: Arc<Mutex<Option<u64>>>,
}

impl PairingPolicy {
    pub fn new(random_pin: bool, require_pairing: bool, clients: PairedClients) -> Self {
        Self {
            random_pin,
            require_pairing,
            clients: Arc::new(Mutex::new(clients)),
            code_owner: Arc::new(Mutex::new(None)),
        }
    }

    /// Take the screen for connection `conn_id`'s code; false while another
    /// connection's code is shown.
    pub fn claim_code(&self, conn_id: u64) -> bool {
        let mut owner = self.code_owner.lock().unwrap_or_else(|p| p.into_inner());
        match *owner {
            Some(other) if other != conn_id => false,
            _ => {
                *owner = Some(conn_id);
                true
            }
        }
    }

    /// Give the screen back if connection `conn_id` holds it.
    pub fn release_code(&self, conn_id: u64) {
        let mut owner = self.code_owner.lock().unwrap_or_else(|p| p.into_inner());
        if *owner == Some(conn_id) {
            *owner = None;
        }
    }
}

/// A fresh four-digit PIN. It never starts with zero: the SRP password is the
/// PIN in decimal, so "0421" typed on the phone and 421 here would differ.
pub fn random_pin() -> u32 {
    rand::thread_rng().gen_range(1000..=9999)
}

/// Paired clients: pairing identifier -> long-term public key. With a
/// directory, the list lives in [`PAIRED_CLIENTS_FILE`] as one
/// `<hex identifier> <hex ltpk>` line per client; without one, in memory only.
#[derive(Debug, Default)]
pub struct PairedClients {
    path: Option<PathBuf>,
    clients: HashMap<Vec<u8>, [u8; 32]>,
}

impl PairedClients {
    /// Load the list from `dir`; a missing file is an empty list.
    pub fn open(dir: Option<&Path>) -> Self {
        let path = dir.map(|d| d.join(PAIRED_CLIENTS_FILE));
        let mut clients = HashMap::new();
        if let Some(path) = &path {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    match parse_line(line) {
                        Some((identifier, ltpk)) => {
                            clients.insert(identifier, ltpk);
                        }
                        None if line.trim().is_empty() => {}
                        None => warn!("skipping malformed line in {}", path.display()),
                    }
                }
            }
        }
        Self { path, clients }
    }

    pub fn get(&self, identifier: &[u8]) -> Option<[u8; 32]> {
        self.clients.get(identifier).copied()
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// Remember a client (replacing an earlier key for the same identifier)
    /// and rewrite the file. On a write error the client stays in memory.
    pub fn insert(&mut self, identifier: &[u8], ltpk: [u8; 32]) -> std::io::Result<()> {
        self.clients.insert(identifier.to_vec(), ltpk);
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut entries: Vec<_> = self.clients.iter().collect();
        entries.sort();
        let body: String = entries
            .into_iter()
            .map(|(id, key)| format!("{} {}\n", hex::encode(id), hex::encode(key)))
            .collect();
        // Flush the new list to disk before it replaces the old one, so a power
        // cut leaves one complete version.
        let tmp = path.with_extension("tmp");
        let mut file = fs::File::create(&tmp)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    }
}

fn parse_line(line: &str) -> Option<(Vec<u8>, [u8; 32])> {
    let mut parts = line.split_whitespace();
    let identifier = hex::decode(parts.next()?).ok()?;
    let ltpk = hex::decode(parts.next()?).ok()?.try_into().ok()?;
    if identifier.is_empty() || parts.next().is_some() {
        return None;
    }
    Some((identifier, ltpk))
}

/// Check the decrypted pair-setup M5 sub-TLV: the client signed
/// iOSDeviceX ‖ identifier ‖ LTPK with LTPK, where iOSDeviceX is derived from
/// the SRP session key. Returns the identifier and LTPK to store.
pub fn verify_setup_client(
    client_tlv: &BTreeMap<u8, Vec<u8>>,
    srp_session_key: &[u8],
) -> Result<(Vec<u8>, [u8; 32])> {
    let identifier = field(client_tlv, tags::IDENTIFIER, "identifier")?;
    let ltpk: [u8; 32] = field(client_tlv, tags::PUBLIC_KEY, "public key")?
        .as_slice()
        .try_into()
        .map_err(|_| Error::Protocol("client public key must be 32 bytes".into()))?;
    let signature = field(client_tlv, tags::SIGNATURE, "signature")?;

    let device_x = hkdf_expand(
        "Pair-Setup-Controller-Sign-Salt",
        "Pair-Setup-Controller-Sign-Info",
        srp_session_key,
    );
    let mut info = Vec::with_capacity(32 + identifier.len() + 32);
    info.extend_from_slice(&device_x);
    info.extend_from_slice(identifier);
    info.extend_from_slice(&ltpk);
    verify_signature(&ltpk, &info, signature)?;
    Ok((identifier.clone(), ltpk))
}

/// Check the decrypted pair-verify M3 sub-TLV: the identifier must be a
/// paired client, and its stored LTPK must verify the signature over
/// client_pub ‖ identifier ‖ server_pub (the two X25519 keys of this
/// pair-verify). Returns the identifier.
pub fn verify_known_client(
    client_tlv: &BTreeMap<u8, Vec<u8>>,
    client_pub: &[u8; 32],
    server_pub: &[u8; 32],
    clients: &PairedClients,
) -> Result<Vec<u8>> {
    let identifier = field(client_tlv, tags::IDENTIFIER, "identifier")?;
    let signature = field(client_tlv, tags::SIGNATURE, "signature")?;
    let ltpk = clients.get(identifier).ok_or_else(|| {
        Error::Protocol(format!(
            "unknown client {}",
            String::from_utf8_lossy(identifier)
        ))
    })?;
    let mut info = Vec::with_capacity(32 + identifier.len() + 32);
    info.extend_from_slice(client_pub);
    info.extend_from_slice(identifier);
    info.extend_from_slice(server_pub);
    verify_signature(&ltpk, &info, signature)?;
    Ok(identifier.clone())
}

fn field<'a>(tlv: &'a BTreeMap<u8, Vec<u8>>, tag: u8, name: &str) -> Result<&'a Vec<u8>> {
    tlv.get(&tag)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Error::Protocol(format!("client {name} missing")))
}

fn verify_signature(ltpk: &[u8; 32], message: &[u8], signature: &[u8]) -> Result<()> {
    let key = VerifyingKey::from_bytes(ltpk)
        .map_err(|_| Error::Crypto("invalid client public key".into()))?;
    let signature = Signature::from_slice(signature)
        .map_err(|_| Error::Crypto("malformed client signature".into()))?;
    key.verify_strict(message, &signature)
        .map_err(|_| Error::Crypto("client signature does not verify".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "atv-pairing-{name}-{}-{}",
            std::process::id(),
            rand::random::<u32>()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn paired_clients_survive_reopen() {
        let dir = temp_dir("reopen");
        let mut clients = PairedClients::open(Some(&dir));
        assert!(clients.is_empty());
        clients.insert(b"phone-a", [1; 32]).unwrap();
        clients.insert(b"phone-b", [2; 32]).unwrap();
        clients.insert(b"phone-a", [3; 32]).unwrap();

        let reopened = PairedClients::open(Some(&dir));
        assert_eq!(reopened.len(), 2);
        assert_eq!(reopened.get(b"phone-a"), Some([3; 32]));
        assert_eq!(reopened.get(b"phone-b"), Some([2; 32]));
        assert_eq!(reopened.get(b"phone-c"), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn setup_signature_must_cover_identifier_and_key() {
        let srp_key = [7u8; 64];
        let key = SigningKey::from_bytes(&[9; 32]);
        let ltpk = key.verifying_key().to_bytes();
        let device_x = hkdf_expand(
            "Pair-Setup-Controller-Sign-Salt",
            "Pair-Setup-Controller-Sign-Info",
            &srp_key,
        );
        let signature = key.sign(&[&device_x[..], &b"phone"[..], &ltpk[..]].concat());
        let mut tlv = BTreeMap::new();
        tlv.insert(tags::IDENTIFIER, b"phone".to_vec());
        tlv.insert(tags::PUBLIC_KEY, ltpk.to_vec());
        tlv.insert(tags::SIGNATURE, signature.to_bytes().to_vec());
        assert_eq!(
            verify_setup_client(&tlv, &srp_key).unwrap(),
            (b"phone".to_vec(), ltpk)
        );

        tlv.insert(tags::IDENTIFIER, b"other".to_vec());
        assert!(verify_setup_client(&tlv, &srp_key).is_err());
    }

    #[test]
    fn random_pin_has_four_digits() {
        for _ in 0..1000 {
            let pin = random_pin();
            assert!((1000..=9999).contains(&pin), "{pin}");
        }
    }
}

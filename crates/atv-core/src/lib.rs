//! atv-core: an Apple TV simulator speaking the Companion Link protocol.
//!
//! A real iPhone (Control Center "Apple TV Remote") discovers the server via
//! Bonjour, pairs with SRP (PIN), establishes an encrypted Companion session
//! and streams button/touch events, which are delivered to an
//! [`AtvDelegate`](delegate::AtvDelegate).
//!
//! Behavior is ported from the Python reference implementation
//! `fake_atv.py` (pyatv repository).

#![forbid(unsafe_code)]

pub mod crypto;
pub mod delegate;
pub mod error;
pub mod identity;
pub mod inspector;
pub mod opack;
pub mod pairing;
pub mod registry;
pub mod server;
pub mod session;
pub mod settings;
pub mod srp;
pub mod text_input;
pub mod tlv;

pub use delegate::{AtvDelegate, EventKind, TextOp, TouchPhase, TrackpadMode};
pub use error::{Error, Result};
pub use identity::DeviceIdentity;
pub use inspector::InspectorHub;
pub use server::{AtvConfig, AtvServer};
pub use settings::{UserSettings, DEFAULT_MOUSE_SPEED};

/// Companion frame types (first byte of the 4-byte frame header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Noop = 1,
    PsStart = 3,
    PsNext = 4,
    PvStart = 5,
    PvNext = 6,
    UOpack = 7,
    EOpack = 8,
    POpack = 9,
}

impl FrameType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Noop),
            3 => Some(Self::PsStart),
            4 => Some(Self::PsNext),
            5 => Some(Self::PvStart),
            6 => Some(Self::PvNext),
            7 => Some(Self::UOpack),
            8 => Some(Self::EOpack),
            9 => Some(Self::POpack),
            _ => None,
        }
    }
}

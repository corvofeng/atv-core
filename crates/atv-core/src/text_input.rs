//! Remote text input (RTI): typing on the phone's keyboard into the text field
//! focused on the TV.
//!
//! The phone asks with `_tiStart` whether a field is focused. A focused field
//! is described by `_tiD`, a keyed-archive binary plist that carries a session
//! UUID and the field's current text; with it the phone raises its keyboard.
//! Each edit then arrives as a `_tiC` event whose `_tiD` holds an
//! `RTITextOperations` archive. Focus changes on the TV are pushed to the
//! phones that sent `_tiStart` as `_tiStarted` / `_tiStopped` events.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use plist::{Dictionary, Uid, Value as Plist};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

use crate::delegate::TextOp;
use crate::error::{Error, Result};

/// A focus change the connection must forward to its phone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputPush {
    /// A field got focus: send `_tiStarted` with this `_tiD`.
    Started(Vec<u8>),
    /// The field lost focus: send `_tiStopped`.
    Stopped,
}

/// Build the `_tiD` that describes a focused text field.
///
/// The shape is the one the iPhone accepted in the keyboard probe: no
/// `$archiver` and no class entries, only the session UUID and
/// `documentState.docSt.contextBeforeInput`.
pub fn build_document_state(session_uuid: &[u8; 16], text: &str) -> Vec<u8> {
    let mut top = Dictionary::new();
    top.insert("sessionUUID".into(), Plist::Uid(Uid::new(1)));
    top.insert("documentState".into(), Plist::Uid(Uid::new(2)));

    let mut document_state = Dictionary::new();
    document_state.insert("docSt".into(), Plist::Uid(Uid::new(3)));
    let mut doc_st = Dictionary::new();
    doc_st.insert("contextBeforeInput".into(), Plist::Uid(Uid::new(4)));

    let objects = vec![
        Plist::String("$null".into()),
        Plist::Data(session_uuid.to_vec()),
        Plist::Dictionary(document_state),
        Plist::Dictionary(doc_st),
        Plist::String(text.into()),
    ];

    let mut root = Dictionary::new();
    root.insert("$top".into(), Plist::Dictionary(top));
    root.insert("$objects".into(), Plist::Array(objects));

    let mut out = Vec::new();
    Plist::Dictionary(root)
        .to_writer_binary(&mut out)
        .expect("writing a plist into a Vec cannot fail");
    out
}

/// Decode the `_tiD` of a `_tiC` event into text operations, in the order
/// they apply: replace the whole text, delete before the cursor, insert.
pub fn parse_text_operations(tid: &[u8]) -> Result<Vec<TextOp>> {
    let archive = Plist::from_reader(std::io::Cursor::new(tid))
        .map_err(|e| Error::Protocol(format!("_tiD is not a plist: {e}")))?;
    let archive = archive
        .as_dictionary()
        .ok_or_else(|| Error::Protocol("_tiD root is not a dictionary".into()))?;
    let objects = archive
        .get("$objects")
        .and_then(Plist::as_array)
        .ok_or_else(|| Error::Protocol("_tiD has no $objects".into()))?;
    let top = archive
        .get("$top")
        .and_then(Plist::as_dictionary)
        .ok_or_else(|| Error::Protocol("_tiD has no $top".into()))?;
    let operations = top
        .get("textOperations")
        .and_then(|v| resolve(objects, v))
        .and_then(Plist::as_dictionary)
        .ok_or_else(|| Error::Protocol("_tiD has no textOperations".into()))?;

    let mut ops = Vec::new();
    if let Some(text) = operations
        .get("textToAssert")
        .and_then(|v| resolve(objects, v))
        .and_then(Plist::as_string)
    {
        ops.push(TextOp::Replace(text.to_string()));
    }
    if let Some(output) = operations
        .get("keyboardOutput")
        .and_then(|v| resolve(objects, v))
        .and_then(Plist::as_dictionary)
    {
        let deletion = output
            .get("deletionCount")
            .and_then(|v| resolve(objects, v))
            .and_then(Plist::as_unsigned_integer)
            .unwrap_or(0);
        if deletion > 0 {
            ops.push(TextOp::Delete(u32::try_from(deletion).unwrap_or(u32::MAX)));
        }
        if let Some(text) = output
            .get("insertionText")
            .and_then(|v| resolve(objects, v))
            .and_then(Plist::as_string)
        {
            if !text.is_empty() {
                ops.push(TextOp::Insert(text.to_string()));
            }
        }
    }
    Ok(ops)
}

/// Follow a keyed-archive reference into `$objects`; UID 0 is `$null`.
fn resolve<'a>(objects: &'a [Plist], value: &'a Plist) -> Option<&'a Plist> {
    match value {
        Plist::Uid(uid) => match uid.get() {
            0 => None,
            index => objects.get(usize::try_from(index).ok()?),
        },
        other => Some(other),
    }
}

struct HubState {
    focused: bool,
    /// Host-chosen identity of the focused field.
    field: String,
    text: String,
    session_uuid: [u8; 16],
    clients: HashMap<u64, UnboundedSender<TextInputPush>>,
}

/// Focus state of the TV's text field, shared by all Companion connections.
pub struct TextInputHub {
    state: Mutex<HubState>,
}

impl TextInputHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(HubState {
                focused: false,
                field: String::new(),
                text: String::new(),
                session_uuid: [0; 16],
                clients: HashMap::new(),
            }),
        })
    }

    fn lock(&self) -> MutexGuard<'_, HubState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Record the focused field (`field` is any stable identity the host
    /// picks) and its text, and tell every connection that sent `_tiStart`.
    ///
    /// Gaining focus pushes `_tiStarted`, losing it pushes `_tiStopped`. A
    /// different field, or new text while the same field stays focused, pushes
    /// `_tiStopped` then `_tiStarted` with a fresh session so the phone reloads
    /// the document. `from_phone` marks text that only changed because the
    /// host applied [`TextOp`]s from the phone: it is recorded silently, since
    /// the phone already shows it.
    pub fn set_focus(&self, focused: bool, field: &str, text: &str, from_phone: bool) {
        let mut state = self.lock();
        let was_focused = state.focused;
        let reload = was_focused
            && focused
            && (state.field != field || (state.text != text && !from_phone));
        state.focused = focused;
        state.field = field.to_string();
        state.text = text.to_string();

        let mut pushes = Vec::new();
        if was_focused && (!focused || reload) {
            pushes.push(TextInputPush::Stopped);
        }
        if focused && (!was_focused || reload) {
            state.session_uuid = rand::random();
            pushes.push(TextInputPush::Started(build_document_state(
                &state.session_uuid,
                &state.text,
            )));
        }
        if pushes.is_empty() {
            return;
        }
        info!(
            "text input focus={focused} reload={reload}, notifying {} connection(s)",
            state.clients.len()
        );
        state
            .clients
            .retain(|_, tx| pushes.iter().all(|push| tx.send(push.clone()).is_ok()));
    }

    /// A connection sent `_tiStart`: remember it and return the `_tiD` to
    /// answer with, or `None` when no field is focused.
    pub fn subscribe(&self, conn_id: u64, tx: UnboundedSender<TextInputPush>) -> Option<Vec<u8>> {
        let mut state = self.lock();
        state.clients.insert(conn_id, tx);
        state
            .focused
            .then(|| build_document_state(&state.session_uuid, &state.text))
    }

    /// The connection sent `_tiStop` or went away.
    pub fn unsubscribe(&self, conn_id: u64) {
        self.lock().clients.remove(&conn_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::{error::TryRecvError, unbounded_channel, UnboundedReceiver};

    // Real `_tiC` payloads from an iPhone, recorded by the keyboard probe.
    const TIC_INSERT_CYRILLIC: &[u8] = include_bytes!("../testdata/tic_insert_cyrillic.bplist");
    const TIC_DELETE_ONE: &[u8] = include_bytes!("../testdata/tic_delete_one.bplist");
    const TIC_ASSERT_TEXT: &[u8] = include_bytes!("../testdata/tic_assert_text.bplist");
    // The `_tiStart` answer that made the iPhone raise its keyboard.
    const TISTART_PROBE_REPLY: &[u8] = include_bytes!("../testdata/tistart_probe_reply.bplist");

    fn parse(bytes: &[u8]) -> Plist {
        Plist::from_reader(std::io::Cursor::new(bytes)).unwrap()
    }

    /// The session UUID and `contextBeforeInput` of a `_tiD`.
    fn document(tid: &[u8]) -> (Vec<u8>, String) {
        let archive = parse(tid);
        let archive = archive.as_dictionary().unwrap();
        let objects = archive["$objects"].as_array().unwrap();
        let top = archive["$top"].as_dictionary().unwrap();
        let uuid = resolve(objects, &top["sessionUUID"]).unwrap().as_data().unwrap();
        let doc = resolve(objects, &top["documentState"]).unwrap().as_dictionary().unwrap();
        let doc_st = resolve(objects, &doc["docSt"]).unwrap().as_dictionary().unwrap();
        let text = resolve(objects, &doc_st["contextBeforeInput"])
            .unwrap()
            .as_string()
            .unwrap();
        (uuid.to_vec(), text.to_string())
    }

    fn context_before_input(tid: &[u8]) -> String {
        document(tid).1
    }

    fn expect_started(rx: &mut UnboundedReceiver<TextInputPush>) -> (Vec<u8>, String) {
        match rx.try_recv() {
            Ok(TextInputPush::Started(tid)) => document(&tid),
            other => panic!("expected Started, got {other:?}"),
        }
    }

    #[test]
    fn document_state_matches_probe_reply() {
        let probe = parse(TISTART_PROBE_REPLY);
        let uuid: [u8; 16] = probe.as_dictionary().unwrap()["$objects"].as_array().unwrap()[1]
            .as_data()
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(parse(&build_document_state(&uuid, "")), probe);
    }

    #[test]
    fn document_state_carries_current_text() {
        let tid = build_document_state(&[7; 16], "привет");
        assert_eq!(context_before_input(&tid), "привет");
    }

    #[test]
    fn parses_cyrillic_insertion() {
        assert_eq!(
            parse_text_operations(TIC_INSERT_CYRILLIC).unwrap(),
            vec![TextOp::Insert("п".into())]
        );
    }

    #[test]
    fn parses_backspace() {
        assert_eq!(parse_text_operations(TIC_DELETE_ONE).unwrap(), vec![TextOp::Delete(1)]);
    }

    #[test]
    fn parses_text_to_assert() {
        assert_eq!(
            parse_text_operations(TIC_ASSERT_TEXT).unwrap(),
            vec![TextOp::Replace("привет ".into())]
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_text_operations(b"not a plist").is_err());
    }

    #[test]
    fn hub_pushes_focus_changes_to_subscribers_only() {
        let hub = TextInputHub::new();
        let (tx, mut rx) = unbounded_channel();
        let (other_tx, mut other_rx) = unbounded_channel();

        // Unfocused: `_tiStart` gets no document, the connection is remembered.
        assert_eq!(hub.subscribe(1, tx), None);
        hub.set_focus(true, "search", "ab", false);
        assert_eq!(expect_started(&mut rx).1, "ab");

        // Text that changed because of the phone's own edits is not pushed back.
        hub.set_focus(true, "search", "abc", true);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

        // A connection that starts while focused gets the current text.
        let tid = hub.subscribe(2, other_tx).unwrap();
        assert_eq!(context_before_input(&tid), "abc");

        hub.unsubscribe(2);
        hub.set_focus(false, "", "", false);
        assert_eq!(rx.try_recv(), Ok(TextInputPush::Stopped));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(other_rx.try_recv(), Err(TryRecvError::Disconnected));
    }

    #[test]
    fn hub_reloads_document_on_field_switch_and_tv_side_edit() {
        let hub = TextInputHub::new();
        let (tx, mut rx) = unbounded_channel();
        assert_eq!(hub.subscribe(1, tx), None);

        hub.set_focus(true, "login", "me", false);
        let (login_session, _) = expect_started(&mut rx);

        // Focus moves to another field while the TV keyboard stays up.
        hub.set_focus(true, "query", "cats", false);
        assert_eq!(rx.try_recv(), Ok(TextInputPush::Stopped));
        let (query_session, text) = expect_started(&mut rx);
        assert_eq!(text, "cats");
        assert_ne!(query_session, login_session);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

        // The app clears the field after a search: the phone gets the new text.
        hub.set_focus(true, "query", "", false);
        assert_eq!(rx.try_recv(), Ok(TextInputPush::Stopped));
        let (cleared_session, text) = expect_started(&mut rx);
        assert_eq!(text, "");
        assert_ne!(cleared_session, query_session);

        // Reporting the same state again pushes nothing.
        hub.set_focus(true, "query", "", false);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }
}

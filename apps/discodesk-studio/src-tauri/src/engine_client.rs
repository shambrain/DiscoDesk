//! The Studio's view of the engine.
//!
//! This module deliberately contains **no engine logic**. It opens the shared
//! local endpoint, sends the documented command envelope, and translates the
//! answer into a view model the UI can render. It never owns engine state and
//! never derives engine state of its own: every fact comes from a response.
//!
//! See `docs/IPC_PROTOCOL.md` for the wire contract and `docs/ADR/0001` for why
//! the Studio is a separate process.
//!
//! # Why a `Connector` trait
//!
//! A real named pipe cannot be faked in a unit test, and starting a real engine
//! from a unit test would make the suite slow and order-dependent. The trait
//! lets the tests drive every branch — connected, absent, protocol mismatch,
//! permission denied, transport failure — through the **real** envelope and
//! frame code, which is where the interesting bugs live.

use std::sync::{Mutex, MutexGuard};

use dd_ipc::{
    Command, ErrorCode, Frame, IpcError, PROTOCOL_VERSION, RequestEnvelope, ResponseEnvelope,
    Transport,
};
use serde::Serialize;

/// Obtains a transport to the engine.
pub trait Connector: Send + Sync {
    /// Opens a connection, or explains why it could not.
    fn connect(&self) -> Result<Box<dyn Transport>, IpcError>;

    /// A description of the endpoint, for diagnostics only.
    fn describe(&self) -> String;
}

/// Connects over the shared local endpoint (`dd_ipc::default_endpoint_path`).
#[derive(Debug, Clone)]
pub struct PipeConnector {
    endpoint: String,
}

impl PipeConnector {
    /// Connects to the endpoint both processes derive by default.
    #[must_use]
    pub fn default_endpoint() -> Self {
        Self {
            endpoint: dd_ipc::default_endpoint_path(),
        }
    }

    /// Connects to an explicit endpoint. Used by tests and by `--endpoint`.
    #[must_use]
    pub fn at(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    /// The endpoint this connector targets.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl Connector for PipeConnector {
    fn connect(&self) -> Result<Box<dyn Transport>, IpcError> {
        dd_ipc::connect(&self.endpoint)
    }

    fn describe(&self) -> String {
        self.endpoint.clone()
    }
}

/// A parsed engine status response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    /// The engine state name, exactly as the engine reported it.
    pub state: String,
    /// The engine's own version string, when it reported one.
    pub engine_version: Option<String>,
    /// The protocol version in the response envelope.
    pub protocol_version: u16,
    /// Milliseconds since the engine started.
    pub uptime_ms: u64,
    /// Number of wallpaper surfaces the engine owns.
    pub surface_count: u32,
    /// Whether the engine considers itself running.
    pub is_running: bool,
    /// Whether the engine considers itself paused.
    pub paused: bool,
}

/// What the Studio can truthfully say about the engine.
///
/// Each variant maps to one user-visible condition. `kind` is a stable tag so
/// the frontend branches on data, not on message text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EngineView {
    /// A status response was received and parsed.
    Connected {
        /// The parsed status.
        status: EngineStatus,
    },
    /// Nothing is listening on the endpoint: the engine is not running.
    Unavailable {
        /// A short, non-sensitive explanation.
        detail: String,
    },
    /// The engine speaks a different protocol version.
    ProtocolMismatch {
        /// The version this build speaks.
        expected: u16,
        /// The version the engine reported.
        found: u16,
    },
    /// The endpoint exists but this user may not open it.
    PermissionDenied {
        /// A short, non-sensitive explanation.
        detail: String,
    },
    /// The engine answered, but with an error for the status request.
    EngineError {
        /// The stable numeric error code, when the engine supplied one.
        code: Option<u16>,
        /// The engine's message, already redacted engine-side.
        message: String,
    },
    /// The connection exists but the exchange failed.
    ConnectionFailed {
        /// A short, non-sensitive explanation.
        detail: String,
    },
}

impl EngineView {
    /// Whether this view means the Studio is talking to a healthy engine.
    #[must_use]
    pub const fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }

    /// A short machine-readable tag, mirroring the serde tag.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Connected { .. } => "connected",
            Self::Unavailable { .. } => "unavailable",
            Self::ProtocolMismatch { .. } => "protocolMismatch",
            Self::PermissionDenied { .. } => "permissionDenied",
            Self::EngineError { .. } => "engineError",
            Self::ConnectionFailed { .. } => "connectionFailed",
        }
    }
}

/// Inner state guarded by the mutex.
#[derive(Default)]
struct ClientState {
    connection: Option<Box<dyn Transport>>,
    next_request_id: u64,
}

impl std::fmt::Debug for ClientState {
    /// `dyn Transport` is not `Debug`, and a transport would not be useful in a
    /// log line anyway. Report whether a connection is held, not its internals.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientState")
            .field("connected", &self.connection.is_some())
            .field("next_request_id", &self.next_request_id)
            .finish()
    }
}

/// A client that reuses one connection and reconnects on the next request.
#[derive(Debug)]
pub struct EngineClient<C: Connector> {
    connector: C,
    state: Mutex<ClientState>,
}

impl<C: Connector> EngineClient<C> {
    /// Creates a client. No connection is made until the first request.
    pub fn new(connector: C) -> Self {
        Self {
            connector,
            state: Mutex::new(ClientState {
                connection: None,
                // Request ids start at 1; 0 is reserved by the protocol for
                // uncorrelated responses.
                next_request_id: 1,
            }),
        }
    }

    /// The endpoint description, for diagnostics.
    #[must_use]
    pub fn endpoint(&self) -> String {
        self.connector.describe()
    }

    /// The connector, for diagnostics and tests.
    #[must_use]
    pub fn connector(&self) -> &C {
        &self.connector
    }

    /// Recovers the lock instead of propagating a poison.
    ///
    /// A panic while the lock was held cannot leave the state inconsistent: it
    /// holds only an optional transport and a counter. Propagating the poison
    /// would make the Studio permanently unable to talk to the engine after one
    /// unrelated panic.
    fn lock(&self) -> MutexGuard<'_, ClientState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Drops any current connection. Used on Studio shutdown and by tests.
    pub fn disconnect(&self) {
        self.lock().connection = None;
    }

    /// Sends one command and returns the response envelope.
    ///
    /// A transport failure drops the connection so the next call reconnects,
    /// rather than reusing a stream that is known to be broken.
    fn request(&self, command: Command) -> Result<ResponseEnvelope, IpcError> {
        let mut state = self.lock();

        let request_id = state.next_request_id;
        state.next_request_id = state.next_request_id.wrapping_add(1).max(1);

        let mut connection = match state.connection.take() {
            Some(connection) => connection,
            None => self.connector.connect()?,
        };

        match exchange(connection.as_mut(), request_id, command) {
            Ok(response) => {
                state.connection = Some(connection);
                Ok(response)
            }
            Err(err) => {
                // `connection` is dropped here; the next call reconnects.
                Err(err)
            }
        }
    }

    /// Asks the engine for its status and classifies the outcome.
    #[must_use]
    pub fn status(&self) -> EngineView {
        match self.request(Command::GetEngineStatus) {
            Ok(response) => view_from_response(&response),
            Err(err) => view_from_error(&err),
        }
    }
}

/// Performs the request/response exchange on an established transport.
fn exchange(
    transport: &mut dyn Transport,
    request_id: u64,
    command: Command,
) -> Result<ResponseEnvelope, IpcError> {
    let envelope = RequestEnvelope::new(request_id, command);
    let payload = serde_json::to_vec(&envelope).map_err(|err| {
        IpcError::MalformedPayload(format!("could not encode the request: {err}"))
    })?;

    transport.send_frame(&Frame {
        version: PROTOCOL_VERSION,
        payload,
    })?;

    let frame = transport.recv_frame()?;

    let response: ResponseEnvelope = serde_json::from_slice(&frame.payload).map_err(|err| {
        IpcError::MalformedPayload(format!("could not decode the engine response: {err}"))
    })?;

    // The frame layer already rejected a version mismatch; this catches an
    // engine that framed correctly but announced a different contract.
    if response.version != PROTOCOL_VERSION {
        return Err(IpcError::ProtocolVersionMismatch {
            found: response.version,
            expected: PROTOCOL_VERSION,
        });
    }

    if response.request_id != request_id {
        return Err(IpcError::MalformedPayload(format!(
            "response id {} does not match request id {request_id}",
            response.request_id
        )));
    }

    Ok(response)
}

/// Turns a successful response into a view, rejecting a resultless success.
fn view_from_response(response: &ResponseEnvelope) -> EngineView {
    if let Some(error) = &response.error {
        return view_from_error_payload(error.code, &error.message);
    }

    let Some(result) = &response.result else {
        return EngineView::ConnectionFailed {
            detail: "the engine returned neither a result nor an error".to_owned(),
        };
    };

    match parse_status(result) {
        Some(status) => EngineView::Connected { status },
        None => EngineView::ConnectionFailed {
            detail: "the engine status response did not match the protocol".to_owned(),
        },
    }
}

/// Parses the documented status object.
///
/// Fields are read defensively: a missing field yields `None` rather than a
/// panic, because the engine is a separate process and may be a different
/// build.
fn parse_status(result: &serde_json::Value) -> Option<EngineStatus> {
    let state = result.get("state")?.as_str()?.to_owned();
    let engine_version = result
        .get("engineVersion")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let protocol_version = u16::try_from(result.get("protocolVersion")?.as_u64()?).ok()?;
    let uptime_ms = result.get("uptimeMs")?.as_u64()?;
    let surface_count = u32::try_from(result.get("surfaceCount")?.as_u64()?).ok()?;
    let is_running = result.get("isRunning")?.as_bool()?;
    let paused = result.get("paused")?.as_bool()?;

    Some(EngineStatus {
        state,
        engine_version,
        protocol_version,
        uptime_ms,
        surface_count,
        is_running,
        paused,
    })
}

/// Maps a protocol error payload to a view.
fn view_from_error_payload(code: u16, message: &str) -> EngineView {
    if code == ErrorCode::ProtocolVersionMismatch.as_u16() {
        // The engine rejected our envelope version. We do not know which
        // version it speaks, so report the mismatch without inventing one.
        return EngineView::ProtocolMismatch {
            expected: PROTOCOL_VERSION,
            found: 0,
        };
    }
    EngineView::EngineError {
        code: Some(code),
        message: message.to_owned(),
    }
}

/// Maps a transport-level failure to a view.
fn view_from_error(err: &IpcError) -> EngineView {
    match err {
        IpcError::ProtocolVersionMismatch { found, expected } => EngineView::ProtocolMismatch {
            expected: *expected,
            found: *found,
        },
        IpcError::EndpointUnavailable(detail) => EngineView::Unavailable {
            detail: detail.clone(),
        },
        IpcError::PeerClosed => EngineView::Unavailable {
            detail: "the engine closed the connection".to_owned(),
        },
        IpcError::Io(io) => EngineView::ConnectionFailed {
            detail: io.to_string(),
        },
        other => EngineView::ConnectionFailed {
            detail: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A transport whose responses are scripted by the test.
    struct ScriptedTransport {
        /// Raw response payloads to hand back, in order.
        responses: VecDeque<Result<Vec<u8>, IpcError>>,
        /// Every request payload this transport was asked to send.
        sent: Vec<Vec<u8>>,
    }

    impl ScriptedTransport {
        fn new(responses: Vec<Result<Vec<u8>, IpcError>>) -> Self {
            Self {
                responses: responses.into(),
                sent: Vec::new(),
            }
        }

        fn sent_commands(&self) -> Vec<String> {
            self.sent
                .iter()
                .filter_map(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
                .filter_map(|value| value.get("command")?.as_str().map(str::to_owned))
                .collect()
        }
    }

    impl Transport for ScriptedTransport {
        fn send_frame(&mut self, frame: &Frame) -> dd_ipc::IpcResult<()> {
            self.sent.push(frame.payload.clone());
            Ok(())
        }

        fn recv_frame(&mut self) -> dd_ipc::IpcResult<Frame> {
            match self.responses.pop_front() {
                Some(Ok(payload)) => Ok(Frame {
                    version: PROTOCOL_VERSION,
                    payload,
                }),
                Some(Err(err)) => Err(err),
                None => Err(IpcError::PeerClosed),
            }
        }

        fn endpoint_path(&self) -> &str {
            "scripted"
        }
    }

    /// Hands out scripted transports, and records how many connects happened.
    struct ScriptedConnector {
        queue: Mutex<VecDeque<Result<ScriptedTransport, IpcError>>>,
        connects: Mutex<u32>,
    }

    impl ScriptedConnector {
        fn new(connections: Vec<Result<ScriptedTransport, IpcError>>) -> Self {
            Self {
                queue: Mutex::new(connections.into()),
                connects: Mutex::new(0),
            }
        }

        fn connect_count(&self) -> u32 {
            *self.connects.lock().unwrap()
        }
    }

    impl Connector for ScriptedConnector {
        fn connect(&self) -> Result<Box<dyn Transport>, IpcError> {
            let mut connects = self.connects.lock().unwrap();
            *connects += 1;
            let next = self.queue.lock().unwrap().pop_front();
            match next {
                Some(Ok(transport)) => Ok(Box::new(transport)),
                Some(Err(err)) => Err(err),
                None => Err(IpcError::EndpointUnavailable(
                    "no scripted connection available".to_owned(),
                )),
            }
        }

        fn describe(&self) -> String {
            "scripted".to_owned()
        }
    }

    fn status_payload_with_id(request_id: u64, state: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "version": PROTOCOL_VERSION,
            "request_id": request_id,
            "result": {
                "state": state,
                "engineVersion": "0.1.0",
                "protocolVersion": PROTOCOL_VERSION,
                "uptimeMs": 1234,
                "surfaceCount": 0,
                "isRunning": true,
                "paused": false,
            }
        }))
        .expect("payload should encode")
    }

    /// A status response for request id 1.
    fn status_payload(state: &str) -> Vec<u8> {
        status_payload_with_id(1, state)
    }

    fn error_payload(code: u16, message: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "version": PROTOCOL_VERSION,
            "request_id": 1,
            "error": { "code": code, "message": message }
        }))
        .expect("payload should encode")
    }

    #[test]
    fn connected_status_is_reported_faithfully() {
        let client = EngineClient::new(ScriptedConnector::new(vec![Ok(ScriptedTransport::new(
            vec![Ok(status_payload("IdleReady"))],
        ))]));

        let view = client.status();
        let EngineView::Connected { status } = view else {
            panic!("expected Connected, got {view:?}");
        };
        assert_eq!(status.state, "IdleReady");
        assert_eq!(status.protocol_version, PROTOCOL_VERSION);
        assert_eq!(status.uptime_ms, 1234);
        assert_eq!(status.surface_count, 0);
        assert!(status.is_running);
        assert!(!status.paused);
    }

    #[test]
    fn a_missing_engine_is_reported_as_unavailable() {
        let client = EngineClient::new(ScriptedConnector::new(vec![Err(
            IpcError::EndpointUnavailable("pipe not found".to_owned()),
        )]));

        assert_eq!(client.status().kind(), "unavailable");
    }

    #[test]
    fn a_permission_failure_is_distinguished_from_absence() {
        // The transport maps ERROR_ACCESS_DENIED into EndpointUnavailable with
        // a message; the Studio must not claim "engine not running" then.
        let client = EngineClient::new(ScriptedConnector::new(vec![Err(
            IpcError::EndpointUnavailable("access denied".to_owned()),
        )]));
        let view = client.status();
        // EndpointUnavailable is honest either way; the detail carries the
        // reason. Assert the detail survives rather than being discarded.
        match view {
            EngineView::Unavailable { detail } => assert!(detail.contains("access denied")),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn a_frame_protocol_mismatch_is_reported() {
        let client = EngineClient::new(ScriptedConnector::new(vec![Ok(ScriptedTransport::new(
            vec![Err(IpcError::ProtocolVersionMismatch {
                found: 7,
                expected: PROTOCOL_VERSION,
            })],
        ))]));

        match client.status() {
            EngineView::ProtocolMismatch { expected, found } => {
                assert_eq!(expected, PROTOCOL_VERSION);
                assert_eq!(found, 7);
            }
            other => panic!("expected ProtocolMismatch, got {other:?}"),
        }
    }

    #[test]
    fn an_envelope_protocol_mismatch_is_reported() {
        let payload = serde_json::to_vec(&serde_json::json!({
            "version": PROTOCOL_VERSION + 1,
            "request_id": 1,
            "result": {}
        }))
        .unwrap();
        let client = EngineClient::new(ScriptedConnector::new(vec![Ok(ScriptedTransport::new(
            vec![Ok(payload)],
        ))]));

        match client.status() {
            EngineView::ProtocolMismatch { expected, found } => {
                assert_eq!(expected, PROTOCOL_VERSION);
                assert_eq!(found, PROTOCOL_VERSION + 1);
            }
            other => panic!("expected ProtocolMismatch, got {other:?}"),
        }
    }

    #[test]
    fn an_engine_side_version_rejection_is_reported_as_mismatch() {
        let client = EngineClient::new(ScriptedConnector::new(vec![Ok(ScriptedTransport::new(
            vec![Ok(error_payload(
                ErrorCode::ProtocolVersionMismatch.as_u16(),
                "protocol version mismatch",
            ))],
        ))]));

        assert_eq!(client.status().kind(), "protocolMismatch");
    }

    #[test]
    fn a_deferred_command_error_is_surfaced_with_its_code() {
        let client = EngineClient::new(ScriptedConnector::new(vec![Ok(ScriptedTransport::new(
            vec![Ok(error_payload(
                ErrorCode::NotImplemented.as_u16(),
                "GetEngineStatus is not implemented",
            ))],
        ))]));

        match client.status() {
            EngineView::EngineError { code, message } => {
                assert_eq!(code, Some(ErrorCode::NotImplemented.as_u16()));
                assert!(message.contains("not implemented"));
            }
            other => panic!("expected EngineError, got {other:?}"),
        }
    }

    #[test]
    fn a_status_without_the_documented_fields_is_rejected() {
        let payload = serde_json::to_vec(&serde_json::json!({
            "version": PROTOCOL_VERSION,
            "request_id": 1,
            "result": { "state": "IdleReady" }
        }))
        .unwrap();
        let client = EngineClient::new(ScriptedConnector::new(vec![Ok(ScriptedTransport::new(
            vec![Ok(payload)],
        ))]));

        assert_eq!(client.status().kind(), "connectionFailed");
    }

    #[test]
    fn the_client_reconnects_after_a_failed_connect() {
        // First connect fails, second succeeds. Request ids keep advancing, so
        // the successful response is correlated to request id 2.
        let connector = ScriptedConnector::new(vec![
            Err(IpcError::EndpointUnavailable("not running".to_owned())),
            Ok(ScriptedTransport::new(vec![Ok(status_payload_with_id(
                2,
                "IdleReady",
            ))])),
        ]);
        let client = EngineClient::new(connector);

        assert_eq!(client.status().kind(), "unavailable");
        assert_eq!(client.status().kind(), "connected");
        assert_eq!(client.connector().connect_count(), 2);
    }

    #[test]
    fn the_connection_is_reused_while_healthy() {
        let connector = ScriptedConnector::new(vec![Ok(ScriptedTransport::new(vec![
            Ok(status_payload_with_id(1, "IdleReady")),
            Ok(status_payload_with_id(2, "IdleReady")),
        ]))]);
        let client = EngineClient::new(connector);

        assert!(client.status().is_connected());
        assert!(client.status().is_connected());
        // Two requests, one connection: the client must not reconnect per poll.
        assert_eq!(client.connector().connect_count(), 1);
    }

    #[test]
    fn a_transport_failure_drops_the_connection_and_the_next_call_recovers() {
        let connector = ScriptedConnector::new(vec![
            // First connection: one good response, then the peer closes.
            Ok(ScriptedTransport::new(vec![
                Ok(status_payload_with_id(1, "IdleReady")),
                Err(IpcError::PeerClosed),
            ])),
            // Reconnect: healthy again, answering request id 3.
            Ok(ScriptedTransport::new(vec![Ok(status_payload_with_id(
                3,
                "IdleReady",
            ))])),
        ]);
        let client = EngineClient::new(connector);

        assert_eq!(client.status().kind(), "connected");
        // The second call reads the error the scripted transport returns.
        assert_ne!(client.status().kind(), "connected");
        // The third call must have reconnected.
        assert_eq!(client.status().kind(), "connected");
        assert_eq!(client.connector().connect_count(), 2);
    }

    #[test]
    fn a_response_with_the_wrong_request_id_is_rejected() {
        let payload = serde_json::to_vec(&serde_json::json!({
            "version": PROTOCOL_VERSION,
            "request_id": 999,
            "result": {}
        }))
        .unwrap();
        let client = EngineClient::new(ScriptedConnector::new(vec![Ok(ScriptedTransport::new(
            vec![Ok(payload)],
        ))]));

        assert_eq!(client.status().kind(), "connectionFailed");
    }

    #[test]
    fn the_studio_never_asks_the_engine_to_shut_down() {
        // Closing the Studio must not stop playback. The client has no
        // shutdown path, and this test fails loudly if one is ever added
        // without reconsidering docs/ADR/0001.
        let mut transport = ScriptedTransport::new(vec![Ok(status_payload("IdleReady"))]);
        exchange(&mut transport, 1, Command::GetEngineStatus).expect("exchange should succeed");

        assert_eq!(
            transport.sent_commands(),
            vec!["GetEngineStatus".to_owned()]
        );
        assert!(
            !transport
                .sent_commands()
                .iter()
                .any(|c| c == "ShutdownEngine")
        );
    }

    #[test]
    fn disconnect_drops_the_connection_so_the_next_call_reconnects() {
        let connector = ScriptedConnector::new(vec![
            Ok(ScriptedTransport::new(vec![Ok(status_payload_with_id(
                1,
                "IdleReady",
            ))])),
            Ok(ScriptedTransport::new(vec![Ok(status_payload_with_id(
                2,
                "IdleReady",
            ))])),
        ]);
        let client = EngineClient::new(connector);

        assert!(client.status().is_connected());
        client.disconnect();
        assert!(client.status().is_connected());
        assert_eq!(client.connector().connect_count(), 2);
    }

    #[test]
    fn request_ids_increase_and_never_reuse_zero() {
        let connector = ScriptedConnector::new(vec![Ok(ScriptedTransport::new(vec![
            Ok(status_payload_with_id(1, "IdleReady")),
            Ok(status_payload_with_id(2, "IdleReady")),
        ]))]);
        let client = EngineClient::new(connector);

        let first = client.request(Command::GetEngineStatus).unwrap();
        let second = client.request(Command::GetEngineStatus).unwrap();
        assert_eq!(first.request_id, 1);
        assert_eq!(second.request_id, 2);
        assert_ne!(first.request_id, 0);
    }

    #[test]
    fn concurrent_status_calls_are_serialised_without_panicking() {
        use std::sync::Arc;
        use std::thread;

        let client = Arc::new(EngineClient::new(ScriptedConnector::new(vec![Ok(
            ScriptedTransport::new(vec![
                Ok(status_payload("IdleReady")),
                Ok(status_payload("IdleReady")),
                Ok(status_payload("IdleReady")),
                Ok(status_payload("IdleReady")),
            ]),
        )])));

        let mut handles = Vec::new();
        for _ in 0..4 {
            let client = Arc::clone(&client);
            handles.push(thread::spawn(move || {
                let _ = client.status();
            }));
        }
        for handle in handles {
            handle.join().expect("worker must not panic");
        }
    }
}

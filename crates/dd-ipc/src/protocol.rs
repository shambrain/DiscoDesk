//! The versioned engine/Studio command protocol.
//!
//! # Scope
//!
//! Gate 0 defines the **contract** only. The command set mirrors the message
//! list in `ARCHITECTURE.md` §4, but every payload is an honest, minimal,
//! *validated* struct. No subsystem behind these commands exists yet: the
//! engine answers them with a structured error that says so, rather than
//! pretending the work was done.
//!
//! # Serialization
//!
//! Envelopes are JSON (`serde_json`) because they are low-rate control traffic
//! — settings, state, telemetry snapshots. The *frame* codec in
//! [`crate::frame`] is hand-rolled so the hot path does not pay for a
//! JSON round-trip on the header.
//!
//! # Validation
//!
//! Every inbound envelope is validated before it reaches any handler:
//!
//! * unknown command names are rejected ([`Command::from_name`]),
//! * unknown enum values are rejected rather than coerced,
//! * strings longer than the documented limits are rejected,
//! * paths are checked for length and for absolute/relative shape,
//! * numeric fields are range-checked.
//!
//! Path *content* is not validated against the file system here. The engine
//! resolves and opens paths in the owning subsystem, under the same
//! permissions as the process, and reports failures as
//! [`crate::error::ErrorCode::NotFound`] or
//! [`crate::error::ErrorCode::PermissionDenied`].

use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, IpcError};

/// The protocol version spoken by this build.
///
/// Incrementing this value invalidates every older peer. The engine rejects a
/// frame whose version differs with
/// [`crate::error::ErrorCode::ProtocolVersionMismatch`].
pub const PROTOCOL_VERSION: u16 = 1;

/// Maximum length of a free-form identifier or path string, in bytes.
pub const MAX_STRING_BYTES: usize = 1024;

/// Maximum length of a display-name style string, in bytes.
pub const MAX_NAME_BYTES: usize = 256;

/// The command set of the protocol.
///
/// This enum is `#[non_exhaustive]`: new commands may be added in a later
/// protocol revision. A decoder must therefore reject an unknown name rather
/// than silently ignoring it, which [`Command::from_name`] does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Command {
    /// Ask for the current engine state.
    GetEngineStatus,
    /// Ask for the platform capability matrix.
    GetPlatformCapabilities,
    /// Ask for the list of selectable audio sources.
    GetAudioSources,
    /// Select the audio source.
    SetAudioSource,
    /// Apply a vibe preset.
    SetVibePreset,
    /// Change individual effect settings.
    SetEffectSettings,
    /// Import a wallpaper for analysis.
    ImportWallpaper,
    /// Cancel a running job.
    CancelJob,
    /// Apply a wallpaper package.
    ApplyPackage,
    /// Pause the engine.
    Pause,
    /// Resume the engine.
    Resume,
    /// Toggle the paused state.
    TogglePause,
    /// Change the performance policy.
    SetPerformancePolicy,
    /// Ask for a diagnostics snapshot.
    GetDiagnosticsSnapshot,
    /// Ask the engine to shut down.
    ShutdownEngine,
}

impl Command {
    /// The wire name of this command.
    ///
    /// These names are part of the protocol and must not change within a
    /// protocol version.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GetEngineStatus => "GetEngineStatus",
            Self::GetPlatformCapabilities => "GetPlatformCapabilities",
            Self::GetAudioSources => "GetAudioSources",
            Self::SetAudioSource => "SetAudioSource",
            Self::SetVibePreset => "SetVibePreset",
            Self::SetEffectSettings => "SetEffectSettings",
            Self::ImportWallpaper => "ImportWallpaper",
            Self::CancelJob => "CancelJob",
            Self::ApplyPackage => "ApplyPackage",
            Self::Pause => "Pause",
            Self::Resume => "Resume",
            Self::TogglePause => "TogglePause",
            Self::SetPerformancePolicy => "SetPerformancePolicy",
            Self::GetDiagnosticsSnapshot => "GetDiagnosticsSnapshot",
            Self::ShutdownEngine => "ShutdownEngine",
        }
    }

    /// Parses a wire name into a command.
    ///
    /// Unknown names are an error, never a default. This is what makes the
    /// `#[non_exhaustive]` marker safe on the wire.
    pub fn from_name(name: &str) -> Result<Self, IpcError> {
        match name {
            "GetEngineStatus" => Ok(Self::GetEngineStatus),
            "GetPlatformCapabilities" => Ok(Self::GetPlatformCapabilities),
            "GetAudioSources" => Ok(Self::GetAudioSources),
            "SetAudioSource" => Ok(Self::SetAudioSource),
            "SetVibePreset" => Ok(Self::SetVibePreset),
            "SetEffectSettings" => Ok(Self::SetEffectSettings),
            "ImportWallpaper" => Ok(Self::ImportWallpaper),
            "CancelJob" => Ok(Self::CancelJob),
            "ApplyPackage" => Ok(Self::ApplyPackage),
            "Pause" => Ok(Self::Pause),
            "Resume" => Ok(Self::Resume),
            "TogglePause" => Ok(Self::TogglePause),
            "SetPerformancePolicy" => Ok(Self::SetPerformancePolicy),
            "GetDiagnosticsSnapshot" => Ok(Self::GetDiagnosticsSnapshot),
            "ShutdownEngine" => Ok(Self::ShutdownEngine),
            other => Err(IpcError::UnknownCommand(other.to_owned())),
        }
    }

    /// Every command in the protocol.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::GetEngineStatus,
            Self::GetPlatformCapabilities,
            Self::GetAudioSources,
            Self::SetAudioSource,
            Self::SetVibePreset,
            Self::SetEffectSettings,
            Self::ImportWallpaper,
            Self::CancelJob,
            Self::ApplyPackage,
            Self::Pause,
            Self::Resume,
            Self::TogglePause,
            Self::SetPerformancePolicy,
            Self::GetDiagnosticsSnapshot,
            Self::ShutdownEngine,
        ]
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The audio source selection mode.
///
/// Unknown values are rejected by [`AudioSourceMode::from_u8`]; there is no
/// "default" mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AudioSourceMode {
    /// Let the engine choose, subject to capability probing.
    Auto,
    /// Use the system mix.
    SystemMix,
    /// Use one specific application.
    Application,
}

impl AudioSourceMode {
    /// The numeric wire value.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Auto => 0,
            Self::SystemMix => 1,
            Self::Application => 2,
        }
    }

    /// Parses the numeric wire value.
    pub fn from_u8(value: u8) -> Result<Self, IpcError> {
        match value {
            0 => Ok(Self::Auto),
            1 => Ok(Self::SystemMix),
            2 => Ok(Self::Application),
            other => Err(IpcError::MalformedPayload(format!(
                "unknown audio source mode {other}"
            ))),
        }
    }
}

/// The payload of the `SetAudioSource` command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetAudioSourcePayload {
    /// The selection mode.
    pub mode: AudioSourceMode,
    /// The stable source identifier. Required for
    /// [`AudioSourceMode::Application`], forbidden otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

impl SetAudioSourcePayload {
    /// Validates the payload against the mode.
    pub fn validate(&self) -> Result<(), IpcError> {
        if let Some(id) = &self.source_id {
            validate_identifier(id, "source_id")?;
        }
        match (self.mode, &self.source_id) {
            (AudioSourceMode::Application, None) => Err(IpcError::InvalidArgument(
                "source_id is required when mode is Application".to_owned(),
            )),
            (AudioSourceMode::Auto, Some(_)) | (AudioSourceMode::SystemMix, Some(_)) => {
                Err(IpcError::InvalidArgument(
                    "source_id must be absent unless mode is Application".to_owned(),
                ))
            }
            _ => Ok(()),
        }
    }
}

/// The payload of the `SetVibePreset` command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetVibePresetPayload {
    /// The stable preset identifier.
    pub preset_id: String,
}

impl SetVibePresetPayload {
    /// Validates the payload.
    pub fn validate(&self) -> Result<(), IpcError> {
        validate_identifier(&self.preset_id, "preset_id")
    }
}

/// The payload of the `SetEffectSettings` command.
///
/// Effects are addressed by name. Each effect carries a bounded intensity in
/// `0.0..=1.0` and an enable flag.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetEffectSettingsPayload {
    /// The effect name.
    pub effect: String,
    /// Whether the effect is enabled.
    pub enabled: bool,
    /// Effect intensity, `0.0..=1.0`.
    pub intensity: f32,
}

impl SetEffectSettingsPayload {
    /// Validates the payload.
    pub fn validate(&self) -> Result<(), IpcError> {
        validate_identifier(&self.effect, "effect")?;
        if !self.intensity.is_finite() {
            return Err(IpcError::InvalidArgument(
                "intensity must be a finite number".to_owned(),
            ));
        }
        if !(0.0..=1.0).contains(&self.intensity) {
            return Err(IpcError::InvalidArgument(
                "intensity must be within 0.0..=1.0".to_owned(),
            ));
        }
        Ok(())
    }
}

/// The payload of the `ImportWallpaper` command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportWallpaperPayload {
    /// The absolute path of the image to import.
    pub path: String,
    /// Whether the engine may run local depth inference for this import.
    ///
    /// The default is `false`: depth inference is optional and every
    /// AI-derived effect must have a deterministic fallback.
    #[serde(default)]
    pub allow_depth_inference: bool,
}

impl ImportWallpaperPayload {
    /// Validates the payload.
    pub fn validate(&self) -> Result<(), IpcError> {
        validate_path(&self.path, "path")
    }
}

/// The payload of the `CancelJob` command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelJobPayload {
    /// The identifier of the job to cancel.
    pub job_id: String,
}

impl CancelJobPayload {
    /// Validates the payload.
    pub fn validate(&self) -> Result<(), IpcError> {
        validate_identifier(&self.job_id, "job_id")
    }
}

/// The payload of the `ApplyPackage` command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPackagePayload {
    /// The package identifier or the path of a `.ddpkg` file.
    pub package_id: String,
}

impl ApplyPackagePayload {
    /// Validates the payload.
    pub fn validate(&self) -> Result<(), IpcError> {
        validate_identifier(&self.package_id, "package_id")
    }
}

/// The frame-rate cap options of the performance policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FrameRateCap {
    /// 24 frames per second.
    Fps24,
    /// 30 frames per second. The product default.
    Fps30,
    /// 60 frames per second.
    Fps60,
}

impl FrameRateCap {
    /// The numeric wire value.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Fps24 => 24,
            Self::Fps30 => 30,
            Self::Fps60 => 60,
        }
    }

    /// Parses the numeric wire value.
    pub fn from_u8(value: u8) -> Result<Self, IpcError> {
        match value {
            24 => Ok(Self::Fps24),
            30 => Ok(Self::Fps30),
            60 => Ok(Self::Fps60),
            other => Err(IpcError::MalformedPayload(format!(
                "unknown frame rate cap {other}"
            ))),
        }
    }
}

/// The payload of the `SetPerformancePolicy` command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPerformancePolicyPayload {
    /// The frame-rate cap.
    pub frame_rate_cap: FrameRateCap,
    /// Whether rendering pauses while a full-screen application covers the
    /// display.
    pub pause_on_fullscreen: bool,
    /// Whether rendering pauses while the display is asleep.
    pub pause_on_display_sleep: bool,
    /// Whether rendering pauses while the system is on battery.
    pub pause_on_battery: bool,
}

impl SetPerformancePolicyPayload {
    /// Validates the payload.
    ///
    /// The boolean flags are always valid, so this exists to keep the
    /// validation surface uniform across payloads.
    pub fn validate(&self) -> Result<(), IpcError> {
        Ok(())
    }
}

/// The error body carried by a failure response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorPayload {
    /// The stable numeric error code.
    pub code: u16,
    /// A short, human-readable description. It must not contain user data.
    pub message: String,
}

impl ErrorPayload {
    /// Builds an error payload from a known code.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_u16(),
            message: message.into(),
        }
    }

    /// Builds an error payload from an [`IpcError`].
    #[must_use]
    pub fn from_error(error: &IpcError) -> Self {
        Self {
            code: error.code().as_u16(),
            message: error.to_string(),
        }
    }

    /// The stable code, if it is a known one.
    #[must_use]
    pub fn error_code(&self) -> Option<ErrorCode> {
        ErrorCode::from_u16(self.code)
    }
}

/// A request envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    /// The protocol version the client speaks.
    pub version: u16,
    /// The client-chosen request identifier, used to correlate responses.
    pub request_id: u64,
    /// The command name.
    pub command: String,
    /// The command payload, if the command has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl RequestEnvelope {
    /// Builds a request envelope for a command without a payload.
    #[must_use]
    pub fn new(request_id: u64, command: Command) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request_id,
            command: command.name().to_owned(),
            payload: None,
        }
    }

    /// Builds a request envelope for a command with a payload.
    ///
    /// Returns an error if the payload cannot be serialized.
    pub fn with_payload(
        request_id: u64,
        command: Command,
        payload: &serde_json::Value,
    ) -> Result<Self, IpcError> {
        Ok(Self {
            version: PROTOCOL_VERSION,
            request_id,
            command: command.name().to_owned(),
            payload: Some(payload.clone()),
        })
    }

    /// Validates the envelope.
    ///
    /// Checks the protocol version, the command name and the command-specific
    /// payload. A payload that is present for a command that takes none is
    /// rejected, so that a malformed client cannot smuggle data past the
    /// handler.
    pub fn validate(&self) -> Result<Command, IpcError> {
        if self.version != PROTOCOL_VERSION {
            return Err(IpcError::ProtocolVersionMismatch {
                found: self.version,
                expected: PROTOCOL_VERSION,
            });
        }

        let command = Command::from_name(&self.command)?;

        match command {
            Command::SetAudioSource => {
                let payload = self.typed_payload::<SetAudioSourcePayload>(command)?;
                payload.validate()?;
            }
            Command::SetVibePreset => {
                let payload = self.typed_payload::<SetVibePresetPayload>(command)?;
                payload.validate()?;
            }
            Command::SetEffectSettings => {
                let payload = self.typed_payload::<SetEffectSettingsPayload>(command)?;
                payload.validate()?;
            }
            Command::ImportWallpaper => {
                let payload = self.typed_payload::<ImportWallpaperPayload>(command)?;
                payload.validate()?;
            }
            Command::CancelJob => {
                let payload = self.typed_payload::<CancelJobPayload>(command)?;
                payload.validate()?;
            }
            Command::ApplyPackage => {
                let payload = self.typed_payload::<ApplyPackagePayload>(command)?;
                payload.validate()?;
            }
            Command::SetPerformancePolicy => {
                let payload = self.typed_payload::<SetPerformancePolicyPayload>(command)?;
                payload.validate()?;
            }
            Command::GetEngineStatus
            | Command::GetPlatformCapabilities
            | Command::GetAudioSources
            | Command::Pause
            | Command::Resume
            | Command::TogglePause
            | Command::GetDiagnosticsSnapshot
            | Command::ShutdownEngine => {
                if self.payload.is_some() {
                    return Err(IpcError::InvalidArgument(format!(
                        "command {} does not accept a payload",
                        command.name()
                    )));
                }
            }
        }

        Ok(command)
    }

    /// Decodes the typed payload for a command.
    fn typed_payload<T: serde::de::DeserializeOwned>(
        &self,
        command: Command,
    ) -> Result<T, IpcError> {
        let raw = self.payload.as_ref().ok_or_else(|| {
            IpcError::InvalidArgument(format!("command {} requires a payload", command.name()))
        })?;

        serde_json::from_value(raw.clone()).map_err(|err| {
            IpcError::MalformedPayload(format!("invalid payload for {}: {err}", command.name()))
        })
    }
}

/// A response envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    /// The protocol version the engine speaks.
    pub version: u16,
    /// The request identifier this response answers.
    pub request_id: u64,
    /// The successful result body, if the command succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// The error body, if the command failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorPayload>,
}

impl ResponseEnvelope {
    /// Builds a successful response.
    #[must_use]
    pub fn ok(request_id: u64, result: Option<serde_json::Value>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request_id,
            result,
            error: None,
        }
    }

    /// Builds a failed response.
    #[must_use]
    pub fn err(request_id: u64, error: &ErrorPayload) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request_id,
            result: None,
            error: Some(error.clone()),
        }
    }

    /// Builds a failed response from an [`IpcError`].
    #[must_use]
    pub fn from_error(request_id: u64, error: &IpcError) -> Self {
        Self::err(request_id, &ErrorPayload::from_error(error))
    }

    /// Whether this response carries a successful result.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
}

/// Validates a bounded identifier string.
///
/// Identifiers are opaque, non-empty and bounded. They are not paths and are
/// not required to be printable.
fn validate_identifier(value: &str, field: &str) -> Result<(), IpcError> {
    if value.is_empty() {
        return Err(IpcError::InvalidArgument(format!(
            "{field} must not be empty"
        )));
    }
    if value.len() > MAX_STRING_BYTES {
        return Err(IpcError::InvalidArgument(format!(
            "{field} exceeds the {MAX_STRING_BYTES} byte limit"
        )));
    }
    Ok(())
}

/// Validates a bounded path string.
///
/// The path must be absolute, must not contain a NUL byte and must not exceed
/// the string limit. The *file system* is not touched here; the owning
/// subsystem resolves and opens the path under the process's own permissions.
fn validate_path(value: &str, field: &str) -> Result<(), IpcError> {
    if value.is_empty() {
        return Err(IpcError::InvalidArgument(format!(
            "{field} must not be empty"
        )));
    }
    if value.len() > MAX_STRING_BYTES {
        return Err(IpcError::InvalidArgument(format!(
            "{field} exceeds the {MAX_STRING_BYTES} byte limit"
        )));
    }
    if value.contains('\0') {
        return Err(IpcError::InvalidArgument(format!(
            "{field} must not contain a NUL byte"
        )));
    }
    if !is_absolute_path(value) {
        return Err(IpcError::InvalidArgument(format!(
            "{field} must be an absolute path"
        )));
    }
    Ok(())
}

/// Returns whether `value` looks like an absolute path on this platform.
///
/// On Windows this accepts `C:\...`, `\\server\share\...` and `\\?\C:\...`.
/// On Unix it accepts a leading `/`.
#[must_use]
pub fn is_absolute_path(value: &str) -> bool {
    #[cfg(windows)]
    {
        let bytes = value.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return bytes.len() == 2 || bytes[2] == b'\\' || bytes[2] == b'/';
        }
        // UNC: \\server\share\...  and extended-length: \\?\...
        value.starts_with("\\\\")
    }
    #[cfg(not(windows))]
    {
        value.starts_with('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_command_name_round_trips() {
        for command in Command::all() {
            assert_eq!(
                Command::from_name(command.name()).expect("name should parse"),
                *command
            );
        }
    }

    #[test]
    fn unknown_command_names_are_rejected() {
        assert!(matches!(
            Command::from_name("NotACommand"),
            Err(IpcError::UnknownCommand(_))
        ));
        assert!(matches!(
            Command::from_name("getenginestatus"),
            Err(IpcError::UnknownCommand(_))
        ));
        assert!(matches!(
            Command::from_name(""),
            Err(IpcError::UnknownCommand(_))
        ));
    }

    #[test]
    fn command_names_are_unique() {
        let mut names: Vec<&str> = Command::all().iter().map(|c| c.name()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate command name in the protocol");
    }

    #[test]
    fn envelope_rejects_a_version_mismatch() {
        let mut envelope = RequestEnvelope::new(1, Command::GetEngineStatus);
        envelope.version = 2;
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::ProtocolVersionMismatch {
                found: 2,
                expected: 1
            })
        ));
    }

    #[test]
    fn envelope_rejects_an_unknown_command() {
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "DefinitelyNotACommand".to_owned(),
            payload: None,
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::UnknownCommand(_))
        ));
    }

    #[test]
    fn envelope_rejects_a_payload_on_a_payloadless_command() {
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "GetEngineStatus".to_owned(),
            payload: Some(json!({"unexpected": true})),
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::InvalidArgument(_))
        ));
    }

    #[test]
    fn envelope_rejects_a_missing_payload() {
        let envelope = RequestEnvelope::new(1, Command::SetVibePreset);
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::InvalidArgument(_))
        ));
    }

    #[test]
    fn envelope_rejects_an_oversized_string() {
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "SetVibePreset".to_owned(),
            payload: Some(json!({ "preset_id": "x".repeat(MAX_STRING_BYTES + 1) })),
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::InvalidArgument(_))
        ));
    }

    #[test]
    fn envelope_rejects_an_empty_identifier() {
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "SetVibePreset".to_owned(),
            payload: Some(json!({ "preset_id": "" })),
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::InvalidArgument(_))
        ));
    }

    #[test]
    fn envelope_rejects_a_non_absolute_import_path() {
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "ImportWallpaper".to_owned(),
            payload: Some(json!({ "path": "relative/image.png" })),
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::InvalidArgument(_))
        ));
    }

    #[test]
    fn envelope_rejects_a_nul_byte_in_a_path() {
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "ImportWallpaper".to_owned(),
            payload: Some(json!({ "path": "C:\\a\u{0}b.png" })),
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::InvalidArgument(_))
        ));
    }

    #[test]
    fn envelope_rejects_an_unknown_enum_value() {
        // `AudioSourceMode` is encoded numerically on the wire so that an
        // unknown value is detectable rather than silently coerced.
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "SetAudioSource".to_owned(),
            payload: Some(json!({ "mode": 99, "source_id": null })),
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::MalformedPayload(_))
        ));
    }

    #[test]
    fn envelope_rejects_an_out_of_range_intensity() {
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request_id: 1,
            command: "SetEffectSettings".to_owned(),
            payload: Some(json!({ "effect": "depth", "enabled": true, "intensity": 1.5 })),
        };
        assert!(matches!(
            envelope.validate(),
            Err(IpcError::InvalidArgument(_))
        ));
    }

    #[test]
    fn envelope_rejects_a_non_finite_intensity() {
        // JSON has no NaN literal, so a non-finite value can only arrive from a
        // hand-crafted payload. Check the validator directly.
        let payload = SetEffectSettingsPayload {
            effect: "depth".to_owned(),
            enabled: true,
            intensity: f32::NAN,
        };
        assert!(payload.validate().is_err());
    }

    #[test]
    fn audio_source_mode_requires_a_source_id_only_for_applications() {
        let missing = SetAudioSourcePayload {
            mode: AudioSourceMode::Application,
            source_id: None,
        };
        assert!(missing.validate().is_err());

        let extra = SetAudioSourcePayload {
            mode: AudioSourceMode::SystemMix,
            source_id: Some("app".to_owned()),
        };
        assert!(extra.validate().is_err());

        let good = SetAudioSourcePayload {
            mode: AudioSourceMode::Application,
            source_id: Some("app".to_owned()),
        };
        assert!(good.validate().is_ok());
    }

    #[test]
    fn audio_source_mode_rejects_unknown_values() {
        assert!(AudioSourceMode::from_u8(0).is_ok());
        assert!(AudioSourceMode::from_u8(1).is_ok());
        assert!(AudioSourceMode::from_u8(2).is_ok());
        assert!(AudioSourceMode::from_u8(3).is_err());
    }

    #[test]
    fn frame_rate_cap_rejects_unknown_values() {
        assert!(FrameRateCap::from_u8(24).is_ok());
        assert!(FrameRateCap::from_u8(30).is_ok());
        assert!(FrameRateCap::from_u8(60).is_ok());
        assert!(FrameRateCap::from_u8(120).is_err());
    }

    #[test]
    fn response_envelopes_report_success_and_failure() {
        let ok = ResponseEnvelope::ok(7, Some(json!({"state": "Stopped"})));
        assert!(ok.is_ok());
        assert_eq!(ok.request_id, 7);

        let err = ResponseEnvelope::from_error(8, &IpcError::UnknownCommand("X".to_owned()));
        assert!(!err.is_ok());
        let payload = err.error.expect("error payload should be present");
        assert_eq!(payload.code, ErrorCode::UnknownCommand.as_u16());
        assert_eq!(
            payload.error_code(),
            Some(ErrorCode::UnknownCommand),
            "error code should map back to the known variant"
        );
    }

    #[test]
    fn envelopes_round_trip_through_json() {
        let envelope = RequestEnvelope::new(42, Command::GetEngineStatus);
        let json_text = serde_json::to_string(&envelope).expect("serialize");
        let parsed: RequestEnvelope = serde_json::from_str(&json_text).expect("deserialize");
        assert_eq!(parsed, envelope);
    }

    #[test]
    fn absolute_path_detection_matches_the_platform() {
        if cfg!(windows) {
            assert!(is_absolute_path(r"C:\Users\x\y.png"));
            assert!(is_absolute_path(r"\\server\share\x.png"));
            assert!(!is_absolute_path("relative/x.png"));
            assert!(!is_absolute_path(""));
        } else {
            assert!(is_absolute_path("/home/user/x.png"));
            assert!(!is_absolute_path("relative/x.png"));
        }
    }
}

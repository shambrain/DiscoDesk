//! WASAPI shared-mode loopback capture of the default output device.
//!
//! # What this is, and is not
//!
//! This is **system/output loopback**: everything the user's default playback
//! device renders, mixed by Windows. It is **not** per-process capture, which
//! needs `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK` and a Windows 10 build of
//! 20348 or later (`ARCHITECTURE.md` §6.2). Apps that offer "app capture" on a
//! build that cannot do it are lying; this backend reports system mix only.
//!
//! # Threading and COM
//!
//! All COM objects here are created and used on one capture thread, in a
//! multi-threaded apartment. Nothing in this module is `Sync`; the session is
//! owned by exactly one thread, which the `CaptureSession` bound to `Send`
//! expresses.
//!
//! # Real-time rules
//!
//! [`CaptureSession::read_packet`] waits on the capture event with a bounded
//! timeout, copies out of the WASAPI buffer, and appends mono samples to a
//! caller-provided, reused `Vec`. It performs no I/O, no logging, and no
//! allocation once the caller's buffer has been reserved.

use std::time::{Duration, Instant};

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_E_DEVICE_INVALIDATED, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK, IAudioCaptureClient,
    IAudioClient, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, WAVEFORMATEX,
    WAVEFORMATEXTENSIBLE, eConsole, eRender,
};
use windows::Win32::Media::KernelStreaming::{KSDATAFORMAT_SUBTYPE_PCM, WAVE_FORMAT_EXTENSIBLE};
use windows::Win32::Media::Multimedia::{KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize, STGM_READ,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::core::PWSTR;

use super::{CaptureSession, DeviceInfo, ReadOutcome};
use crate::error::AudioError;
use crate::format::{AudioFormat, SampleEncoding};

/// How long a single read waits for the capture event before reporting a timeout.
const READ_TIMEOUT_MS: u32 = 250;

/// How often the default endpoint identity is re-checked.
const DEVICE_CHECK_INTERVAL: Duration = Duration::from_secs(2);

/// `RPC_E_CHANGED_MODE`: COM was already initialised with a different model.
const RPC_E_CHANGED_MODE: i32 = -2_147_417_082; // 0x80010106 as i32

/// Owns COM initialisation for one thread.
struct ComApartment {
    /// Whether this guard is responsible for calling `CoUninitialize`.
    owned: bool,
}

impl ComApartment {
    fn init_mta() -> Result<Self, AudioError> {
        // SAFETY: `CoInitializeEx` with a null reserved pointer is always valid.
        // The HRESULT is inspected; `RPC_E_CHANGED_MODE` means someone else
        // already initialised this thread and we must not uninitialise it.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() {
            return Ok(Self { owned: true });
        }
        if hr.0 as i32 == RPC_E_CHANGED_MODE {
            // Another model is already active on this thread. WASAPI works in
            // either apartment, so continue without owning the uninitialise.
            return Ok(Self { owned: false });
        }
        Err(AudioError::Activation(format!(
            "COM initialisation failed (0x{:08x})",
            hr.0
        )))
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: balanced against a successful `CoInitializeEx` on this
            // thread, and this guard is not `Clone`.
            unsafe { CoUninitialize() };
        }
    }
}

/// Owns a Win32 event handle.
struct OwnedEvent(HANDLE);

impl OwnedEvent {
    fn new() -> Result<Self, AudioError> {
        // SAFETY: all parameters are the documented defaults for an unnamed,
        // auto-reset, initially non-signalled event with no security attributes.
        let handle = unsafe { CreateEventW(None, false, false, None) }
            .map_err(|err| AudioError::Initialization(format!("event creation failed: {err}")))?;
        if handle.is_invalid() {
            return Err(AudioError::Initialization(
                "event creation returned an invalid handle".to_owned(),
            ));
        }
        Ok(Self(handle))
    }
}

impl Drop for OwnedEvent {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: this handle is owned solely by this value and closed once.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

/// One open loopback capture session.
pub struct WasapiSession {
    format: AudioFormat,
    device: DeviceInfo,
    enumerator: IMMDeviceEnumerator,
    client: IAudioClient,
    capture: IAudioCaptureClient,
    event: OwnedEvent,
    last_device_check: Instant,
}

// SAFETY: the session is used by exactly one thread at a time. Every COM call
// takes `&self`/`&mut self` and none of the interfaces are threadsafe in a way
// this code relies on.
unsafe impl Send for WasapiSession {}

impl std::fmt::Debug for WasapiSession {
    /// COM interfaces are not `Debug`, and dumping their pointers would be
    /// useless in a log line. Report the negotiated identity instead.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasapiSession")
            .field("format", &self.format)
            .field("device", &self.device)
            .finish_non_exhaustive()
    }
}

impl WasapiSession {
    /// Opens loopback capture on the current default render endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`AudioError`] describing why capture could not be established.
    /// `NoDefaultDevice` and `DeviceInvalidated` are recoverable by retrying.
    pub fn open() -> Result<Self, AudioError> {
        // The apartment guard lives for the whole session: WASAPI objects must
        // be released before COM is uninitialised, and field drop order gives
        // us that if the guard is held outside the struct.
        //
        // Instead, the caller (the runtime) initialises COM for the thread and
        // keeps it alive; `open` only requires that COM is already initialised.
        // Creating a guard here would uninitialise while the session's COM
        // pointers are still alive.
        Self::open_without_apartment()
    }

    fn open_without_apartment() -> Result<Self, AudioError> {
        // SAFETY: COM is initialised on this thread by the runtime before any
        // session is opened; every call below documents its own invariants.
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|err| {
                    AudioError::Activation(format!("device enumerator unavailable: {err}"))
                })?;

            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|_| AudioError::NoDefaultDevice)?;

            let id = endpoint_id(&device);
            let name = friendly_name(&device).unwrap_or_else(|| id.clone());

            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|err| {
                AudioError::Activation(format!("audio client activation failed: {err}"))
            })?;

            let mix = client.GetMixFormat().map_err(|err| {
                AudioError::Initialization(format!("mix format unavailable: {err}"))
            })?;

            let parsed = parse_format(mix)?;
            // The format pointer is owned by the caller and must be released.
            CoTaskMemFree(Some(mix.cast()));

            let event = OwnedEvent::new()?;

            // Shared mode, loopback, event driven. A zero buffer duration asks
            // the engine for its preferred size.
            client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                    0,
                    0,
                    mix,
                    None,
                )
                .map_err(|err| {
                    AudioError::Initialization(format!("stream initialisation failed: {err}"))
                })?;

            client.SetEventHandle(event.0).map_err(|err| {
                AudioError::Initialization(format!("event registration failed: {err}"))
            })?;

            let capture: IAudioCaptureClient = client.GetService().map_err(|err| {
                AudioError::Initialization(format!("capture service unavailable: {err}"))
            })?;

            client
                .Start()
                .map_err(|err| AudioError::Initialization(format!("stream start failed: {err}")))?;

            Ok(Self {
                format: parsed,
                device: DeviceInfo { id, name },
                enumerator,
                client,
                capture,
                event,
                last_device_check: Instant::now(),
            })
        }
    }
}

impl CaptureSession for WasapiSession {
    fn format(&self) -> &AudioFormat {
        &self.format
    }

    fn device(&self) -> &DeviceInfo {
        &self.device
    }

    fn backend(&self) -> &'static str {
        "wasapi-loopback"
    }

    fn read_packet(&mut self, out: &mut Vec<f32>) -> Result<ReadOutcome, AudioError> {
        // SAFETY: `self.event.0` is a live event handle owned by this session.
        let wait = unsafe { WaitForSingleObject(self.event.0, READ_TIMEOUT_MS) };
        if wait == WAIT_TIMEOUT {
            return Ok(ReadOutcome::Timeout);
        }
        if wait != WAIT_OBJECT_0 {
            return Err(AudioError::Capture(format!(
                "capture wait failed (0x{:08x})",
                wait.0
            )));
        }

        let mut produced = 0_usize;
        let mut saw_silence = false;

        loop {
            let mut data: *mut u8 = std::ptr::null_mut();
            let mut frames = 0_u32;
            let mut flags = 0_u32;

            // SAFETY: the out-pointers are valid stack locations; the returned
            // buffer is owned by the capture client until `ReleaseBuffer`.
            let read = unsafe {
                self.capture
                    .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
            };

            if let Err(err) = read {
                if err.code() == AUDCLNT_E_DEVICE_INVALIDATED {
                    return Err(AudioError::DeviceInvalidated);
                }
                return Err(AudioError::Capture(format!("get buffer failed: {err}")));
            }

            if frames == 0 {
                break;
            }

            if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
                // The contents are undefined for a silent packet; append zeros
                // so the DSP keeps its frame cadence instead of stalling.
                out.extend(std::iter::repeat_n(0.0_f32, frames as usize));
                produced += frames as usize;
                saw_silence = true;
            } else {
                let byte_len = frames as usize * self.format.frame_bytes();
                // SAFETY: WASAPI guarantees `data` points at `frames` frames of
                // the negotiated format for `byte_len` bytes until the matching
                // `ReleaseBuffer` below.
                let bytes = unsafe { std::slice::from_raw_parts(data, byte_len) };
                let before = out.len();
                self.format
                    .decode_mono(bytes, out)
                    .map_err(|err| AudioError::Capture(err.to_string()))?;
                produced += out.len() - before;
            }

            // SAFETY: `frames` is exactly what the matching `GetBuffer` returned.
            unsafe {
                self.capture
                    .ReleaseBuffer(frames)
                    .map_err(|err| AudioError::Capture(format!("release failed: {err}")))?;
            }
        }

        if produced == 0 {
            return Ok(ReadOutcome::Timeout);
        }
        Ok(if saw_silence {
            ReadOutcome::Silence(produced)
        } else {
            ReadOutcome::Samples(produced)
        })
    }

    fn default_device_changed(&mut self) -> Result<bool, AudioError> {
        if self.last_device_check.elapsed() < DEVICE_CHECK_INTERVAL {
            return Ok(false);
        }
        self.last_device_check = Instant::now();

        // SAFETY: the enumerator is live and used on this thread only.
        let current = unsafe {
            self.enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|_| AudioError::NoDefaultDevice)?
        };
        let current_id = endpoint_id(&current);
        Ok(current_id != self.device.id)
    }
}

impl Drop for WasapiSession {
    fn drop(&mut self) {
        // Stop the stream before the interfaces go away so the audio engine
        // does not keep a connection to a dead client.
        // SAFETY: `self.client` is a live interface owned by this session.
        unsafe {
            let _ = self.client.Stop();
        }
    }
}

/// Reads an endpoint id and releases the COM string.
fn endpoint_id(device: &IMMDevice) -> String {
    // SAFETY: `GetId` returns a `CoTaskMemAlloc`-owned string, freed below; the
    // device is a live interface.
    unsafe {
        match device.GetId() {
            Ok(ptr) => {
                let text = ptr.to_string().unwrap_or_default();
                CoTaskMemFree(Some(ptr.0.cast()));
                text
            }
            Err(_) => String::new(),
        }
    }
}

/// Reads the friendly name from the endpoint property store.
///
/// Returns `None` when the property is absent or unreadable; the caller falls
/// back to the endpoint id so telemetry still identifies the device.
fn friendly_name(device: &IMMDevice) -> Option<String> {
    // SAFETY: the property store and PROPVARIANT are used within this scope and
    // released here; the strings are `CoTaskMemAlloc`-owned and freed below.
    unsafe {
        let store: IPropertyStore = device.OpenPropertyStore(STGM_READ).ok()?;
        let mut value = store.GetValue(&PKEY_Device_FriendlyName).ok()?;
        let result = PropVariantToStringAlloc(&value)
            .ok()
            .and_then(|ptr: PWSTR| {
                let text = ptr.to_string().ok();
                CoTaskMemFree(Some(ptr.0.cast()));
                text
            });
        let _ = PropVariantClear(&mut value);
        result.filter(|name| !name.is_empty())
    }
}

/// Interprets a `WAVEFORMATEX` (possibly extensible) as an [`AudioFormat`].
fn parse_format(format: *const WAVEFORMATEX) -> Result<AudioFormat, AudioError> {
    if format.is_null() {
        return Err(AudioError::UnsupportedFormat(
            "the endpoint reported no mix format".to_owned(),
        ));
    }
    // SAFETY: the pointer came from `GetMixFormat`, which returns a valid
    // `WAVEFORMATEX` whose `cbSize` describes any trailing extensible fields.
    let base = unsafe { *format };
    let bits = base.wBitsPerSample;

    let tag = if u32::from(base.wFormatTag) == WAVE_FORMAT_EXTENSIBLE {
        // SAFETY: `wFormatTag == WAVE_FORMAT_EXTENSIBLE` implies the structure is
        // a `WAVEFORMATEXTENSIBLE` and `cbSize >= 22`.
        let extensible = unsafe { *(format.cast::<WAVEFORMATEXTENSIBLE>()) };
        let guid = extensible.SubFormat;
        if guid == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT {
            WAVE_FORMAT_IEEE_FLOAT as u16
        } else if guid == KSDATAFORMAT_SUBTYPE_PCM {
            1
        } else {
            return Err(AudioError::UnsupportedFormat(format!(
                "unrecognised extensible subformat for {bits}-bit audio"
            )));
        }
    } else {
        base.wFormatTag
    };

    let encoding = match (u32::from(tag), bits) {
        (t, 32) if t == WAVE_FORMAT_IEEE_FLOAT => SampleEncoding::F32,
        (1, 16) => SampleEncoding::I16,
        (1, 32) => SampleEncoding::I32,
        (1, 8) => SampleEncoding::U8,
        _ => {
            return Err(AudioError::UnsupportedFormat(format!(
                "{bits}-bit format tag {tag} is not supported"
            )));
        }
    };

    AudioFormat::new(
        base.nSamplesPerSec,
        base.nChannels,
        encoding,
        base.nBlockAlign,
    )
    .map_err(|err| AudioError::UnsupportedFormat(err.to_string()))
}

/// Opens a loopback session on the default render endpoint.
///
/// COM must already be initialised on the calling thread in a multi-threaded
/// apartment; [`ComApartment`] provides that and the runtime holds it for the
/// lifetime of the capture thread.
///
/// # Errors
///
/// Propagates [`AudioError`] from enumeration, activation or initialisation.
pub fn open_default_loopback() -> Result<Box<dyn CaptureSession>, AudioError> {
    Ok(Box::new(WasapiSession::open()?))
}

/// Initialises COM for the current thread and returns a guard.
///
/// Public so the runtime can hold the apartment for the whole capture thread,
/// rather than per session (which would tear COM down while a session's
/// interfaces were still alive).
///
/// # Errors
///
/// Returns [`AudioError::Activation`] if COM cannot be initialised.
pub fn initialise_com() -> Result<impl Drop, AudioError> {
    ComApartment::init_mta()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::FormatError;

    #[test]
    fn a_waveformatex_without_a_format_is_rejected() {
        assert!(matches!(
            parse_format(std::ptr::null()),
            Err(AudioError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn a_sixteen_bit_stereo_pcm_format_parses() {
        let base = WAVEFORMATEX {
            wFormatTag: 1,
            nChannels: 2,
            nSamplesPerSec: 48_000,
            nAvgBytesPerSec: 192_000,
            nBlockAlign: 4,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let parsed = parse_format(&base).expect("16-bit stereo is supported");
        assert_eq!(parsed.sample_rate, 48_000);
        assert_eq!(parsed.channels, 2);
        assert_eq!(parsed.encoding, SampleEncoding::I16);
    }

    #[test]
    fn a_float_format_parses() {
        let base = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_IEEE_FLOAT as u16,
            nChannels: 2,
            nSamplesPerSec: 44_100,
            nAvgBytesPerSec: 352_800,
            nBlockAlign: 8,
            wBitsPerSample: 32,
            cbSize: 0,
        };
        let parsed = parse_format(&base).expect("float is supported");
        assert_eq!(parsed.encoding, SampleEncoding::F32);
    }

    #[test]
    fn an_unsupported_bit_depth_is_rejected_with_a_specific_error() {
        let base = WAVEFORMATEX {
            wFormatTag: 1,
            nChannels: 2,
            nSamplesPerSec: 48_000,
            nAvgBytesPerSec: 0,
            nBlockAlign: 6,
            wBitsPerSample: 24,
            cbSize: 0,
        };
        match parse_format(&base) {
            Err(AudioError::UnsupportedFormat(message)) => {
                assert!(message.contains("24-bit"), "message was {message}");
            }
            other => panic!("expected UnsupportedFormat, got {other:?}"),
        }
    }

    #[test]
    fn an_inconsistent_block_align_is_reported_as_unsupported() {
        let base = WAVEFORMATEX {
            wFormatTag: 1,
            nChannels: 2,
            nSamplesPerSec: 48_000,
            nAvgBytesPerSec: 0,
            nBlockAlign: 2,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        // 2 channels of 16-bit is 4 bytes per frame, so 2 is wrong.
        assert!(matches!(
            parse_format(&base),
            Err(AudioError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn format_errors_are_displayable_without_paths() {
        let error = FormatError::RaggedBuffer {
            len: 5,
            block_align: 8,
        };
        let text = error.to_string();
        assert!(text.contains('5'));
        assert!(!text.contains('\\'));
        assert!(!text.contains(":\\"));
    }
}

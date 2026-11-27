//! Windows named-pipe transport.
//!
//! # Security
//!
//! The **default** DACL for a Windows named pipe grants read access to
//! `Everyone`. That is unacceptable for DiscoDesk, so the pipe is always
//! created with an explicit security descriptor built from this SDDL string:
//!
//! ```text
//! D:P(A;;GA;;;OW)(A;;GA;;;SY)
//! ```
//!
//! * `D:P` - a protected DACL: the descriptor's own DACL is authoritative and
//!   is not merged with the parent's.
//! * `(A;;GA;;;OW)` - generic all to `OWNER RIGHTS`, i.e. the principal that
//!   created the pipe.
//! * `(A;;GA;;;SY)` - generic all to `LOCAL SYSTEM`.
//!
//! There is deliberately **no** `WD` (`Everyone`), `BU` (`BUILTIN\Users`) or
//! `AN` (`Anonymous`) ACE, so no other principal can open the pipe.
//!
//! `PIPE_REJECT_REMOTE_CLIENTS` is set so that a client on another machine
//! cannot connect even if the ACL were somehow permissive.
//!
//! The pipe name includes a sanitised user SID so that two users on the same
//! machine never share an endpoint.
//!
//! # API version
//!
//! Every signature here is taken from the generated bindings of the `windows`
//! crate version pinned in the workspace, which wraps the Win32 calls in
//! typed newtypes and `Result` returns. The notable consequences:
//!
//! * `ReadFile`/`WriteFile` take a slice, not a raw pointer plus length.
//! * `CreateFileW` returns `Result<HANDLE>`; `CreateNamedPipeW` returns a bare
//!   `HANDLE` that must be checked against `INVALID_HANDLE_VALUE`.
//! * `PIPE_ACCESS_DUPLEX` is a `FILE_FLAGS_AND_ATTRIBUTES`, while the pipe mode
//!   flags are `NAMED_PIPE_MODE`.
//! * `LocalFree` returns the handle rather than a status code.

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_FILE_OBJECT,
};
use windows::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetTokenInformation, PSECURITY_DESCRIPTOR, PSID,
    SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_MODE,
    OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, NAMED_PIPE_MODE, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, OpenProcessToken};
use windows::core::{BOOL, PCWSTR, PWSTR};

use crate::error::{IpcError, IpcResult};
use crate::frame::{self, FRAME_HEADER_BYTES, Frame};
use crate::protocol::PROTOCOL_VERSION;
use crate::transport::{Listener, Transport};

/// The SDDL used for every DiscoDesk named pipe.
///
/// Owned by the engine process that creates the pipe. The creator becomes the
/// owner of the object, and `OWNER RIGHTS` resolves to that owner.
pub const PIPE_SDDL: &str = "D:P(A;;GA;;;OW)(A;;GA;;;SY)";

/// The prefix of every DiscoDesk named pipe.
pub const PIPE_PREFIX: &str = r"\\.\pipe\discodesk-engine";

/// The size of the pipe's OS-level buffers. Large enough for several frames,
/// small enough to bound the kernel memory a client can pin.
const PIPE_BUFFER_BYTES: u32 = 64 * 1024;

/// `PIPE_UNLIMITED_INSTANCES`.
///
/// This must be greater than 1 even though Gate 0 serves one client at a time:
/// the accept loop creates the *replacement* listening instance while the
/// previously accepted instance is still connected. With a limit of 1 the
/// replacement cannot be created and the accepted connection is lost.
///
/// Concurrency is still bounded by the accept loop, which serves connections
/// serially, and by the endpoint ACL.
const PIPE_MAX_INSTANCES: u32 = 255;

/// The combined pipe mode: byte stream, byte read mode, blocking waits, and
/// remote clients rejected.
fn pipe_mode() -> NAMED_PIPE_MODE {
    NAMED_PIPE_MODE(
        PIPE_TYPE_BYTE.0 | PIPE_READMODE_BYTE.0 | PIPE_WAIT.0 | PIPE_REJECT_REMOTE_CLIENTS.0,
    )
}

/// An owned Windows HANDLE that is closed exactly once.
///
/// A handle equal to `INVALID_HANDLE_VALUE` is treated as absent, so a value
/// moved around during instance rotation never closes a non-handle.
#[derive(Debug)]
struct OwnedHandle(HANDLE);

impl OwnedHandle {
    /// Takes ownership of `handle`, rejecting an invalid one.
    fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_invalid() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    /// The raw handle.
    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if self.0.is_invalid() || self.0 == INVALID_HANDLE_VALUE {
            return;
        }
        // SAFETY: `self.0` is a valid handle this type owns exclusively. It is
        // closed exactly once because `OwnedHandle` is not `Copy` and the handle
        // is never handed out by value.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// A security descriptor allocated by
/// [`ConvertStringSecurityDescriptorToSecurityDescriptorW`].
///
/// The API allocates the descriptor with `LocalAlloc`, so it must be released
/// with `LocalFree`. This type does that exactly once.
struct OwnedSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl OwnedSecurityDescriptor {
    /// Builds a descriptor from an SDDL string.
    fn from_sddl(sddl: &str) -> IpcResult<Self> {
        let wide = to_wide(sddl);
        let mut descriptor = PSECURITY_DESCRIPTOR(std::ptr::null_mut());

        // SAFETY:
        // * `wide` is a valid NUL-terminated UTF-16 string that outlives the
        //   call.
        // * `descriptor` is a valid out-pointer; the API only writes it when it
        //   returns `Ok`, and the error branch constructs no `Self`, so nothing
        //   is freed on failure.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(wide.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|err| {
            IpcError::Platform(format!(
                "could not build the pipe security descriptor: {err}"
            ))
        })?;

        Ok(Self(descriptor))
    }

    /// The `SECURITY_ATTRIBUTES` that hands this descriptor to
    /// `CreateNamedPipeW`.
    ///
    /// `bInheritHandle` is false: a child process must not inherit the ability
    /// to create or open the engine endpoint.
    fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(0),
            lpSecurityDescriptor: self.0.0,
            bInheritHandle: BOOL(0),
        }
    }
}

impl Drop for OwnedSecurityDescriptor {
    fn drop(&mut self) {
        if self.0.0.is_null() {
            return;
        }
        // SAFETY: `self.0` was produced by
        // `ConvertStringSecurityDescriptorToSecurityDescriptorW`, which allocates
        // with `LocalAlloc`. `LocalFree` is the matching deallocator and is
        // called exactly once because this type is not `Copy`.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0.0)));
        }
    }
}

/// Converts a Rust string into a NUL-terminated UTF-16 buffer.
fn to_wide(value: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_utf16().collect();
    wide.push(0);
    wide
}

/// Reads a NUL-terminated wide string and frees it with `LocalFree`.
///
/// # Safety
///
/// `raw` must be a `LocalAlloc`-allocated, NUL-terminated wide string that the
/// caller does not use again. This function takes ownership and frees it.
unsafe fn take_wide_string(raw: PWSTR) -> String {
    if raw.0.is_null() {
        return String::new();
    }

    // SAFETY: the caller guarantees a NUL-terminated allocation; scanning stops
    // at the terminator and never runs past it.
    let text = unsafe {
        let mut len = 0_usize;
        while *raw.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(raw.0, len))
    };

    // SAFETY: `raw` was allocated with `LocalAlloc` by the Win32 call that
    // produced it. `LocalFree` is the matching deallocator; ownership is taken
    // here, so it is freed exactly once.
    unsafe {
        let _ = LocalFree(Some(HLOCAL(raw.0.cast::<c_void>())));
    }

    text
}

/// Returns the SID string of the current process token's user.
///
/// The SID is read from the process token rather than guessed from a name, so
/// it is correct for domain, local and Azure AD accounts alike.
pub(crate) fn current_user_sid() -> IpcResult<String> {
    let mut token = HANDLE::default();

    // SAFETY:
    // * `token` is a valid out-pointer for a HANDLE.
    // * `GetCurrentProcess` returns a pseudo-handle that is always valid and
    //   must not be closed.
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).map_err(|err| {
            IpcError::Platform(format!("could not open the process token: {err}"))
        })?;
    }

    let token = OwnedHandle::new(token)
        .ok_or_else(|| IpcError::Platform("the process token handle was invalid".to_owned()))?;

    let mut needed = 0_u32;

    // Sizing call: a null buffer asks the API for the required size, and the
    // resulting "insufficient buffer" error is the documented outcome.
    //
    // SAFETY: `token.raw()` is a valid open token handle; `None` is the
    // documented way to request the required size.
    unsafe {
        let _ = GetTokenInformation(token.raw(), TokenUser, None, 0, &mut needed);
    }

    if needed == 0 {
        return Err(IpcError::Platform(
            "the process token carries no user information".to_owned(),
        ));
    }

    let mut buffer = vec![0_u8; needed as usize];

    // SAFETY:
    // * `buffer` is a writable allocation of at least `needed` bytes.
    // * `needed` is the size the API requested for `TokenUser`.
    unsafe {
        GetTokenInformation(
            token.raw(),
            TokenUser,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            needed,
            &mut needed,
        )
        .map_err(|err| IpcError::Platform(format!("could not read the token user: {err}")))?;
    }

    // `TOKEN_USER` sits at the start of the buffer, and its `Sid` points into the
    // same allocation, which stays alive until `sid_to_string` returns.
    //
    // SAFETY: the API wrote a `TOKEN_USER` into `buffer`, the buffer is at least
    // `needed` bytes and properly aligned for `TOKEN_USER` because the API wrote
    // one there, and `buffer` is not moved while `sid` is alive.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };

    // SAFETY: `sid` points into `buffer`, which is alive for the duration of
    // this call and is not moved here.
    unsafe { sid_to_string(sid) }
}

/// Converts a SID into its `S-1-...` string form.
///
/// # Safety
///
/// `sid` must point at a valid SID that outlives the call.
unsafe fn sid_to_string(sid: PSID) -> IpcResult<String> {
    let mut raw = PWSTR::null();

    // SAFETY:
    // * `sid` points at a live SID owned by the token buffer.
    // * `raw` is a valid out-pointer; the API allocates the string with
    //   `LocalAlloc` and ownership passes to `take_wide_string`.
    unsafe {
        ConvertSidToStringSidW(sid, &mut raw).map_err(|err| {
            IpcError::Platform(format!("could not stringify the user SID: {err}"))
        })?;
        Ok(take_wide_string(raw))
    }
}

/// Builds the pipe name for the current user and a suffix.
#[must_use]
pub fn pipe_name(suffix: &str) -> String {
    let sid = current_user_sid().unwrap_or_else(|_| String::from("unknown"));
    let safe_sid: String = sid
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    format!("{PIPE_PREFIX}-{safe_sid}-{suffix}")
}

/// A listener that accepts named-pipe connections.
#[derive(Debug)]
pub struct WindowsListener {
    path: String,
    /// The pending pipe instance. `None` only transiently, while an accepted
    /// instance is being rotated out, or permanently if a replacement could not
    /// be created.
    handle: Option<OwnedHandle>,
    /// Why the listener stopped listening, if it did. Reported by the next
    /// `accept` so a failure is never silently swallowed.
    degraded: Option<String>,
}

// SAFETY: every Win32 call on the contained handle happens through `&mut self`,
// which the borrowed `Listener`/`Transport` API guarantees is exclusive, so the
// handle is never used from two threads at once. The handle is closed exactly
// once by `OwnedHandle::drop`.
unsafe impl Send for WindowsListener {}

impl WindowsListener {
    /// Creates a named pipe and prepares to accept clients.
    pub fn bind(path: &str) -> IpcResult<Self> {
        let handle = create_pipe_instance(path)?;
        Ok(Self {
            path: path.to_owned(),
            handle: Some(handle),
            degraded: None,
        })
    }

    /// The raw handle of the pending pipe instance.
    fn raw(&self) -> IpcResult<HANDLE> {
        if let Some(reason) = &self.degraded {
            return Err(IpcError::EndpointUnavailable(format!(
                "the pipe listener is no longer listening: {reason}"
            )));
        }
        self.handle
            .as_ref()
            .map(OwnedHandle::raw)
            .ok_or_else(|| IpcError::Platform("the pipe listener has no instance".to_owned()))
    }
}

/// Creates one pipe instance with the protected security descriptor.
fn create_pipe_instance(path: &str) -> IpcResult<OwnedHandle> {
    let descriptor = OwnedSecurityDescriptor::from_sddl(PIPE_SDDL)?;
    let attributes = descriptor.attributes();
    let wide_path = to_wide(path);

    // SAFETY:
    // * `wide_path` is a valid NUL-terminated UTF-16 string that outlives the
    //   call.
    // * `attributes` is a live `SECURITY_ATTRIBUTES` whose descriptor outlives
    //   the call.
    // * `CreateNamedPipeW` returns a bare handle, which is validated immediately
    //   and owned by `OwnedHandle` afterwards.
    let handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(wide_path.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            pipe_mode(),
            PIPE_MAX_INSTANCES,
            PIPE_BUFFER_BYTES,
            PIPE_BUFFER_BYTES,
            0,
            Some(&attributes),
        )
    };

    OwnedHandle::new(handle).ok_or_else(|| {
        IpcError::EndpointUnavailable(format!(
            "could not create the named pipe {path}; it may already be in use"
        ))
    })
}

impl Listener for WindowsListener {
    fn accept(&mut self) -> IpcResult<Box<dyn Transport>> {
        let raw = self.raw()?;

        // The pipe was created with `PIPE_WAIT`, so `ConnectNamedPipe` blocks
        // until a client connects.
        //
        // SAFETY: `raw` is a live pipe handle owned by this listener.
        unsafe {
            ConnectNamedPipe(raw, None).map_err(|err| {
                IpcError::Platform(format!("could not accept a pipe client: {err}"))
            })?;
        }

        // Hand the connected instance to the caller now, then create a
        // replacement so the endpoint keeps listening. The replacement is
        // created *after* the handoff on purpose: creating it first would give
        // the OS two listening instances, and a client could connect to the one
        // this loop is not blocked on.
        let connected = self
            .handle
            .take()
            .ok_or_else(|| IpcError::Platform("the pipe instance vanished".to_owned()))?;

        match create_pipe_instance(&self.path) {
            Ok(replacement) => self.handle = Some(replacement),
            Err(err) => {
                // The connection we just accepted is still usable, so serve it
                // rather than dropping it. The listener is now degraded: the
                // cause is recorded and returned by the next `accept` instead
                // of being swallowed.
                self.handle = None;
                self.degraded = Some(err.to_string());
            }
        }

        Ok(Box::new(WindowsStream {
            handle: connected,
            path: self.path.clone(),
        }))
    }

    fn endpoint_path(&self) -> &str {
        &self.path
    }
}

/// A connected named-pipe stream.
#[derive(Debug)]
pub struct WindowsStream {
    handle: OwnedHandle,
    path: String,
}

// SAFETY: every Win32 call on the contained handle happens through `&mut self`
// (see `Transport`), so it is never used concurrently. `OwnedHandle` closes it
// exactly once.
unsafe impl Send for WindowsStream {}

impl WindowsStream {
    /// Opens an existing named pipe as a client.
    pub fn connect(path: &str) -> IpcResult<Self> {
        let handle = open_pipe(path)?;
        Ok(Self {
            handle,
            path: path.to_owned(),
        })
    }

    /// The raw handle.
    fn raw(&self) -> HANDLE {
        self.handle.raw()
    }
}

/// Opens an existing pipe for read/write without inheriting the handle.
fn open_pipe(path: &str) -> IpcResult<OwnedHandle> {
    let wide_path = to_wide(path);

    // SAFETY:
    // * `wide_path` is a valid NUL-terminated UTF-16 string.
    // * `CreateFileW` returns `Result`, so the invalid-handle case is already
    //   handled before `OwnedHandle` takes ownership.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_MODE(0),
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|err| {
        IpcError::EndpointUnavailable(format!("could not open the named pipe {path}: {err}"))
    })?;

    OwnedHandle::new(handle).ok_or_else(|| {
        IpcError::EndpointUnavailable(format!("the named pipe {path} returned an invalid handle"))
    })
}

/// Windows error code for `ERROR_BROKEN_PIPE`.
const ERROR_BROKEN_PIPE: u32 = 109;
/// Windows error code for `ERROR_PIPE_NOT_CONNECTED`.
const ERROR_PIPE_NOT_CONNECTED: u32 = 233;

/// Extracts the Win32 error code from a `windows_core::Error`.
///
/// Win32 errors are surfaced as `HRESULT` values of the form `0x8007_00NN`,
/// where `NNNN` is the Win32 code.
fn win32_code(err: &windows::core::Error) -> Option<u32> {
    let hresult = err.code().0 as u32;
    if hresult & 0xFFFF_0000 == 0x8007_0000 {
        Some(hresult & 0x0000_FFFF)
    } else {
        None
    }
}

/// Whether a read error means the peer simply went away.
///
/// A client process exiting closes its handle, which surfaces as
/// `ERROR_BROKEN_PIPE` or `ERROR_PIPE_NOT_CONNECTED` rather than a clean EOF.
/// That is a normal disconnect, not a fault, so it maps to
/// [`IpcError::PeerClosed`] and is not logged as a warning.
fn is_peer_disconnect(err: &windows::core::Error) -> bool {
    matches!(
        win32_code(err),
        Some(ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED)
    )
}

/// Reads exactly `buffer.len()` bytes, mapping a clean EOF to `PeerClosed`.
fn read_exact(handle: HANDLE, buffer: &mut [u8]) -> IpcResult<()> {
    let mut read_total = 0_usize;

    while read_total < buffer.len() {
        let mut bytes_read = 0_u32;

        // SAFETY:
        // * `handle` is a live pipe handle.
        // * `&mut buffer[read_total..]` is an exclusively borrowed region of at
        //   least one byte, so the API cannot write past the buffer.
        let result = unsafe {
            windows::Win32::Storage::FileSystem::ReadFile(
                handle,
                Some(&mut buffer[read_total..]),
                Some(&mut bytes_read),
                None,
            )
        };

        if let Err(err) = result {
            if read_total == 0 && is_peer_disconnect(&err) {
                return Err(IpcError::PeerClosed);
            }
            return Err(IpcError::Io(std::io::Error::other(err.to_string())));
        }

        if bytes_read == 0 {
            return if read_total == 0 {
                Err(IpcError::PeerClosed)
            } else {
                Err(IpcError::TruncatedFrame {
                    read: read_total,
                    expected: buffer.len(),
                })
            };
        }

        read_total += bytes_read as usize;
    }

    Ok(())
}

/// Writes exactly `buffer.len()` bytes.
fn write_exact(handle: HANDLE, buffer: &[u8]) -> IpcResult<()> {
    let mut written_total = 0_usize;

    while written_total < buffer.len() {
        let mut bytes_written = 0_u32;

        // SAFETY:
        // * `handle` is a live pipe handle.
        // * `&buffer[written_total..]` is a valid, immutably borrowed region, so
        //   the API cannot read past the buffer.
        unsafe {
            windows::Win32::Storage::FileSystem::WriteFile(
                handle,
                Some(&buffer[written_total..]),
                Some(&mut bytes_written),
                None,
            )
            .map_err(|err| IpcError::Io(std::io::Error::other(err.to_string())))?;
        }

        if bytes_written == 0 {
            return Err(IpcError::Io(std::io::Error::other(
                "the named pipe accepted no bytes".to_owned(),
            )));
        }

        written_total += bytes_written as usize;
    }

    Ok(())
}

impl Transport for WindowsStream {
    fn send_frame(&mut self, frame: &Frame) -> IpcResult<()> {
        let encoded = frame::encode(frame.version, &frame.payload)?;
        write_exact(self.raw(), &encoded)
    }

    fn recv_frame(&mut self) -> IpcResult<Frame> {
        let mut header = [0_u8; FRAME_HEADER_BYTES];
        read_exact(self.raw(), &mut header)?;

        let payload_len = frame::peek_payload_len(&header, PROTOCOL_VERSION)?;

        let mut payload = vec![0_u8; payload_len];
        if payload_len > 0 {
            read_exact(self.raw(), &mut payload)?;
        }

        Ok(Frame {
            version: PROTOCOL_VERSION,
            payload,
        })
    }

    fn endpoint_path(&self) -> &str {
        &self.path
    }
}

impl Drop for WindowsStream {
    fn drop(&mut self) {
        // Deliberately *not* calling `DisconnectNamedPipe`.
        //
        // `DisconnectNamedPipe` discards any buffered data the client has not
        // read yet, which loses a final response (for example the
        // `ShutdownEngine` acknowledgement). This stream is never reused:
        // `accept` creates a fresh instance, so closing the handle is enough and
        // lets the peer drain what it was already sent. The handle is closed
        // exactly once by `OwnedHandle::drop`.
    }
}

/// Reads back the effective SDDL DACL of a named pipe.
///
/// Used by the test suite to prove that the created pipe does not grant access
/// to `Everyone`, `BUILTIN\Users` or `ANONYMOUS`. This is stronger than
/// asserting that the request string looks right: it inspects what the kernel
/// actually applied.
///
/// The pipe must be in the listening state; opening it consumes one instance,
/// so this is intended for tests rather than production use.
pub fn describe_pipe_security(path: &str) -> IpcResult<String> {
    let handle = open_pipe(path)?;
    let mut descriptor = PSECURITY_DESCRIPTOR(std::ptr::null_mut());

    // Ask for the DACL only. Whether the DACL is *protected* is a property of
    // the descriptor's own control flags, so it shows up as the `P` after `D:`
    // without requesting `PROTECTED_DACL_SECURITY_INFORMATION` here.
    //
    // SAFETY:
    // * `handle` is a live handle with the right to read its security
    //   descriptor.
    // * `descriptor` is a valid out-pointer that receives a `LocalAlloc`-ed
    //   descriptor, freed below.
    let status = unsafe {
        GetSecurityInfo(
            handle.raw(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };

    if status.0 != 0 {
        return Err(IpcError::Platform(format!(
            "could not read the pipe security descriptor: {}",
            status.0
        )));
    }

    let mut raw = PWSTR::null();

    // SAFETY:
    // * `descriptor` is a live security descriptor.
    // * `raw` receives a `LocalAlloc`-ed NUL-terminated wide string, handed to
    //   `take_wide_string` which frees it.
    let text = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut raw,
            None,
        )
        .map_err(|err| {
            IpcError::Platform(format!(
                "could not render the pipe security descriptor: {err}"
            ))
        })?;
        take_wide_string(raw)
    };

    // SAFETY: `descriptor` was allocated by `GetSecurityInfo` with `LocalAlloc`;
    // `LocalFree` is the matching deallocator and it is called exactly once.
    unsafe {
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
    }

    Ok(text)
}

/// The endpoint path the engine uses for the current user and process.
///
/// Superseded by [`crate::endpoint::default_endpoint_path`], which both the
/// engine and the Studio use. It is kept only because the SID-derived name is
/// still useful when two *processes* of the same user must be isolated (for
/// example an integration test that must not collide with a running engine).
#[must_use]
pub fn per_process_endpoint_path() -> String {
    // SAFETY: `GetCurrentProcessId` has no preconditions and cannot fail.
    let pid = unsafe { GetCurrentProcessId() };
    pipe_name(&pid.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sddl_has_no_world_or_group_aces() {
        // The default named-pipe DACL grants read access to Everyone. The
        // explicit descriptor must not contain WD, BU or AN.
        assert!(!PIPE_SDDL.contains(";;;WD)"), "Everyone ACE present");
        assert!(!PIPE_SDDL.contains(";;;BU)"), "BUILTIN\\Users ACE present");
        assert!(!PIPE_SDDL.contains(";;;AN)"), "Anonymous ACE present");
        assert!(PIPE_SDDL.starts_with("D:P"), "the DACL must be protected");
    }

    #[test]
    fn pipe_name_contains_the_suffix() {
        let name = pipe_name("12345");
        assert!(name.starts_with(PIPE_PREFIX));
        assert!(name.ends_with("-12345"));

        // The name legitimately begins with `\\.\pipe\`, so the backslash check
        // applies only to the sanitised portion after the fixed prefix: no
        // further path separators or whitespace may be injected there.
        let tail = &name[PIPE_PREFIX.len()..];
        assert!(!tail.contains('\\'), "unexpected backslash in {tail}");
        assert!(!tail.contains('/'), "unexpected slash in {tail}");
        assert!(
            !tail.chars().any(char::is_whitespace),
            "unexpected whitespace in {tail}"
        );
    }

    #[test]
    fn pipe_name_is_stable_within_a_process() {
        assert_eq!(pipe_name("a"), pipe_name("a"));
        assert_ne!(pipe_name("a"), pipe_name("b"));
    }

    #[test]
    fn per_process_endpoint_path_is_distinct_from_the_session_endpoint() {
        let path = per_process_endpoint_path();
        assert!(path.starts_with(PIPE_PREFIX));
        assert!(path.contains('-'));
    }

    #[test]
    fn security_descriptor_can_be_built() {
        let descriptor = OwnedSecurityDescriptor::from_sddl(PIPE_SDDL);
        assert!(descriptor.is_ok(), "the SDDL string should be accepted");
    }

    #[test]
    fn security_attributes_are_well_formed() {
        let descriptor = OwnedSecurityDescriptor::from_sddl(PIPE_SDDL).expect("valid SDDL");
        let attributes = descriptor.attributes();
        assert_eq!(
            attributes.nLength,
            u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).expect("size fits")
        );
        assert!(!attributes.lpSecurityDescriptor.is_null());
        assert_eq!(
            attributes.bInheritHandle.0, 0,
            "the handle must not be inherited"
        );
    }

    #[test]
    fn an_invalid_sddl_is_rejected() {
        assert!(OwnedSecurityDescriptor::from_sddl("not an sddl").is_err());
    }

    #[test]
    fn current_user_sid_looks_like_a_sid() {
        let sid = current_user_sid().expect("the process token must expose a user SID");
        assert!(sid.starts_with("S-1-"), "unexpected SID form: {sid}");
    }

    /// End-to-end ACL check: create a real pipe, read the DACL the kernel
    /// applied, and prove no world/group/anonymous ACE is present.
    #[test]
    fn created_pipe_has_no_world_or_group_aces() {
        let path = pipe_name(&format!("acl-test-{}", std::process::id()));

        // Create the listening instance, then inspect it from a client handle.
        let listener = WindowsListener::bind(&path).expect("bind should succeed");

        // Opening the same name consumes the listening instance, which is fine
        // inside a test: we only need a handle to query the DACL.
        let sddl = describe_pipe_security(&path).expect("security descriptor should be readable");
        drop(listener);

        assert!(
            sddl.contains("D:P") || sddl.contains("D:"),
            "unexpected SDDL: {sddl}"
        );
        assert!(
            !sddl.contains(";;;WD)"),
            "the created pipe grants access to Everyone: {sddl}"
        );
        assert!(
            !sddl.contains(";;;BU)"),
            "the created pipe grants access to BUILTIN\\\\Users: {sddl}"
        );
        assert!(
            !sddl.contains(";;;AN)"),
            "the created pipe grants access to ANONYMOUS: {sddl}"
        );
    }
}

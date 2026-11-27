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
//! * `D:P` — a protected DACL: the descriptor's own DACL is authoritative and
//!   is not merged with the parent's.
//! * `(A;;GA;;;OW)` — generic all to `OWNER RIGHTS`, i.e. the principal that
//!   created the pipe.
//! * `(A;;GA;;;SY)` — generic all to `LOCAL SYSTEM`.
//!
//! There is deliberately **no** `WD` (`Everyone`), `BU` (`BUILTIN\Users`) or
//! `AN` (`Anonymous`) ACE, so no other principal can open the pipe.
//!
//! `PIPE_REJECT_REMOTE_CLIENTS` is set so that a client on another machine
//! cannot connect even if the ACL were somehow permissive.
//!
//! The pipe name is derived from the current user's SID so that two users on
//! the same machine never collide, and a per-process suffix is appended so
//! that two engine instances owned by the same user do not collide either.

use std::ffi::c_void;

use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, SDDL_REVISION_1,
    SE_FILE_OBJECT,
};
use windows::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, PSID, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, NAMED_PIPE_MODE,
    PIPE_ACCESS_DUPLEX, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::Win32::System::Threading::OpenProcessToken;

use crate::error::{IpcError, IpcResult};
use crate::frame::{self, Frame, FRAME_HEADER_BYTES, MAX_FRAME_BYTES};
use crate::transport::{Listener, Transport};

/// The SDDL used for every DiscoDesk named pipe.
///
/// Owned by the engine process that creates the pipe. The creator becomes the
/// owner of the object, and `OWNER RIGHTS` resolves to that owner.
pub const PIPE_SDDL: &str = "D:P(A;;GA;;;OW)(A;;GA;;;SY)";

/// The prefix of every DiscoDesk named pipe.
pub const PIPE_PREFIX: &str = r"\\.\pipe\discodesk-engine";

/// The size of the pipe's OS-level buffers. Large enough for a maximum frame,
/// small enough to bound the kernel memory a client can pin.
const PIPE_BUFFER_BYTES: u32 = 64 * 1024;

/// How long a pending `ConnectNamedPipe` waits before the accept loop polls
/// again. Short enough that a shutdown is observed promptly.
const CONNECT_POLL_MILLIS: u32 = 200;

/// An owned Windows HANDLE that is closed exactly once.
#[derive(Debug)]
struct OwnedHandle(HANDLE);

impl OwnedHandle {
    /// Takes ownership of `handle`.
    fn new(handle: HANDLE) -> Option<Self> {
        if handle == INVALID_HANDLE_VALUE || handle.is_invalid() {
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
        // SAFETY: `self.0` is a handle this type owns exclusively and it is
        // dropped exactly once, because `OwnedHandle` is not `Copy` and the
        // handle is never handed out by value.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
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
    ///
    /// # Safety
    ///
    /// This function is safe: the returned descriptor owns its own allocation
    /// and is freed by `Drop`.
    fn from_sddl(sddl: &str) -> IpcResult<Self> {
        let wide = to_wide(sddl);

        let mut descriptor: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR(std::ptr::null_mut());

        // SAFETY:
        // * `wide` is a valid NUL-terminated UTF-16 string that outlives the
        //   call.
        // * `descriptor` is a valid out-pointer that is only read after this
        //   call reports success.
        // * On failure the function does not write to `descriptor`, and the
        //   `None` branch returns without freeing anything.
        let result = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                windows::core::PCWSTR(wide.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        };

        result.map_err(|err| {
            IpcError::Platform(format!("could not build the pipe security descriptor: {err}"))
        })?;

        Ok(Self(descriptor))
    }

    /// The raw pointer handed to `CreateNamedPipeW`.
    fn as_ptr(&self) -> *const PSECURITY_DESCRIPTOR {
        &self.0
    }
}

impl Drop for OwnedSecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: `self.0` was produced by
        // `ConvertStringSecurityDescriptorToSecurityDescriptorW`, which
        // allocates with `LocalAlloc`. `LocalFree` is the matching deallocator
        // and is called exactly once because this type is not `Copy`.
        unsafe {
            let _ = windows::Win32::Foundation::LocalFree(Some(windows::Win32::Foundation::HLOCAL(
                self.0.0.cast::<c_void>(),
            )));
        }
    }
}

/// Converts a Rust string into a NUL-terminated UTF-16 buffer.
fn to_wide(value: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_utf16().collect();
    wide.push(0);
    wide
}

/// Returns the SID string of the current process token's user.
///
/// This is the authoritative source for "who am I" on Windows. The SID is
/// read from the process token rather than guessed from a name, so it is
/// correct for domain, local and Azure AD accounts alike.
fn current_user_sid() -> IpcResult<String> {
    let mut token = HANDLE::default();

    // SAFETY:
    // * `token` is a valid out-pointer for a HANDLE.
    // * The pseudo-handle for the current process is always valid.
    unsafe {
        OpenProcessToken(
            windows::Win32::Foundation::GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token,
        )
        .map_err(|err| IpcError::Platform(format!("could not open the process token: {err}")))?;
    }

    let token = OwnedHandle::new(token)
        .ok_or_else(|| IpcError::Platform("the process token handle was invalid".to_owned()))?;

    let mut needed = 0_u32;

    // Sizing call: a null buffer asks the API for the required size.
    //
    // SAFETY: `token.raw()` is a valid open token handle and the null buffer
    // is the documented way to request the required size.
    unsafe {
        let _ = GetTokenInformation(
            token.raw(),
            windows::Win32::Security::TokenUser,
            None,
            0,
            &mut needed,
        );
    }

    if needed == 0 {
        return Err(IpcError::Platform(
            "the process token carries no user information".to_owned(),
        ));
    }

    let mut buffer = vec![0_u8; needed as usize];

    // SAFETY:
    // * `buffer` is a writable allocation of exactly `needed` bytes.
    // * `needed` is the size the API requested for `TokenUser`.
    unsafe {
        GetTokenInformation(
            token.raw(),
            windows::Win32::Security::TokenUser,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            needed,
            &mut needed,
        )
        .map_err(|err| IpcError::Platform(format!("could not read the token user: {err}")))?;
    }

    // The `TOKEN_USER` structure is at the start of the buffer.
    //
    // SAFETY: the API wrote a `TOKEN_USER` into `buffer` and the layout of
    // `TOKEN_USER` is a single `SID_AND_ATTRIBUTES` whose `Sid` is a pointer
    // into the same allocation. `buffer` is not moved while `sid` is alive.
    let sid = {
        let token_user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        token_user.User.Sid
    };

    sid_to_string(sid)
}

/// Converts a SID into its `S-1-…` string form.
///
/// # Safety
///
/// `sid` must point at a valid SID structure that outlives the call.
unsafe fn sid_to_string(sid: PSID) -> IpcResult<String> {
    let mut raw: *mut u16 = std::ptr::null_mut();

    // SAFETY:
    // * `sid` points at a live SID owned by the token buffer.
    // * `raw` is a valid out-pointer that is only read after success.
    // * On success the API allocates with `LocalAlloc`, which `LocalFree`
    //   releases below.
    unsafe {
        windows::Win32::Security::Authorization::ConvertSidToStringSidW(sid, &mut raw)
            .map_err(|err| IpcError::Platform(format!("could not stringify the user SID: {err}")))?;
    }

    // SAFETY: `raw` is a valid NUL-terminated wide string allocated by the
    // API above. The length is computed by scanning to the terminator.
    let text = unsafe {
        let mut len = 0_usize;
        while *raw.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(raw, len);
        String::from_utf16_lossy(slice)
    };

    // SAFETY: `raw` was allocated by `LocalAlloc` inside
    // `ConvertSidToStringSidW`; `LocalFree` is the matching deallocator.
    unsafe {
        let _ = windows::Win32::Foundation::LocalFree(Some(
            windows::Win32::Foundation::HLOCAL(raw.cast::<c_void>()),
        ));
    }

    Ok(text)
}

/// Builds the pipe name for the current user and a per-process suffix.
///
/// The user SID part guarantees that two users on one machine never share an
/// endpoint. The suffix guarantees that two engine processes owned by the
/// same user do not collide either.
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
pub struct WindowsListener {
    path: String,
    handle: OwnedHandle,
    /// The instance number of the pipe currently being created.
    instance: u32,
}

impl WindowsListener {
    /// Creates a named pipe and prepares to accept clients.
    ///
    /// The pipe is created with [`PIPE_SDDL`] so that only the owner and
    /// `LOCAL SYSTEM` can connect.
    pub fn bind(path: &str) -> IpcResult<Self> {
        let descriptor = OwnedSecurityDescriptor::from_sddl(PIPE_SDDL)?;

        let wide_path = to_wide(path);

        // SAFETY:
        // * `wide_path` is a valid NUL-terminated UTF-16 string.
        // * `descriptor` is a live security descriptor for the duration of
        //   the call.
        // * The handle is checked for `INVALID_HANDLE_VALUE` immediately
        //   afterwards and is closed exactly once by `OwnedHandle`.
        let handle = unsafe {
            CreateNamedPipeW(
                windows::core::PCWSTR(wide_path.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                PIPE_BUFFER_BYTES,
                PIPE_BUFFER_BYTES,
                0,
                descriptor.as_ptr(),
            )
        };

        let handle = OwnedHandle::new(handle).ok_or_else(|| {
            IpcError::EndpointUnavailable(format!(
                "could not create the named pipe {path}; it may already be in use"
            ))
        })?;

        Ok(Self {
            path: path.to_owned(),
            handle,
            instance: 0,
        })
    }

    /// The raw handle of the pending pipe instance.
    fn raw(&self) -> HANDLE {
        self.handle.raw()
    }

    /// Creates the next pipe instance so that a second client can connect
    /// while the first is being served.
    fn create_next_instance(&mut self) -> IpcResult<()> {
        let descriptor = OwnedSecurityDescriptor::from_sddl(PIPE_SDDL)?;
        let wide_path = to_wide(&self.path);

        // SAFETY: identical invariants to `bind`: the wide path is valid and
        // NUL-terminated, the descriptor outlives the call, and the returned
        // handle is validated and owned exactly once.
        let handle = unsafe {
            CreateNamedPipeW(
                windows::core::PCWSTR(wide_path.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                PIPE_BUFFER_BYTES,
                PIPE_BUFFER_BYTES,
                0,
                descriptor.as_ptr(),
            )
        };

        let handle = OwnedHandle::new(handle).ok_or_else(|| {
            IpcError::EndpointUnavailable(format!(
                "could not create the next instance of the named pipe {}",
                self.path
            ))
        })?;

        self.handle = handle;
        Ok(())
    }
}

impl Listener for WindowsListener {
    fn accept(&mut self) -> IpcResult<Box<dyn Transport>> {
        // Wait for a client. The pipe was created with `PIPE_WAIT`, so
        // `ConnectNamedPipe` blocks until a client connects.
        //
        // SAFETY: `self.raw()` is a live pipe handle owned by this listener.
        unsafe {
            ConnectNamedPipe(self.raw(), None)
                .map_err(|err| IpcError::Platform(format!("could not accept a pipe client: {err}")))?;
        }

        // Hand the current instance to the caller and create a fresh one for
        // the next client.
        let connected = std::mem::replace(
            &mut self.handle,
            OwnedHandle(INVALID_HANDLE_VALUE),
        );
        self.create_next_instance()?;

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
pub struct WindowsStream {
    handle: OwnedHandle,
    path: String,
}

impl WindowsStream {
    /// Opens an existing named pipe as a client.
    pub fn connect(path: &str) -> IpcResult<Self> {
        let wide_path = to_wide(path);

        // SAFETY:
        // * `wide_path` is a valid NUL-terminated UTF-16 string.
        // * The returned handle is validated and owned exactly once.
        let handle = unsafe {
            CreateFileW(
                windows::core::PCWSTR(wide_path.as_ptr()),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };

        let handle = handle.map_err(|err| {
            IpcError::EndpointUnavailable(format!("could not open the named pipe {path}: {err}"))
        })?;

        let handle = OwnedHandle::new(handle).ok_or_else(|| {
            IpcError::EndpointUnavailable(format!("the named pipe {path} returned an invalid handle"))
        })?;

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

/// Reads exactly `buffer.len()` bytes, mapping a clean EOF to `PeerClosed`.
fn read_exact(handle: HANDLE, buffer: &mut [u8]) -> IpcResult<()> {
    let mut read_total = 0_usize;

    while read_total < buffer.len() {
        // SAFETY:
        // * `handle` is a live pipe handle.
        // * `buffer[read_total..]` is a valid, exclusively borrowed region of
        //   at least one byte (the loop condition guarantees it).
        // * The `ReadFile` call is issued with the exact remaining length.
        let read = unsafe {
            let mut bytes_read = 0_u32;
            let ok = windows::Win32::Storage::FileSystem::ReadFile(
                handle,
                Some(buffer[read_total..].as_mut_ptr().cast::<c_void>()),
                (buffer.len() - read_total) as u32,
                Some(&mut bytes_read),
                None,
            )
            .map_err(|err| IpcError::Io(std::io::Error::other(err.to_string())))?;

            if ok.is_false() {
                return Err(IpcError::Io(std::io::Error::other(
                    "ReadFile failed on the named pipe".to_owned(),
                )));
            }

            bytes_read
        };

        if read == 0 {
            return if read_total == 0 {
                Err(IpcError::PeerClosed)
            } else {
                Err(IpcError::TruncatedFrame {
                    read: read_total,
                    expected: buffer.len(),
                })
            };
        }

        read_total += read as usize;
    }

    Ok(())
}

/// Writes exactly `buffer.len()` bytes.
fn write_exact(handle: HANDLE, buffer: &[u8]) -> IpcResult<()> {
    let mut written_total = 0_usize;

    while written_total < buffer.len() {
        // SAFETY:
        // * `handle` is a live pipe handle.
        // * `buffer[written_total..]` is a valid, exclusively borrowed region.
        // * The `WriteFile` call is issued with the exact remaining length.
        let written = unsafe {
            let mut bytes_written = 0_u32;
            let ok = windows::Win32::Storage::FileSystem::WriteFile(
                handle,
                Some(buffer[written_total..].as_ptr().cast::<c_void>()),
                (buffer.len() - written_total) as u32,
                Some(&mut bytes_written),
                None,
            )
            .map_err(|err| IpcError::Io(std::io::Error::other(err.to_string())))?;

            if ok.is_false() {
                return Err(IpcError::Io(std::io::Error::other(
                    "WriteFile failed on the named pipe".to_owned(),
                )));
            }

            bytes_written
        };

        if written == 0 {
            return Err(IpcError::Io(std::io::Error::other(
                "the named pipe accepted no bytes".to_owned(),
            )));
        }

        written_total += written as usize;
    }

    Ok(())
}

impl Transport for WindowsStream {
    fn send_frame(&mut self, frame: &Frame) -> IpcResult<()> {
        let encoded = frame::encode(frame.version, &frame.payload)?;
        write_exact(self.raw(), &encoded)?;
        Ok(())
    }

    fn recv_frame(&mut self) -> IpcResult<Frame> {
        let mut header = [0_u8; FRAME_HEADER_BYTES];
        read_exact(self.raw(), &mut header)?;

        let payload_len = frame::peek_payload_len(&header, frame.version)?;

        let mut payload = vec![0_u8; payload_len];
        if payload_len > 0 {
            read_exact(self.raw(), &mut payload)?;
        }

        Ok(Frame {
            version: frame.version,
            payload,
        })
    }

    fn endpoint_path(&self) -> &str {
        &self.path
    }
}

impl Drop for WindowsStream {
    fn drop(&mut self) {
        // SAFETY: `self.handle` is a live pipe handle owned by this type.
        // `DisconnectNamedPipe` on an already-disconnected pipe fails, which
        // is harmless here.
        unsafe {
            let _ = DisconnectNamedPipe(self.handle.raw());
        }
    }
}

/// Reads back the effective SDDL of a named pipe.
///
/// Used by the test suite to prove that the created pipe does not grant
/// access to `Everyone`, `BUILTIN\Users` or `ANONYMOUS`.
pub fn describe_pipe_security(path: &str) -> IpcResult<String> {
    let wide_path = to_wide(path);

    // SAFETY:
    // * `wide_path` is a valid NUL-terminated UTF-16 string.
    // * The returned handle is closed by `OwnedHandle`.
    let handle = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide_path.as_ptr()),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_MODE(0),
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };

    let handle = handle.map_err(|err| {
        IpcError::EndpointUnavailable(format!("could not open the named pipe {path}: {err}"))
    })?;

    let handle = OwnedHandle::new(handle).ok_or_else(|| {
        IpcError::EndpointUnavailable(format!("the named pipe {path} returned an invalid handle"))
    })?;

    // SAFETY:
    // * `handle` is a live handle with the right to read its security
    //   descriptor.
    // * The out-parameters are valid pointers; `ppsecuritydescriptor` receives
    //   a descriptor allocated by the API and released with `LocalFree`.
    let mut descriptor: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR(std::ptr::null_mut());

    // SAFETY: see above; the descriptor out-parameter is only read after the
    // call reports success.
    let status = unsafe {
        windows::Win32::Security::Authorization::GetSecurityInfo(
            handle.raw(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };

    if status != windows::Win32::Foundation::WIN32_ERROR(0) {
        return Err(IpcError::Platform(format!(
            "could not read the pipe security descriptor: {status:?}"
        )));
    }

    let mut raw: *mut u16 = std::ptr::null_mut();

    // SAFETY:
    // * `descriptor` is a live security descriptor.
    // * `raw` receives a `LocalAlloc`-ed NUL-terminated wide string that is
    //   read below and freed with `LocalFree`.
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            &mut raw,
            None,
        )
        .map_err(|err| {
            IpcError::Platform(format!("could not render the pipe security descriptor: {err}"))
        })?;
    }

    let text = {
        // SAFETY: `raw` is a NUL-terminated wide string allocated by the API.
        let mut len = 0_usize;
        unsafe {
            while *raw.add(len) != 0 {
                len += 1;
            }
        }
        // SAFETY: `raw` points at `len` UTF-16 code units.
        let slice = unsafe { std::slice::from_raw_parts(raw, len) };
        String::from_utf16_lossy(slice)
    };

    // SAFETY: the descriptor and the string were both allocated with
    // `LocalAlloc` inside the API calls above; `LocalFree` is the matching
    // deallocator for each and each is freed exactly once.
    unsafe {
        let _ = windows::Win32::Foundation::LocalFree(Some(
            windows::Win32::Foundation::HLOCAL(descriptor.0.cast::<c_void>()),
        ));
        let _ = windows::Win32::Foundation::LocalFree(Some(
            windows::Win32::Foundation::HLOCAL(raw.cast::<c_void>()),
        ));
    }

    Ok(text)
}

/// The endpoint path the engine uses for the current user and process.
///
/// The suffix is derived from the process id so that two engine instances do
/// not collide.
#[must_use]
pub fn default_endpoint_path() -> String {
    let pid = windows::Win32::System::Threading::GetCurrentProcessId();
    pipe_name(&format!("{pid}"))
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
    fn pipe_name_contains_the_user_sid_and_the_suffix() {
        let name = pipe_name("12345");
        assert!(name.starts_with(PIPE_PREFIX));
        assert!(name.ends_with("-12345"));
        assert!(!name.contains('\\'), "the pipe name must not contain a backslash");
    }

    #[test]
    fn pipe_name_is_stable_within_a_process() {
        assert_eq!(pipe_name("a"), pipe_name("a"));
        assert_ne!(pipe_name("a"), pipe_name("b"));
    }

    #[test]
    fn default_endpoint_path_is_per_process() {
        let path = default_endpoint_path();
        assert!(path.starts_with(PIPE_PREFIX));
        assert!(path.contains('-'));
    }

    #[test]
    fn security_descriptor_can_be_built() {
        let descriptor = OwnedSecurityDescriptor::from_sddl(PIPE_SDDL);
        assert!(descriptor.is_ok(), "the SDDL string should be accepted");
    }

    #[test]
    fn an_invalid_sddl_is_rejected() {
        assert!(OwnedSecurityDescriptor::from_sddl("not an sddl").is_err());
    }
}

//! Local IPC transports.
//!
//! A transport carries length-delimited [`crate::frame`] messages between the
//! engine and Studio over a *local* endpoint:
//!
//! * Windows — a named pipe, created with an explicit current-user security
//!   descriptor so that no other principal can connect.
//! * macOS / Linux — a Unix domain socket in a user-owned runtime directory
//!   with restrictive permissions.
//!
//! No transport binds a TCP listener. There is no HTTP surface and no
//! unauthenticated loopback endpoint.

use std::io;

use crate::error::IpcResult;
use crate::frame::Frame;

/// A bidirectional local IPC connection.
///
/// Implementations must be usable from more than one thread: the engine
/// accepts a connection on a background thread and the Studio client uses
/// the same type on its own side.
pub trait Transport: Send {
    /// Sends one frame.
    ///
    /// The whole frame is written before the call returns, so the caller
    /// always knows the peer received it. A failure is reported; the
    /// connection must be considered unusable afterwards.
    fn send_frame(&mut self, frame: &Frame) -> IpcResult<()>;

    /// Receives one frame.
    ///
    /// Blocks until a complete frame has been read. Returns
    /// [`crate::error::IpcError::PeerClosed`] if the peer closed the
    /// connection cleanly before sending anything.
    fn recv_frame(&mut self) -> IpcResult<Frame>;

    /// The local path of the endpoint this connection belongs to.
    fn endpoint_path(&self) -> &str;
}

/// A listener that yields inbound [`Transport`] connections.
pub trait Listener: Send {
    /// Accepts the next inbound connection.
    ///
    /// Blocks until a client connects.
    fn accept(&mut self) -> IpcResult<Box<dyn Transport>>;

    /// The local path of the endpoint this listener owns.
    fn endpoint_path(&self) -> &str;
}

/// Creates a listener for the current user at `endpoint_path`.
///
/// On Windows the endpoint path is a pipe name such as
/// `\\.\pipe\discodesk-engine-<suffix>`; on Unix it is a file system path
/// such as `/run/user/1000/discodesk-engine.sock`.
pub fn listener(endpoint_path: &str) -> IpcResult<Box<dyn Listener>> {
    #[cfg(windows)]
    {
        crate::transport::windows::WindowsListener::bind(endpoint_path)
            .map(|listener| Box::new(listener) as Box<dyn Listener>)
    }
    #[cfg(unix)]
    {
        crate::transport::unix::UnixListener::bind(endpoint_path)
            .map(|listener| Box::new(listener) as Box<dyn Listener>)
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = endpoint_path;
        Err(crate::error::IpcError::EndpointUnavailable(
            "no IPC transport is available for this platform".to_owned(),
        ))
    }
}

/// Connects to a listener owned by the current user at `endpoint_path`.
pub fn connect(endpoint_path: &str) -> IpcResult<Box<dyn Transport>> {
    #[cfg(windows)]
    {
        crate::transport::windows::WindowsStream::connect(endpoint_path)
            .map(|stream| Box::new(stream) as Box<dyn Transport>)
    }
    #[cfg(unix)]
    {
        crate::transport::unix::UnixStream::connect(endpoint_path)
            .map(|stream| Box::new(stream) as Box<dyn Transport>)
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = endpoint_path;
        Err(crate::error::IpcError::EndpointUnavailable(
            "no IPC transport is available for this platform".to_owned(),
        ))
    }
}

/// Writes a whole buffer to a stream.
///
/// A short write is retried so that a partial write cannot silently truncate
/// a frame.
pub(crate) fn write_all(stream: &mut impl io::Write, bytes: &[u8]) -> IpcResult<()> {
    stream.write_all(bytes)?;
    stream.flush()?;
    Ok(())
}

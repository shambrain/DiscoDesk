//! Unix domain-socket transport (macOS and Linux).
//!
//! The socket lives in a directory that only the current user can access, and
//! the socket itself is created with `0600` permissions so that no other
//! principal can connect.
//!
//! This module is `cfg(unix)`-gated at the call site in
//! [`crate::transport`], so it never participates in a Windows build.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener as StdUnixListener, UnixStream as StdUnixStream};
use std::path::{Path, PathBuf};

use crate::error::{IpcError, IpcResult};
use crate::frame::{self, FRAME_HEADER_BYTES, Frame};
use crate::transport::{Listener, Transport};

/// The file mode the socket is created with: owner read/write only.
const SOCKET_MODE: u32 = 0o600;

/// A listener that accepts Unix domain-socket connections.
pub struct UnixListener {
    path: PathBuf,
    listener: StdUnixListener,
}

impl UnixListener {
    /// Creates the socket at `path`.
    ///
    /// A stale socket file from a previous crashed run is removed first, but
    /// only if it really is a socket.
    pub fn bind(path: &str) -> IpcResult<Self> {
        let path = PathBuf::from(path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                IpcError::EndpointUnavailable(format!(
                    "could not create the socket directory {}: {err}",
                    parent.display()
                ))
            })?;
        }

        remove_stale_socket(&path)?;

        let listener = StdUnixListener::bind(&path).map_err(|err| {
            IpcError::EndpointUnavailable(format!(
                "could not bind the Unix socket {}: {err}",
                path.display()
            ))
        })?;

        fs::set_permissions(&path, fs::Permissions::from_mode(SOCKET_MODE)).map_err(|err| {
            IpcError::EndpointUnavailable(format!(
                "could not restrict the permissions of {}: {err}",
                path.display()
            ))
        })?;

        Ok(Self { path, listener })
    }
}

impl Listener for UnixListener {
    fn accept(&mut self) -> IpcResult<Box<dyn Transport>> {
        let (stream, _) = self.listener.accept().map_err(|err| {
            IpcError::Platform(format!("could not accept a Unix socket client: {err}"))
        })?;

        fs::set_permissions(&self.path, fs::Permissions::from_mode(SOCKET_MODE)).map_err(
            |err| {
                IpcError::EndpointUnavailable(format!(
                    "could not restrict the permissions of {}: {err}",
                    self.path.display()
                ))
            },
        )?;

        Ok(Box::new(UnixStream {
            stream,
            path: self.path.clone(),
        }))
    }

    fn endpoint_path(&self) -> &str {
        self.path.to_str().unwrap_or_default()
    }
}

/// A connected Unix domain-socket stream.
pub struct UnixStream {
    stream: StdUnixStream,
    path: PathBuf,
}

impl UnixStream {
    /// Connects to an existing socket.
    pub fn connect(path: &str) -> IpcResult<Self> {
        let path = PathBuf::from(path);
        let stream = StdUnixStream::connect(&path).map_err(|err| {
            IpcError::EndpointUnavailable(format!(
                "could not connect to the Unix socket {}: {err}",
                path.display()
            ))
        })?;

        Ok(Self { stream, path })
    }
}

impl Transport for UnixStream {
    fn send_frame(&mut self, frame: &Frame) -> IpcResult<()> {
        let encoded = frame::encode(frame.version, &frame.payload)?;
        self.stream.write_all(&encoded)?;
        self.stream.flush()?;
        Ok(())
    }

    fn recv_frame(&mut self) -> IpcResult<Frame> {
        let mut header = [0_u8; FRAME_HEADER_BYTES];
        let read = self.stream.read(&mut header)?;
        if read == 0 {
            return Err(IpcError::PeerClosed);
        }
        if read < FRAME_HEADER_BYTES {
            return Err(IpcError::TruncatedFrame {
                read,
                expected: FRAME_HEADER_BYTES,
            });
        }

        let payload_len = frame::peek_payload_len(&header, frame.version)?;

        let mut payload = vec![0_u8; payload_len];
        if payload_len > 0 {
            self.stream.read_exact(&mut payload)?;
        }

        Ok(Frame {
            version: frame.version,
            payload,
        })
    }

    fn endpoint_path(&self) -> &str {
        self.path.to_str().unwrap_or_default()
    }
}

/// Removes a leftover socket file, but only if it really is a socket.
///
/// A regular file at the socket path is left alone and reported as an error,
/// so that a typo cannot delete a user file.
fn remove_stale_socket(path: &Path) -> IpcResult<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(IpcError::EndpointUnavailable(format!(
                "could not inspect {}: {err}",
                path.display()
            )));
        }
    };

    if !metadata.file_type().is_socket() {
        return Err(IpcError::EndpointUnavailable(format!(
            "{} exists and is not a socket; refusing to remove it",
            path.display()
        )));
    }

    fs::remove_file(path).map_err(|err| {
        IpcError::EndpointUnavailable(format!(
            "could not remove the stale socket {}: {err}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_mode_is_owner_only() {
        assert_eq!(SOCKET_MODE, 0o600);
    }

    #[test]
    fn a_regular_file_at_the_socket_path_is_not_removed() {
        let dir = std::env::temp_dir().join("dd-ipc-unix-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir should be creatable");

        let path = dir.join("not-a-socket");
        fs::write(&path, b"important").expect("file should be writable");

        let result = remove_stale_socket(&path);
        assert!(result.is_err(), "a regular file must not be removed");
        assert!(
            path.exists(),
            "the regular file was removed; that is a data-loss bug"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_path_is_not_an_error() {
        let path = std::env::temp_dir().join("dd-ipc-unix-missing.sock");
        let _ = fs::remove_file(&path);
        assert!(remove_stale_socket(&path).is_ok());
    }
}

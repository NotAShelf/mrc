//! MRC
//! A library for interacting with the MPV media player using its JSON IPC (Inter-Process Communication) protocol.
//!
//! This crate provides a set of utilities to communicate with MPV's IPC socket, enabling you to send commands
//! and retrieve responses in a structured format.
//!
//! ## Features
//!
//! - Send commands to MPV's IPC socket
//! - Retrieve responses in JSON format
//! - Supports common MPV commands like `set_property`, `seek`, and `playlist-next`
//! - Flexible socket path configuration
//!
//! ## Example Usage
//! ```rust
//! use serde_json::json;
//! use tokio;
//! use mrc::{send_ipc_command, playlist_next, set_property};
//!
//! #[tokio::main]
//! async fn main() {
//!     let result = playlist_next(None).await;
//!     match result {
//!         Ok(response) => println!("Playlist moved to next: {:?}", response),
//!         Err(err) => eprintln!("Error: {:?}", err),
//!     }
//!
//!     let property_result = set_property("volume", &json!(50), None).await;
//!     match property_result {
//!         Ok(response) => println!("Volume set: {:?}", response),
//!         Err(err) => eprintln!("Error: {:?}", err),
//!     }
//! }
//! ```
//!
//! ## Constants
//!
//! ### `SOCKET_PATH`
//! Default path for the MPV IPC socket: `/tmp/mpvsocket`
//!

pub mod interactive;

use serde_json::{Value, json};
use std::io;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::{debug, error};

pub const SOCKET_PATH: &str = "/tmp/mpvsocket";
const SOCKET_TIMEOUT_SECS: u64 = 5;

/// Errors that can occur when interacting with the MPV IPC interface.
#[derive(Error, Debug)]
pub enum MrcError {
    /// Connection to the MPV socket could not be established.
    #[error("failed to connect to MPV socket: {0}")]
    ConnectionError(#[from] io::Error),

    /// Error when parsing a JSON response from MPV.
    #[error("failed to parse JSON response: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Error when a socket operation times out.
    #[error("socket operation timed out after {0} seconds")]
    SocketTimeout(u64),

    /// Error when MPV returns an error response.
    #[error("MPV error: {0}")]
    MpvError(String),

    /// Error when trying to use a property that doesn't exist.
    #[error("property '{0}' not found")]
    PropertyNotFound(String),

    /// Error when the socket response is not valid UTF-8.
    #[error("invalid UTF-8 in socket response: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    /// Error when a network operation fails.
    #[error("network error: {0}")]
    NetworkError(String),

    /// Error when the server connection is lost or broken.
    #[error("server connection lost: {0}")]
    ConnectionLost(String),

    /// Error when a communication protocol is violated.
    #[error("protocol error: {0}")]
    ProtocolError(String),

    /// Error when invalid input is provided.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Error related to TLS operations.
    #[error("TLS error: {0}")]
    TlsError(String),
}

/// A specialized Result type for MRC operations.
pub type Result<T> = std::result::Result<T, MrcError>;

/// Connects to the MPV IPC socket with timeout.
async fn connect_to_socket(socket_path: &str) -> Result<UnixStream> {
    debug!("Connecting to socket at {}", socket_path);

    tokio::time::timeout(
        std::time::Duration::from_secs(SOCKET_TIMEOUT_SECS),
        UnixStream::connect(socket_path),
    )
    .await
    .map_err(|_| MrcError::SocketTimeout(SOCKET_TIMEOUT_SECS))?
    .map_err(MrcError::ConnectionError)
}

/// Sends a command message to the socket with timeout.
async fn send_message(socket: &mut UnixStream, command: &str, args: &[Value]) -> Result<()> {
    let mut command_array = vec![json!(command)];
    command_array.extend_from_slice(args);
    let message = json!({ "command": command_array });
    let message_str = format!("{}\n", serde_json::to_string(&message)?);

    debug!("Serialized message to send with newline: {}", message_str);

    // Write with timeout
    tokio::time::timeout(
        std::time::Duration::from_secs(SOCKET_TIMEOUT_SECS),
        socket.write_all(message_str.as_bytes()),
    )
    .await
    .map_err(|_| MrcError::SocketTimeout(SOCKET_TIMEOUT_SECS))??;

    // Flush with timeout
    tokio::time::timeout(
        std::time::Duration::from_secs(SOCKET_TIMEOUT_SECS),
        socket.flush(),
    )
    .await
    .map_err(|_| MrcError::SocketTimeout(SOCKET_TIMEOUT_SECS))??;

    debug!("Message sent and flushed");
    Ok(())
}

/// Reads and parses the response from the socket.
async fn read_response(socket: &mut UnixStream) -> Result<Value> {
    let mut response = vec![0; 1024];

    // Read with timeout
    let n = tokio::time::timeout(
        std::time::Duration::from_secs(SOCKET_TIMEOUT_SECS),
        socket.read(&mut response),
    )
    .await
    .map_err(|_| MrcError::SocketTimeout(SOCKET_TIMEOUT_SECS))??;

    if n == 0 {
        return Err(MrcError::ConnectionLost(
            "Socket closed unexpectedly".into(),
        ));
    }

    let response_str = String::from_utf8(response[..n].to_vec())?;
    debug!("Raw response: {}", response_str);

    let json_response =
        serde_json::from_str::<Value>(&response_str).map_err(MrcError::ParseError)?;

    debug!("Parsed IPC response: {:?}", json_response);

    // Check if MPV returned an error
    if let Some(error) = json_response.get("error").and_then(|e| e.as_str()) {
        if !error.is_empty() {
            return Err(MrcError::MpvError(error.to_string()));
        }
    }

    Ok(json_response)
}

/// Sends a generic IPC command to the specified socket and returns the parsed response data.
///
/// # Arguments
/// - `command`: The name of the command to send to MPV.
/// - `args`: A slice of `Value` arguments to include in the command.
/// - `socket_path`: An optional custom path to the MPV IPC socket. If `None`, the default path is used.
///
/// # Returns
/// A `Result` containing an `Option<Value>` with the parsed response data if successful.
///
/// # Errors
/// Returns a `MrcError` if the connection to the socket fails or if the response cannot be parsed.
pub async fn send_ipc_command(
    command: &str,
    args: &[Value],
    socket_path: Option<&str>,
) -> Result<Option<Value>> {
    let socket_path = socket_path.unwrap_or(SOCKET_PATH);
    debug!(
        "Sending IPC command: {} with arguments: {:?}",
        command, args
    );

    let mut socket = connect_to_socket(socket_path).await?;
    send_message(&mut socket, command, args).await?;
    let json_response = read_response(&mut socket).await?;

    Ok(json_response.get("data").cloned())
}

/// Represents common MPV commands.
///
/// This enum provides variants for frequently used MPV commands, which can be converted to their
/// string equivalents using the `as_str` method.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
#[derive(Debug)]
pub enum MpvCommand {
    /// Sets a property to a specified value in MPV.
    SetProperty,
    /// Moves to the next item in the playlist.
    PlaylistNext,
    /// Moves to the previous item in the playlist.
    PlaylistPrev,
    /// Seeks to a specific time in the current media.
    Seek,
    /// Quits the MPV application.
    Quit,
    /// Moves an item in the playlist from one index to another.
    PlaylistMove,
    /// Removes an item from the playlist.
    PlaylistRemove,
    /// Clears all items from the playlist.
    PlaylistClear,
    /// Retrieves the value of a property in MPV.
    GetProperty,
    /// Loads a file into MPV.
    LoadFile,
}

impl MpvCommand {
    /// Converts MPV commands to their string equivalents.
    ///
    /// # Returns
    /// A string slice representing the command.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::SetProperty => "set_property",
            Self::PlaylistNext => "playlist-next",
            Self::PlaylistPrev => "playlist-prev",
            Self::Seek => "seek",
            Self::Quit => "quit",
            Self::PlaylistMove => "playlist-move",
            Self::PlaylistRemove => "playlist-remove",
            Self::PlaylistClear => "playlist-clear",
            Self::GetProperty => "get_property",
            Self::LoadFile => "loadfile",
        }
    }
}

/// Sends the `set_property` command to MPV to change a property value.
///
/// # Arguments
/// - `property`: The name of the property to set.
/// - `value`: The new value to assign to the property.
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn set_property(
    property: &str,
    value: &Value,
    socket_path: Option<&str>,
) -> Result<Option<Value>> {
    send_ipc_command(
        MpvCommand::SetProperty.as_str(),
        &[json!(property), value.clone()],
        socket_path,
    )
    .await
}

/// Sends the `playlist-next` command to move to the next playlist item.
///
/// # Arguments
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn playlist_next(socket_path: Option<&str>) -> Result<Option<Value>> {
    send_ipc_command(MpvCommand::PlaylistNext.as_str(), &[], socket_path).await
}

/// Sends the `playlist-prev` command to move to the previous playlist item.
///
/// # Arguments
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn playlist_prev(socket_path: Option<&str>) -> Result<Option<Value>> {
    send_ipc_command(MpvCommand::PlaylistPrev.as_str(), &[], socket_path).await
}

/// Sends the `seek` command to seek the media playback by a given number of seconds.
///
/// # Arguments
/// - `seconds`: The number of seconds to seek.
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn seek(seconds: f64, socket_path: Option<&str>) -> Result<Option<Value>> {
    send_ipc_command(MpvCommand::Seek.as_str(), &[json!(seconds)], socket_path).await
}

/// Sends the `quit` command to terminate MPV.
///
/// # Arguments
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn quit(socket_path: Option<&str>) -> Result<Option<Value>> {
    send_ipc_command(MpvCommand::Quit.as_str(), &[], socket_path).await
}

/// Sends the `playlist-move` command to move a playlist item from one index to another.
///
/// # Arguments
/// - `from_index`: The index of the item to move.
/// - `to_index`: The index to move the item to.
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn playlist_move(
    from_index: usize,
    to_index: usize,
    socket_path: Option<&str>,
) -> Result<Option<Value>> {
    send_ipc_command(
        MpvCommand::PlaylistMove.as_str(),
        &[json!(from_index), json!(to_index)],
        socket_path,
    )
    .await
}

/// Sends the `playlist-remove` command to remove an item from the playlist.
///
/// # Arguments
/// - `index`: The index of the item to remove, or `None` to remove the current item.
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn playlist_remove(
    index: Option<usize>,
    socket_path: Option<&str>,
) -> Result<Option<Value>> {
    let args = match index {
        Some(idx) => vec![json!(idx)],
        None => vec![json!("current")],
    };
    send_ipc_command(MpvCommand::PlaylistRemove.as_str(), &args, socket_path).await
}

/// Sends the `playlist-clear` command to clear the playlist.
///
/// # Arguments
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn playlist_clear(socket_path: Option<&str>) -> Result<Option<Value>> {
    send_ipc_command(MpvCommand::PlaylistClear.as_str(), &[], socket_path).await
}

/// Sends the `get_property` command to retrieve a property value from MPV.
///
/// # Arguments
/// - `property`: The name of the property to retrieve.
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn get_property(property: &str, socket_path: Option<&str>) -> Result<Option<Value>> {
    send_ipc_command(
        MpvCommand::GetProperty.as_str(),
        &[json!(property)],
        socket_path,
    )
    .await
}

/// Sends the `loadfile` command to load a file into MPV.
///
/// # Arguments
/// - `filename`: The name of the file to load.
/// - `append`: Whether to append the file to the playlist (`true`) or replace the current file (`false`).
/// - `socket_path`: An optional custom socket path.
///
/// # Returns
/// A `Result` containing the response data.
///
/// # Errors
/// Returns an error if the connection to the socket fails or the command execution encounters issues.
pub async fn loadfile(
    filename: &str,
    append: bool,
    socket_path: Option<&str>,
) -> Result<Option<Value>> {
    let append_flag = if append {
        json!("append-play")
    } else {
        json!("replace")
    };
    send_ipc_command(
        MpvCommand::LoadFile.as_str(),
        &[json!(filename), append_flag],
        socket_path,
    )
    .await
}

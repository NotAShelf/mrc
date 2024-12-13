use serde_json::{json, Value};
use std::io::{self};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::{debug, error};

pub const SOCKET_PATH: &str = "/tmp/mpvsocket";

/// Sends a generic IPC command to the specified socket and returns the parsed response data.
///
/// If a socket path is not provided, it will fall back to the example path of `/tmp/mpvsocket`
pub async fn send_ipc_command(
    command: &str,
    args: &[Value],
    socket_path: Option<&str>,
) -> io::Result<Option<Value>> {
    let socket_path = socket_path.unwrap_or(SOCKET_PATH);
    debug!(
        "Sending IPC command: {} with arguments: {:?}",
        command, args
    );

    match UnixStream::connect(socket_path).await {
        Ok(mut socket) => {
            debug!("Connected to socket at {}", socket_path);

            let mut command_array = vec![json!(command)];
            command_array.extend_from_slice(args);
            let message = json!({ "command": command_array });
            let message_str = format!("{}\n", serde_json::to_string(&message)?);
            debug!("Serialized message to send with newline: {}", message_str);

            socket.write_all(message_str.as_bytes()).await?;
            socket.flush().await?;
            debug!("Message sent and flushed");

            let mut response = vec![0; 1024];
            let n = socket.read(&mut response).await?;
            let response_str = String::from_utf8_lossy(&response[..n]);
            debug!("Raw response: {}", response_str);

            match serde_json::from_str::<Value>(&response_str) {
                Ok(json_response) => {
                    debug!("Parsed IPC response: {:?}", json_response);
                    Ok(json_response.get("data").cloned())
                }

                Err(e) => {
                    error!("Failed to parse response: {}", e);
                    Ok(None)
                }
            }
        }

        Err(e) => {
            error!("Failed to connect to MPV socket: {}", e);
            Err(e)
        }
    }
}

// Common MPV commands
#[derive(Debug)]
pub enum MpvCommand {
    SetProperty,
    PlaylistNext,
    PlaylistPrev,
    Seek,
    Quit,
    PlaylistMove,
    PlaylistRemove,
    PlaylistClear,
    GetProperty,
    LoadFile,
}

// Send any generic command to the MPV IPC socket.
pub async fn set_property(
    property: &str,
    value: &Value,
    socket_path: Option<&str>,
) -> io::Result<Option<Value>> {
    send_ipc_command(
        MpvCommand::SetProperty.as_str(),
        &[json!(property), value.clone()],
        socket_path,
    )
    .await
}

pub async fn playlist_next(socket_path: Option<&str>) -> io::Result<Option<Value>> {
    send_ipc_command(MpvCommand::PlaylistNext.as_str(), &[], socket_path).await
}

pub async fn playlist_prev(socket_path: Option<&str>) -> io::Result<Option<Value>> {
    send_ipc_command(MpvCommand::PlaylistPrev.as_str(), &[], socket_path).await
}

pub async fn seek(seconds: f64, socket_path: Option<&str>) -> io::Result<Option<Value>> {
    send_ipc_command(MpvCommand::Seek.as_str(), &[json!(seconds)], socket_path).await
}

pub async fn quit(socket_path: Option<&str>) -> io::Result<Option<Value>> {
    send_ipc_command(MpvCommand::Quit.as_str(), &[], socket_path).await
}

pub async fn playlist_move(
    from_index: usize,
    to_index: usize,
    socket_path: Option<&str>,
) -> io::Result<Option<Value>> {
    send_ipc_command(
        MpvCommand::PlaylistMove.as_str(),
        &[json!(from_index), json!(to_index)],
        socket_path,
    )
    .await
}

pub async fn playlist_remove(
    index: Option<usize>,
    socket_path: Option<&str>,
) -> io::Result<Option<Value>> {
    let args = match index {
        Some(idx) => vec![json!(idx)],
        None => vec![json!("current")],
    };
    send_ipc_command(MpvCommand::PlaylistRemove.as_str(), &args, socket_path).await
}

pub async fn playlist_clear(socket_path: Option<&str>) -> io::Result<Option<Value>> {
    send_ipc_command(MpvCommand::PlaylistClear.as_str(), &[], socket_path).await
}

pub async fn get_property(property: &str, socket_path: Option<&str>) -> io::Result<Option<Value>> {
    send_ipc_command(
        MpvCommand::GetProperty.as_str(),
        &[json!(property)],
        socket_path,
    )
    .await
}

pub async fn loadfile(
    filename: &str,
    append: bool,
    socket_path: Option<&str>,
) -> io::Result<Option<Value>> {
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

impl MpvCommand {
    // Convert commands to their string equivalents
    pub fn as_str(&self) -> &str {
        match self {
            MpvCommand::SetProperty => "set_property",
            MpvCommand::PlaylistNext => "playlist-next",
            MpvCommand::PlaylistPrev => "playlist-prev",
            MpvCommand::Seek => "seek",
            MpvCommand::Quit => "quit",
            MpvCommand::PlaylistMove => "playlist-move",
            MpvCommand::PlaylistRemove => "playlist-remove",
            MpvCommand::PlaylistClear => "playlist-clear",
            MpvCommand::GetProperty => "get_property",
            MpvCommand::LoadFile => "loadfile",
        }
    }
}

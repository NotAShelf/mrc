use serde_json::{Value, json};
use std::io::{self};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::{debug, error};

pub const SOCKET_PATH: &str = "/tmp/mpvsocket";

/// Sends an IPC command to the MPV socket and returns the parsed response data.
pub async fn send_ipc_command(command: &str, args: &[Value]) -> io::Result<Option<Value>> {
    debug!(
        "Sending IPC command: {} with arguments: {:?}",
        command, args
    );

    match UnixStream::connect(SOCKET_PATH).await {
        Ok(mut socket) => {
            debug!("Found MPV socket at {}", SOCKET_PATH);

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

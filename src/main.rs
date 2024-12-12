use clap::{Parser, Subcommand};
use serde_json::json;
use std::io::{self};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tracing::{debug, info, error};
use tracing_subscriber;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: CommandOptions,
}

#[derive(Subcommand)]
enum CommandOptions {
    Play { index: Option<usize> },
    Pause,
    Stop,
    Next,
    Prev,
    Seek { seconds: i32 },
    Move { index1: usize, index2: usize },
    Remove { index: Option<usize> },
    Clear,
    List,
    Add { filenames: Vec<String> },
    Replace { filenames: Vec<String> },
    Prop { properties: Vec<String> },
}

const SOCKET_PATH: &str = "/tmp/mpvsocket";
async fn send_ipc_command(command: &str, args: &[serde_json::Value]) -> io::Result<Option<serde_json::Value>> {
    debug!("Sending IPC command: {} with arguments: {:?}", command, args);

    match UnixStream::connect(SOCKET_PATH).await {
        Ok(mut socket) => {
            debug!("Connected to MPV socket successfully");

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

            match serde_json::from_str::<serde_json::Value>(&response_str) {
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

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if !PathBuf::from(SOCKET_PATH).exists() {
        debug!(SOCKET_PATH);
        error!("Error: MPV socket not found. Is MPV running?");
        return Ok(());
    }

    match cli.command {
        CommandOptions::Play { index } => {
            if let Some(idx) = index {
                info!("Playing media at index: {}", idx);
                send_ipc_command("set_property", &[json!("playlist-pos"), json!(idx)]).await?;
            }
            info!("Unpausing playback");
            send_ipc_command("set_property", &[json!("pause"), json!(false)]).await?;
        }
        CommandOptions::Pause => {
            info!("Pausing playback");
            send_ipc_command("set_property", &[json!("pause"), json!(true)]).await?;
        }
        CommandOptions::Stop => {
            info!("Stopping playback and quitting MPV");
            send_ipc_command("quit", &[]).await?;
        }
        CommandOptions::Next => {
            info!("Skipping to next item in the playlist");
            send_ipc_command("playlist-next", &[]).await?;
        }
        CommandOptions::Prev => {
            info!("Skipping to previous item in the playlist");
            send_ipc_command("playlist-prev", &[]).await?;
        }
        CommandOptions::Seek { seconds } => {
            info!("Seeking to {} seconds", seconds);
            send_ipc_command("seek", &[json!(seconds)]).await?;
        }
        CommandOptions::Move { index1, index2 } => {
            info!("Moving item from index {} to {}", index1, index2);
            send_ipc_command("playlist-move", &[json!(index1), json!(index2)]).await?;
        }
        CommandOptions::Remove { index } => {
            if let Some(idx) = index {
                info!("Removing item at index {}", idx);
                send_ipc_command("playlist-remove", &[json!(idx)]).await?;
            } else {
                info!("Removing current item from playlist");
                send_ipc_command("playlist-remove", &[json!("current")]).await?;
            }
        }
        CommandOptions::Clear => {
            info!("Clearing the playlist");
            send_ipc_command("playlist-clear", &[]).await?;
        }
        CommandOptions::List => {
            info!("Listing playlist items");
            if let Some(data) = send_ipc_command("get_property", &[json!("playlist")]).await? {
                println!("{}", serde_json::to_string_pretty(&data)?);
            }
        }
        CommandOptions::Add { filenames } => {
            info!("Adding {} files to the playlist", filenames.len());
            for filename in filenames {
                send_ipc_command("loadfile", &[json!(filename), json!("append-play")]).await?;
            }
        }
        CommandOptions::Replace { filenames } => {
            info!("Replacing current playlist with {} files", filenames.len());
            if let Some(first_file) = filenames.first() {
                send_ipc_command("loadfile", &[json!(first_file), json!("replace")]).await?;
                for filename in &filenames[1..] {
                    send_ipc_command("loadfile", &[json!(filename), json!("append-play")]).await?;
                }
            }
        }
        CommandOptions::Prop { properties } => {
            info!("Fetching properties: {:?}", properties);
            for property in properties {
                if let Some(data) = send_ipc_command("get_property", &[json!(property)]).await? {
                    println!("{}: {}", property, data);
                }
            }
        }
    }

    Ok(())
}

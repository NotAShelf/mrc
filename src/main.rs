use clap::{Parser, Subcommand};
use serde_json::json;
use std::io::{self};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

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
    let mut socket = UnixStream::connect(SOCKET_PATH).await?;
    let message = json!({ "command": [command, args] });
    let message_str = serde_json::to_string(&message)?;
    socket.write_all(message_str.as_bytes()).await?;
    socket.flush().await?;
    let mut response = vec![0; 1024];
    let n = socket.read(&mut response).await?;
    let response_str = String::from_utf8_lossy(&response[..n]);
    let json_response: Result<serde_json::Value, _> = serde_json::from_str(&response_str);

    match json_response {
        Ok(response) => Ok(response.get("data").cloned()),
        Err(e) => {
            eprintln!("Failed to parse response: {}", e);
            Ok(None)
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();
    if !PathBuf::from(SOCKET_PATH).exists() {
        eprintln!("Error: MPV socket not found. Is MPV running?");
        return Ok(());
    }
    match cli.command {
        CommandOptions::Play { index } => {
            if let Some(idx) = index {
                send_ipc_command("set_property", &[json!("playlist-pos"), json!(idx)]).await?;
            }
            send_ipc_command("set_property", &[json!("pause"), json!(false)]).await?;
        }
        CommandOptions::Pause => {
            send_ipc_command("set_property", &[json!("pause"), json!(true)]).await?;
        }
        CommandOptions::Stop => {
            send_ipc_command("quit", &[]).await?;
        }
        CommandOptions::Next => {
            send_ipc_command("playlist-next", &[]).await?;
        }
        CommandOptions::Prev => {
            send_ipc_command("playlist-prev", &[]).await?;
        }
        CommandOptions::Seek { seconds } => {
            send_ipc_command("seek", &[json!(seconds)]).await?;
        }
        CommandOptions::Move { index1, index2 } => {
            send_ipc_command("playlist-move", &[json!(index1), json!(index2)]).await?;
        }
        CommandOptions::Remove { index } => {
            if let Some(idx) = index {
                send_ipc_command("playlist-remove", &[json!(idx)]).await?;
            } else {
                send_ipc_command("playlist-remove", &[json!("current")]).await?;
            }
        }
        CommandOptions::Clear => {
            send_ipc_command("playlist-clear", &[]).await?;
        }
        CommandOptions::List => {
            if let Some(data) = send_ipc_command("get_property", &[json!("playlist")]).await? {
                println!("{}", serde_json::to_string_pretty(&data)?);
            }
        }
        CommandOptions::Add { filenames } => {
            for filename in filenames {
                send_ipc_command("loadfile", &[json!(filename), json!("append-play")]).await?;
            }
        }
        CommandOptions::Replace { filenames } => {
            if let Some(first_file) = filenames.first() {
                send_ipc_command("loadfile", &[json!(first_file), json!("replace")]).await?;
                for filename in &filenames[1..] {
                    send_ipc_command("loadfile", &[json!(filename), json!("append-play")]).await?;
                }
            }
        }
        CommandOptions::Prop { properties } => {
            for property in properties {
                if let Some(data) = send_ipc_command("get_property", &[json!(property)]).await? {
                    println!("{}: {}", property, data);
                }
            }
        }
    }

    Ok(())
}

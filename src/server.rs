use std::env;
use std::sync::Arc;

use native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_native_tls::TlsAcceptor;
use tracing::{info, debug, error};

use mrc::{set_property, get_property, playlist_clear, playlist_next, playlist_prev, quit, seek};

const AUTH_TOKEN: &str = "your_secure_token";

async fn handle_connection(
    stream: tokio::net::TcpStream,
    acceptor: Arc<TlsAcceptor>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = acceptor.accept(stream).await?;
    let mut buffer = vec![0; 2048];

    let n = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    debug!("Received request:\n{}", request);

    let headers = request.split("\r\n").collect::<Vec<&str>>();
    let token_line = headers.iter().find(|&&line| line.starts_with("Authorization:"));
    let token = match token_line {
        Some(line) => line.split(" ").nth(1).unwrap_or_default(),
        None => "",
    };

    if token != AUTH_TOKEN {
        stream.write_all(b"Authentication failed\n").await?;
        return Ok(());
    }

    info!("Client authenticated");
    stream.write_all(b"Authenticated\n").await?;

    let command = request.split("\r\n\r\n").last().unwrap_or("");
    info!("Received command: {}", command);

    let response = match process_command(command.trim()).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error processing command: {}", e);
            format!("Error: {:?}", e)
        }
    };

    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn process_command(command: &str) -> Result<String, String> {
    match command {
        "pause" => {
            info!("Pausing playback");
            set_property("pause", &json!(true), None)
                .await
                .map_err(|e| format!("Failed to pause: {:?}", e))?;
            Ok("Paused playback\n".to_string())
        }

        "play" => {
            info!("Unpausing playback");
            set_property("pause", &json!(false), None)
                .await
                .map_err(|e| format!("Failed to play: {:?}", e))?;
            Ok("Resumed playback\n".to_string())
        }

        "stop" => {
            info!("Stopping playback and quitting MPV");
            quit(None)
                .await
                .map_err(|e| format!("Failed to stop: {:?}", e))?;
            Ok("Stopped playback\n".to_string())
        }

        "next" => {
            info!("Skipping to next item in the playlist");
            playlist_next(None)
                .await
                .map_err(|e| format!("Failed to skip to next: {:?}", e))?;
            Ok("Skipped to next item\n".to_string())
        }

        "prev" => {
            info!("Skipping to previous item in the playlist");
            playlist_prev(None)
                .await
                .map_err(|e| format!("Failed to skip to previous: {:?}", e))?;
            Ok("Skipped to previous item\n".to_string())
        }

        "seek" => {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if let Some(seconds) = parts.get(1) {
                if let Ok(sec) = seconds.parse::<i32>() {
                    info!("Seeking to {} seconds", sec);
                    seek(sec.into(), None)
                        .await
                        .map_err(|e| format!("Failed to seek: {:?}", e))?;
                    return Ok(format!("Seeking to {} seconds\n", sec));
                }
            }
            Err("Invalid seek command".to_string())
        }

        "clear" => {
            info!("Clearing the playlist");
            playlist_clear(None)
                .await
                .map_err(|e| format!("Failed to clear playlist: {:?}", e))?;
            Ok("Cleared playlist\n".to_string())
        }

        "list" => {
            info!("Listing playlist items");
            match get_property("playlist", None).await {
                Ok(Some(data)) => Ok(format!(
                    "Playlist: {}",
                    serde_json::to_string_pretty(&data).unwrap()
                )),
                Ok(None) => Err("No playlist data available".to_string()),
                Err(e) => Err(format!("Failed to fetch playlist: {:?}", e)),
            }
        }
        _ => Err("Unknown command".to_string()),
    }
}

fn create_tls_acceptor() -> Result<TlsAcceptor, Box<dyn std::error::Error + Send + Sync>> {
    // FIXME: This is ugly, needs to be cleaned up.
    let pfx_path = match env::var("TLS_PFX_PATH") {
        Ok(path) => path,
        Err(_) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Environment variable TLS_PFX_PATH is missing. Please provide the path to the TLS certificate file.",
            )));
        }
    };

    let password = match env::var("TLS_PASSWORD") {
        Ok(password) => password,
        Err(_) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Environment variable TLS_PASSWORD is missing. Please provide the password for the TLS certificate.",
            )));
        }
    };

    // Try to read the PFX file and handle possible errors
    let mut file = match std::fs::File::open(&pfx_path) {
        Ok(f) => f,
        Err(e) => return Err(Box::new(e)),
    };

    let mut identity = vec![];
    if let Err(e) = std::io::Read::read_to_end(&mut file, &mut identity) {
        return Err(Box::new(e));
    }

    // Try to create Identity from PFX data
    let identity = match Identity::from_pkcs12(&identity, &password) {
        Ok(id) => id,
        Err(e) => return Err(Box::new(e)),
    };

    // Try to create TlsAcceptor from Identity
    let native_acceptor = match NativeTlsAcceptor::new(identity) {
        Ok(na) => na,
        Err(e) => return Err(Box::new(e)),
    };

    Ok(TlsAcceptor::from(native_acceptor))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    info!("Server is starting...");
    match create_tls_acceptor() {
        Ok(acceptor) => {
            let acceptor = Arc::new(acceptor);

            // TODO: This needs to be accepted by Clap, and as arguments to the program
            // But we can, for now, define those as consts that clap falls back to.
            let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
            info!("Server is listening on 127.0.0.1:8080...");

            loop {
                let (stream, _) = listener.accept().await?;
                info!("New connection accepted.");

                let acceptor = Arc::clone(&acceptor);
                tokio::spawn(handle_connection(stream, acceptor));
            }
        }

        Err(e) => {
            error!("Failed to initialize TLS: {}", e);
            return Err(e);
        }
    }
}

use std::env;
use std::io::Read;
use std::sync::Arc;

use clap::Parser;
use native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_native_tls::TlsAcceptor;
use tracing::{debug, error, info};

use mrc::{
    MrcError, Result as MrcResult, get_property, playlist_clear, playlist_next, playlist_prev,
    quit, seek, set_property,
};

#[derive(Parser)]
#[command(author, version, about)]
struct Config {
    /// The IP address and port to bind the server to
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    bind: String,

    /// Path to MPV IPC socket
    #[arg(short, long, default_value = "/tmp/mpvsocket")]
    socket: String,
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    acceptor: Arc<TlsAcceptor>,
) -> MrcResult<()> {
    let mut stream = acceptor
        .accept(stream)
        .await
        .map_err(|e| MrcError::TlsError(e.to_string()))?;
    let mut buffer = vec![0; 2048];

    let n = stream
        .read(&mut buffer)
        .await
        .map_err(MrcError::ConnectionError)?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    debug!("Received request:\n{}", request);

    let headers = request.split("\r\n").collect::<Vec<&str>>();
    let token_line = headers
        .iter()
        .find(|&&line| line.starts_with("Authorization:"));
    let token = match token_line {
        Some(line) => line.split(" ").nth(1).unwrap_or_default(),
        None => "",
    };

    let auth_token = match env::var("AUTH_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            error!("Authentication token is not set. Connection cannot be accepted.");
            stream.write_all(b"Authentication token not set\n").await?;

            // You know what? I do not care to panic when the authentication token is
            // missing in the environment. Start the goddamned server and hell, even
            // accept incoming connections. Authenticated requests will be refused
            // when the token is incorrect or not set, so we can simply continue here.
            return Ok(());
        }
    };

    if token != auth_token {
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

    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        response.len(),
        response
    );
    stream.write_all(http_response.as_bytes()).await?;

    Ok(())
}

async fn process_command(command: &str) -> MrcResult<String> {
    match command {
        "pause" => {
            info!("Pausing playback");
            set_property("pause", &json!(true), None).await?;
            Ok("Paused playback\n".to_string())
        }

        "play" => {
            info!("Unpausing playback");
            set_property("pause", &json!(false), None).await?;
            Ok("Resumed playback\n".to_string())
        }

        "stop" => {
            info!("Stopping playback and quitting MPV");
            quit(None).await?;
            Ok("Stopped playback\n".to_string())
        }

        "next" => {
            info!("Skipping to next item in the playlist");
            playlist_next(None).await?;
            Ok("Skipped to next item\n".to_string())
        }

        "prev" => {
            info!("Skipping to previous item in the playlist");
            playlist_prev(None).await?;
            Ok("Skipped to previous item\n".to_string())
        }

        "seek" => {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if let Some(seconds) = parts.get(1) {
                if let Ok(sec) = seconds.parse::<i32>() {
                    info!("Seeking to {} seconds", sec);
                    seek(sec.into(), None).await?;
                    return Ok(format!("Seeking to {} seconds\n", sec));
                }
            }
            Err(MrcError::InvalidInput("Invalid seek command".to_string()))
        }

        "clear" => {
            info!("Clearing the playlist");
            playlist_clear(None).await?;
            Ok("Cleared playlist\n".to_string())
        }

        "list" => {
            info!("Listing playlist items");
            match get_property("playlist", None).await {
                Ok(Some(data)) => {
                    let pretty_json =
                        serde_json::to_string_pretty(&data).map_err(MrcError::ParseError)?;
                    Ok(format!("Playlist: {}", pretty_json))
                }
                Ok(None) => Err(MrcError::PropertyNotFound("playlist".to_string())),
                Err(e) => Err(e),
            }
        }
        _ => Err(MrcError::InvalidInput(format!(
            "Unknown command: {}",
            command
        ))),
    }
}

fn create_tls_acceptor() -> MrcResult<TlsAcceptor> {
    let pfx_path = env::var("TLS_PFX_PATH")
        .map_err(|_| MrcError::InvalidInput("TLS_PFX_PATH not set".to_string()))?;
    let password = env::var("TLS_PASSWORD")
        .map_err(|_| MrcError::InvalidInput("TLS_PASSWORD not set".to_string()))?;

    let mut file = std::fs::File::open(&pfx_path).map_err(MrcError::ConnectionError)?;
    let mut identity = vec![];
    file.read_to_end(&mut identity)
        .map_err(MrcError::ConnectionError)?;

    let identity = Identity::from_pkcs12(&identity, &password)
        .map_err(|e| MrcError::TlsError(e.to_string()))?;
    let native_acceptor =
        NativeTlsAcceptor::new(identity).map_err(|e| MrcError::TlsError(e.to_string()))?;
    Ok(TlsAcceptor::from(native_acceptor))
}

#[tokio::main]
async fn main() -> MrcResult<()> {
    tracing_subscriber::fmt::init();
    let config = Config::parse();

    if !std::path::Path::new(&config.socket).exists() {
        error!(
            "Error: MPV socket not found at '{}'. Is MPV running?",
            config.socket
        );
        return Err(MrcError::ConnectionError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("MPV socket not found at '{}'", config.socket),
        )));
    }

    info!("Server is starting...");
    match create_tls_acceptor() {
        Ok(acceptor) => {
            let acceptor = Arc::new(acceptor);
            let listener = tokio::net::TcpListener::bind(&config.bind)
                .await
                .map_err(MrcError::ConnectionError)?;
            info!("Server is listening on {}", config.bind);

            loop {
                let (stream, _) = listener.accept().await.map_err(MrcError::ConnectionError)?;
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

use std::env;
use std::io::Read;
use std::sync::Arc;

use clap::Parser;
use native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_native_tls::TlsAcceptor;
use tracing::{debug, error, info, warn};

use mrc::{MrcError, Result as MrcResult, commands::Commands};

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
            warn!("AUTH_TOKEN environment variable not set. Authentication disabled.");
            let response = "HTTP/1.1 401 Unauthorized\r\nContent-Length: 29\r\n\r\nAuthentication token not set\n";
            stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }
    };

    if token.is_empty() || token != auth_token {
        warn!(
            "Authentication failed for token: {}",
            if token.is_empty() {
                "<empty>"
            } else {
                "<redacted>"
            }
        );
        let response =
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 21\r\n\r\nAuthentication failed\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    info!("Client authenticated successfully");

    let command = request.split("\r\n\r\n").last().unwrap_or("").trim();

    if command.is_empty() {
        warn!("Received empty command");
        let response =
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 20\r\n\r\nNo command provided\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    info!("Processing command: {}", command);

    let (status_code, response_body) = match process_command(command).await {
        Ok(response) => ("200 OK", response),
        Err(e) => {
            error!("Error processing command '{}': {}", command, e);
            ("400 Bad Request", format!("Error: {}\n", e))
        }
    };

    let http_response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
        status_code,
        response_body.len(),
        response_body
    );

    stream.write_all(http_response.as_bytes()).await?;

    Ok(())
}

async fn process_command(command: &str) -> MrcResult<String> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    match parts.as_slice() {
        ["pause"] => {
            Commands::pause().await?;
            Ok("Paused playback\n".to_string())
        }

        ["play"] => {
            Commands::play(None).await?;
            Ok("Resumed playback\n".to_string())
        }

        ["play", index] => {
            if let Ok(idx) = index.parse::<usize>() {
                Commands::play(Some(idx)).await?;
                Ok(format!("Playing from index {}\n", idx))
            } else {
                Err(MrcError::InvalidInput(format!("Invalid index: {}", index)))
            }
        }

        ["stop"] => {
            Commands::stop().await?;
            Ok("Stopped playback\n".to_string())
        }

        ["next"] => {
            Commands::next().await?;
            Ok("Skipped to next item\n".to_string())
        }

        ["prev"] => {
            Commands::prev().await?;
            Ok("Skipped to previous item\n".to_string())
        }

        ["seek", seconds] => {
            if let Ok(sec) = seconds.parse::<f64>() {
                Commands::seek_to(sec).await?;
                Ok(format!("Seeking to {} seconds\n", sec))
            } else {
                Err(MrcError::InvalidInput(format!(
                    "Invalid seconds: {}",
                    seconds
                )))
            }
        }

        ["clear"] => {
            Commands::clear_playlist().await?;
            Ok("Cleared playlist\n".to_string())
        }

        ["list"] => {
            // For server response, we need to capture the output differently
            // since Commands::list_playlist() prints to stdout
            match mrc::get_property("playlist", None).await? {
                Some(data) => {
                    let pretty_json =
                        serde_json::to_string_pretty(&data).map_err(MrcError::ParseError)?;
                    Ok(format!("Playlist: {}\n", pretty_json))
                }
                None => Ok("Playlist is empty\n".to_string()),
            }
        }

        _ => Err(MrcError::InvalidInput(format!(
            "Unknown command: {}. Available commands: pause, play [index], stop, next, prev, seek <seconds>, clear, list",
            command.trim()
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

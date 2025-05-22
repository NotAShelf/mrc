# MRC - MPV Remote Control

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

[mpv video player]: https://mpv.io/

**MRC** is a fast, robust JSON IPC wrapper for the [mpv video player]. It
provides:

- Type-safe MPV IPC communication
- Command-line interface with interactive mode
- Secure remote control over HTTPS
- Async Support with Tokio for high performance

## Features

- **Complete MPV Control**: Play, pause, seek, playlist management, and property
  access
- **Interactive CLI**: Real-time command interface with help system
- **Secure Remote Access**: TLS-encrypted server with token authentication
- **Flexible Configuration**: Custom socket paths and environment-based setup

## Quick Start

### Prerequisites

1. **Install MPV** with IPC support enabled
2. **Install Rust** (edition 2024+)

### Installation

```bash
# Clone the repository
git clone https://github.com/notashelf/mrc.git
cd mrc

# Build the project
cargo build --release

# Or install directly
cargo install --path .
```

### Basic Usage

1. **Start MPV with IPC socket**:

   ```bash
   mpv --idle --input-ipc-server=/tmp/mpvsocket
   ```

2. **Use the CLI**:

   ```bash
   # Basic playback control
   cargo run --bin cli play
   cargo run --bin cli pause
   cargo run --bin cli next

   # Load files
   cargo run --bin cli add ~/Music/song.mp3 ~/Videos/movie.mkv

   # Interactive mode
   cargo run --bin cli interactive
   ```

## Usage Guide

### CLI Commands

| Command                | Description                |            Example            |
| :--------------------- | -------------------------- | :---------------------------: |
| `play [index]`         | Start/resume playback      |  `mrc play` or `mrc play 2`   |
| `pause`                | Pause playback             |          `mrc pause`          |
| `stop`                 | Stop and quit MPV          |          `mrc stop`           |
| `next`                 | Skip to next playlist item |          `mrc next`           |
| `prev`                 | Skip to previous item      |          `mrc prev`           |
| `seek <seconds>`       | Seek to position           |        `mrc seek 120`         |
| `add <files...>`       | Add files to playlist      | `mrc add song1.mp3 song2.mp3` |
| `remove [index]`       | Remove playlist item       |        `mrc remove 0`         |
| `move <from> <to>`     | Move playlist item         |        `mrc move 0 3`         |
| `clear`                | Clear playlist             |          `mrc clear`          |
| `list`                 | Show playlist              |          `mrc list`           |
| `prop <properties...>` | Get properties             |  `mrc prop volume duration`   |
| `interactive`          | Enter interactive mode     |       `mrc interactive`       |

### Interactive Mode

Enter interactive mode for real-time control:

```bash
$ cargo run --bin cli interactive
Entering interactive mode. Type 'exit' to quit.
mpv> play
mpv> seek 60
mpv> set volume 80
mpv> get filename
mpv> help
mpv> exit
```

### Library Usage

Basic example:

```rust
use mrc::{playlist_next, set_property, get_property};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set volume to 50%
    set_property("volume", &json!(50), None).await?;

    // Get current filename
    if let Some(filename) = get_property("filename", None).await? {
        println!("Playing: {}", filename);
    }

    // Skip to next track
    playlist_next(None).await?;

    Ok(())
}
```

### Server Mode

For remote control over HTTPS:

1. **Generate TLS certificates**:

   ```bash
   # Create self-signed certificate
   openssl req -x509 -newkey rsa:4096 -keyout private_key.pem -out certificate.pem -days 365 -nodes

   # Convert to PKCS#12 format
   openssl pkcs12 -export -out identity.pfx -inkey private_key.pem -in certificate.pem
   ```

2. **Set environment variables**:

   ```bash
   export TLS_PFX_PATH="./identity.pfx"
   export TLS_PASSWORD="your_pfx_password"
   export AUTH_TOKEN="your_secure_token"
   ```

3. **Start the server**:

   ```bash
   cargo run --bin server
   # Server starts on https://127.0.0.1:8080
   ```

4. **Make requests**:

   ```bash
   # Using curl
   curl -k -H "Authorization: Bearer your_secure_token" \
        -d "pause" https://127.0.0.1:8080/

   # Using any HTTP client
   POST https://127.0.0.1:8080/
   Authorization: Bearer your_secure_token
   Content-Type: text/plain

   play
   ```

## Configuration

### Environment Variables

| Variable       | Description                      | Default | Required    |
| -------------- | -------------------------------- | ------- | ----------- |
| `TLS_PFX_PATH` | Path to PKCS#12 certificate file | -       | Server only |
| `TLS_PASSWORD` | Password for PKCS#12 file        | -       | Server only |
| `AUTH_TOKEN`   | Authentication token for server  | -       | Server only |

### Socket Path

By default, MRC uses `/tmp/mpvsocket`. You can customize this:

```rust
// In your code
use mrc::set_property;

set_property("volume", &json!(50), Some("/your/socket/path")).await?;
```

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Check linting
cargo clippy
```

### Testing MPV IPC

Test IPC communication manually:

```bash
# Send raw command to MPV socket
echo '{ "command": ["get_property", "volume"] }' | socat - /tmp/mpvsocket

# Expected response
{"data":50,"error":"success","request_id":0}
```

## Troubleshooting

### Common Issues

**Socket not found error**:

- Ensure MPV is running: `mpv --idle --input-ipc-server=/tmp/mpvsocket`
- Check socket exists: `ls -la /tmp/mpvsocket`

**Server certificate errors**:

- Verify certificate files exist and are readable
- Check environment variables are set correctly
- Test certificate: `openssl pkcs12 -info -in identity.pfx`

**Permission denied**:

- Check socket permissions: `ls -la /tmp/mpvsocket`
- Run MPV with appropriate user permissions

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug cargo run --bin cli interactive
```

## License

This project is licensed under the Mozilla Public License 2.0 - see the
[LICENSE](LICENSE) file for details.

## Acknowledgments

- [MPV Media Player](https://mpv.io/) for the excellent IPC interface

# mpvrc - MPV Remote Control

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)
[![crates.io](https://img.shields.io/crates/v/mpvrc.svg)](https://crates.io/crates/mpvrc)

[mpv video player]: https://mpv.io/

mpvrc is an unofficial command-line tool and library for controlling the
[mpv video player] via its JSON IPC socket interface. Generally designed as a
scriptable playback controller and a real-time interactive REPL in addition to a
TLS-encrypted HTTP API for _remote_ control. mpvrc has you covered for anything
that is even vaguely related to controlling a MPV instance.

## Features

The project is equipped with various neat features such as, but not limited to:

- **Full playback control**: play, pause, stop, seek, skip, and playlist
  management
- **Interactive REPL**: vi-style editing, command history, and keyboard
  shortcuts
- **HTTP server**: TLS-encrypted remote control with token authentication
- **Library API**: use MRC as a Rust crate in your own projects

It's also async-first, for high-performance concurrency.

## Prerequisites

1. **MPV** with IPC support (any recent version)
2. **Rust** 1.92.0+ (or Nix for shell-based development)

## Installation

### From source

```bash
# Clone and build using a recent cargo version
$ git clone https://github.com/notashelf/mpvrc.git
$ cd mpvrc

# Build in release mode
$ cargo build --release
```

### With Nix

The recommended way of building mpvrc is using Nix to acquire a developer shell.

```bash
# Clone and build using cargo from the provided shell
$ git clone https://github.com/notashelf/mpvrc.git
$ cd mpvrc
$ nix develop
$ cargo build --release
```

### From crates.io

You can also get mpvrc from <https://crates.io>.

```bash
# Install to ~/.cargo/bin
$ cargo install mpvrc --locked
```

## Quick Start

### 1. Start MPV with IPC

Using mpvrc is quite simple. Start `mpv` with an IPC socket first:

```bash
# `/tmp/mpvsocket` is the default socket path for mpvrc
$ mpv --idle --input-ipc-server=/tmp/mpvsocket
```

### 2. Control playback

Then, once the socket is up, you may use `mpvrc` to control MPV.

```bash
# Play/pause
$ mpvrc play
$ mpvrc pause

# Navigation
$ mpvrc next
$ mpvrc prev

# Seek 2 minutes forward
$ mpvrc seek 120

# Add files to playlist
$ mpvrc add ~/Videos/movie.mkv ~/Videos/episode1.mkv

# View playlist
$ mpvrc list

# Interactive mode
$ mpvrc interactive
```

## CLI Options

```plaintext
Usage: mpvrc [OPTIONS] <COMMAND>

Options:
  -s, --socket <PATH>    Path to MPV IPC socket [default: /tmp/mpvsocket]
  -y, --yes              Skip confirmation prompts for destructive commands
  -d, --debug            Enable debug logging
  -h, --help             Print help
  -V, --version          Print version
```

## Commands

<!--markdownlint-disable MD013-->

| Command              | Description                    | Example                        |
| :------------------- | ------------------------------ | :----------------------------- |
| `play [index]`       | Start/resume playback          | `mpvrc play` or `mpvrc play 2` |
| `pause`              | Pause playback                 | `mpvrc pause`                  |
| `stop`               | Stop and quit MPV              | `mpvrc stop`                   |
| `next`               | Skip to next playlist item     | `mpvrc next`                   |
| `prev`               | Skip to previous item          | `mpvrc prev`                   |
| `seek <seconds>`     | Seek to position in seconds    | `mpvrc seek 120`               |
| `add <files...>`     | Add files to playlist          | `mpvrc add a.mp3 b.mp3`        |
| `remove [index]`     | Remove item (default: current) | `mpvrc remove 0`               |
| `move <from> <to>`   | Move playlist item             | `mpvrc move 0 3`               |
| `clear`              | Clear playlist                 | `mpvrc clear`                  |
| `list`               | Show playlist                  | `mpvrc list`                   |
| `prop <props...>`    | Get property values            | `mpvrc prop volume`            |
| `interactive`        | Enter interactive REPL         | `mpvrc interactive`            |
| `completion <shell>` | Generate shell completions     | `mpvrc completion zsh`         |

<!--markdownlint-enable MD013-->

### Shell Completions

Generate completions for your shell:

```bash
# Zsh
$ mpvrc completion zsh > ~/.zsh/completions/_mpvrc

# Bash
$ mpvrc completion bash > /etc/bash_completion.d/mpvrc

# Fish
$ mpvrc completion fish > ~/.config/fish/completions/mpvrc.fish
```

### Interactive Mode

```bash
$ mpvrc interactive
Entering interactive mode. Type 'help' for commands or 'exit' to quit.
Socket: /tmp/mpvsocket

mpv> play
mpv> seek 60
mpv> list
  0 | Episode 1 | /path/to/episode1.mkv
> 1 | Episode 2 | /path/to/episode2.mkv
  2 | Episode 3 | /path/to/episode3.mkv
mpv> exit
```

Keyboard shortcuts in interactive mode:

- `Ctrl+C` - Interrupt current command
- `Ctrl+L` - Clear screen
- `Ctrl+D` - Exit (EOF)

## Server Mode

Run MRC as a TLS-encrypted HTTP server for remote control:

### 1. Generate TLS certificates

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
openssl pkcs12 -export -out identity.pfx -inkey key.pem -in cert.pem
```

### 2. Configure environment

```bash
export TLS_PFX_PATH="./identity.pfx"
export TLS_PASSWORD="your_password"
export AUTH_TOKEN="your_secret_token"
```

### 3. Start server

```bash
mpvrc server --bind 127.0.0.1:8080 --socket /tmp/mpvsocket
```

### 4. Send commands

```bash
curl -k -H "Authorization: Bearer your_secret_token" \
     -d "play" https://127.0.0.1:8080/
```

## Library Usage

Add MRC to your `Cargo.toml`:

```toml
[dependencies]
mpvrc = "0.2"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

Control MPV from Rust:

```rust
use mpvrc::{get_property, playlist_next, set_property};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set volume
    set_property("volume", &json!(75), None).await?;

    // Get current filename
    if let Some(filename) = get_property("filename", None).await? {
        println!("Now playing: {}", filename);
    }

    // Skip to next track
    playlist_next(None).await?;

    Ok(())
}
```

## Configuration

### Environment Variables

| Variable       | Description                     | Required |
| -------------- | ------------------------------- | -------- |
| `TLS_PFX_PATH` | Path to PKCS#12 certificate     | Server   |
| `TLS_PASSWORD` | Password for PKCS#12 file       | Server   |
| `AUTH_TOKEN`   | Bearer token for authentication | Server   |

### Custom Socket Path

Use `-s` or `--socket` to specify a different MPV socket:

```bash
# As of 0.3.0, mvrc supports custom socket path
$ mpvrc -s /var/run/mpv/socket play
```

## Building & Development

The recommended developer setup for working with mpvrc is using Nix. Use
`nix develop` or `direnv allow` to enter a reproducible devshell with the
expected tooling, then work with Rust sources as usual:

```bash
# Build
$ cargo build

# Release build
$ cargo build --release

# Run tests
$ cargo test

# Lint
$ cargo clippy --all-targets
```

## Troubleshooting

### Socket not found

```bash
# Verify MPV is running with IPC
ls -la /tmp/mpvsocket

# Restart MPV with correct socket path
mpv --idle --input-ipc-server=/tmp/mpvsocket
```

### Debug output

```bash
mpvrc -d interactive
# or
RUST_LOG=debug mpvrc interactive
```

## License

This project is made available under Mozilla Public License (MPL) version 2.0.
See [LICENSE](LICENSE) for more details on the exact conditions. An online copy
is provided [here](https://www.mozilla.org/en-US/MPL/2.0/).

use clap::{Parser, Subcommand};
use mrc::set_property;
use mrc::SOCKET_PATH;
use mrc::{
    get_property, loadfile, playlist_clear, playlist_move, playlist_next, playlist_prev,
    playlist_remove, quit, seek, MrcError, Result,
};
use serde_json::json;
use std::{io::{self, Write}, path::PathBuf};
use tracing::{debug, error, info};

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
    /// Play media at the specified index in the playlist
    Play {
        /// The index of the media to play
        index: Option<usize>,
    },

    /// Pause the currently playing media
    Pause,

    /// Stop the playback and quit MPV
    Stop,

    /// Skip to the next item in the playlist
    Next,

    /// Skip to the previous item in the playlist
    Prev,

    /// Seek to a specific position in the currently playing media
    Seek {
        /// The number of seconds to seek to
        seconds: i32,
    },

    /// Move an item in the playlist from one index to another
    Move {
        /// The index of the item to move
        index1: usize,

        /// The index to move the item to
        index2: usize,
    },

    /// Remove an item from the playlist
    ///
    /// If invoked while playlist has no entries, or if the only entry
    /// is the active video, then this will exit MPV.
    Remove {
        /// The index of the item to remove (optional)
        index: Option<usize>,
    },

    /// Clear the entire playlist
    Clear,

    /// List all the items in the playlist
    List,

    /// Add files to the playlist
    ///
    /// Needs at least one file to be passed.
    Add {
        /// The filenames of the files to add
        filenames: Vec<String>,
    },

    /// Replace the current playlist with new files
    Replace {
        /// The filenames of the files to replace the playlist with
        filenames: Vec<String>,
    },

    /// Fetch properties of the current playback or playlist
    Prop {
        /// The properties to fetch
        properties: Vec<String>,
    },

    /// Enter interactive mode to send commands to MPV IPC
    Interactive,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    if !PathBuf::from(SOCKET_PATH).exists() {
        debug!(SOCKET_PATH);
        error!("Error: MPV socket not found. Is MPV running?");
        return Err(MrcError::ConnectionError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "MPV socket not found",
        )));
    }

    match cli.command {
        CommandOptions::Play { index } => {
            if let Some(idx) = index {
                info!("Playing media at index: {}", idx);
                set_property("playlist-pos", &json!(idx), None).await?;
            }
            info!("Unpausing playback");
            set_property("pause", &json!(false), None).await?;
        }

        CommandOptions::Pause => {
            info!("Pausing playback");
            set_property("pause", &json!(true), None).await?;
        }

        CommandOptions::Stop => {
            info!("Stopping playback and quitting MPV");
            quit(None).await?;
        }

        CommandOptions::Next => {
            info!("Skipping to next item in the playlist");
            playlist_next(None).await?;
        }

        CommandOptions::Prev => {
            info!("Skipping to previous item in the playlist");
            playlist_prev(None).await?;
        }

        CommandOptions::Seek { seconds } => {
            info!("Seeking to {} seconds", seconds);
            seek(seconds.into(), None).await?;
        }

        CommandOptions::Move { index1, index2 } => {
            info!("Moving item from index {} to {}", index1, index2);
            playlist_move(index1, index2, None).await?;
        }

        CommandOptions::Remove { index } => {
            if let Some(idx) = index {
                info!("Removing item at index {}", idx);
                playlist_remove(Some(idx), None).await?;
            } else {
                info!("Removing current item from playlist");
                playlist_remove(None, None).await?;
            }
        }

        CommandOptions::Clear => {
            info!("Clearing the playlist");
            playlist_clear(None).await?;
        }

        CommandOptions::List => {
            info!("Listing playlist items");
            if let Some(data) = get_property("playlist", None).await? {
                let pretty_json = serde_json::to_string_pretty(&data)
                    .map_err(MrcError::ParseError)?;
                println!("{}", pretty_json);
            }
        }

        CommandOptions::Add { filenames } => {
            if filenames.is_empty() {
                let e = "No files provided to add to the playlist";
                error!("{}", e);
                return Err(MrcError::InvalidInput(e.to_string()));
            }

            info!("Adding {} files to the playlist", filenames.len());
            for filename in filenames {
                loadfile(&filename, true, None).await?;
            }
        }

        CommandOptions::Replace { filenames } => {
            info!("Replacing current playlist with {} files", filenames.len());
            if let Some(first_file) = filenames.first() {
                loadfile(first_file, false, None).await?;
                for filename in &filenames[1..] {
                    loadfile(filename, true, None).await?;
                }
            }
        }

        CommandOptions::Prop { properties } => {
            info!("Fetching properties: {:?}", properties);
            for property in properties {
                if let Some(data) = get_property(&property, None).await? {
                    println!("{property}: {data}");
                }
            }
        }

        CommandOptions::Interactive => {
            println!("Entering interactive mode. Type 'exit' to quit.");
            let stdin = io::stdin();
            let mut stdout = io::stdout();

            loop {
                print!("mpv> ");
                stdout.flush().map_err(MrcError::ConnectionError)?;
                let mut input = String::new();
                stdin.read_line(&mut input).map_err(MrcError::ConnectionError)?;
                let trimmed = input.trim();

                if trimmed.eq_ignore_ascii_case("exit") {
                    println!("Exiting interactive mode.");
                    break;
                }

                // I don't like this either, but it looks cleaner than a multi-line
                // print macro just cramped in here.
                let commands = vec![
                    (
                        "play [index]",
                        "Play or unpause playback, optionally at the specified index",
                    ),
                    ("pause", "Pause playback"),
                    ("stop", "Stop playback and quit MPV"),
                    ("next", "Skip to the next item in the playlist"),
                    ("prev", "Skip to the previous item in the playlist"),
                    ("seek <seconds>", "Seek to the specified position"),
                    ("clear", "Clear the playlist"),
                    ("list", "List all items in the playlist"),
                    ("add <files>", "Add files to the playlist"),
                    ("get <property>", "Get the specified property"),
                    (
                        "set <property> <value>",
                        "Set the specified property to a value",
                    ),
                    ("help", "Show this help message"),
                    ("exit", "Quit interactive mode"),
                ];

                if trimmed.eq_ignore_ascii_case("help") {
                    println!("Valid commands:");
                    for (command, description) in commands {
                        println!("  {} - {}", command, description);
                    }
                    continue;
                }

                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                match parts.as_slice() {
                    ["play"] => {
                        info!("Unpausing playback");
                        set_property("pause", &json!(false), None).await?;
                    }

                    ["play", index] => {
                        if let Ok(idx) = index.parse::<usize>() {
                            info!("Playing media at index: {}", idx);
                            set_property("playlist-pos", &json!(idx), None).await?;
                            set_property("pause", &json!(false), None).await?;
                        } else {
                            println!("Invalid index: {}", index);
                        }
                    }

                    ["pause"] => {
                        info!("Pausing playback");
                        set_property("pause", &json!(true), None).await?;
                    }

                    ["stop"] => {
                        info!("Pausing playback");
                        quit(None).await?;
                    }

                    ["next"] => {
                        info!("Skipping to next item in the playlist");
                        playlist_next(None).await?;
                    }

                    ["prev"] => {
                        info!("Skipping to previous item in the playlist");
                        playlist_prev(None).await?;
                    }

                    ["seek", seconds] => {
                        if let Ok(sec) = seconds.parse::<i32>() {
                            info!("Seeking to {} seconds", sec);
                            seek(sec.into(), None).await?;
                        } else {
                            println!("Invalid seconds: {}", seconds);
                        }
                    }

                    ["clear"] => {
                        info!("Clearing the playlist");
                        playlist_clear(None).await?;
                    }

                    ["list"] => {
                        info!("Listing playlist items");
                        if let Some(data) = get_property("playlist", None).await? {
                            let pretty_json = serde_json::to_string_pretty(&data)
                                .map_err(MrcError::ParseError)?;
                            println!("{}", pretty_json);
                        }
                    }

                    ["add", files @ ..] => {
                        if files.is_empty() {
                            println!("No files provided to add to the playlist");
                        } else {
                            info!("Adding {} files to the playlist", files.len());
                            for file in files {
                                loadfile(file, true, None).await?;
                            }
                        }
                    }

                    ["get", property] => {
                        if let Some(data) = get_property(property, None).await? {
                            println!("{property}: {data}");
                        }
                    }

                    ["set", property, value] => {
                        let json_value = serde_json::from_str::<serde_json::Value>(value)
                            .unwrap_or_else(|_| json!(value));
                        set_property(property, &json_value, None).await?;
                        println!("Set {property} to {value}");
                    }

                    _ => {
                        println!("Unknown command: {}", trimmed);
                        println!("Valid commands: play <index>, pause, stop, next, prev, seek <seconds>, clear, list, add <files>, get <property>, set <property> <value>, help, exit");
                    }
                }
            }
        }
    }

    Ok(())
}

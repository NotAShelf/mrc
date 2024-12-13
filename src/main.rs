use clap::{Parser, Subcommand};
use serde_json::json;
use std::io::{self};
use std::path::PathBuf;
use tracing::{debug, error, info};

use mrc::set_property;
use mrc::SOCKET_PATH;
use mrc::{
    get_property, loadfile, playlist_clear, playlist_move, playlist_next, playlist_prev,
    playlist_remove, quit, seek,
};

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
                println!("{}", serde_json::to_string_pretty(&data)?);
            }
        }

        CommandOptions::Add { filenames } => {
            if filenames.is_empty() {
                let e = "No files provided to add to the playlist";
                error!("{}", e);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
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
    }

    Ok(())
}

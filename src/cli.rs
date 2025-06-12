use clap::{Parser, Subcommand};
use mrc::SOCKET_PATH;
use mrc::commands::Commands;
use mrc::interactive::InteractiveMode;
use mrc::{MrcError, Result};
use std::path::PathBuf;
use tracing::{debug, error};

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
            Commands::play(index).await?;
        }

        CommandOptions::Pause => {
            Commands::pause().await?;
        }

        CommandOptions::Stop => {
            Commands::stop().await?;
        }

        CommandOptions::Next => {
            Commands::next().await?;
        }

        CommandOptions::Prev => {
            Commands::prev().await?;
        }

        CommandOptions::Seek { seconds } => {
            Commands::seek_to(seconds.into()).await?;
        }

        CommandOptions::Move { index1, index2 } => {
            Commands::move_item(index1, index2).await?;
        }

        CommandOptions::Remove { index } => {
            Commands::remove_item(index).await?;
        }

        CommandOptions::Clear => {
            Commands::clear_playlist().await?;
        }

        CommandOptions::List => {
            Commands::list_playlist().await?;
        }

        CommandOptions::Add { filenames } => {
            Commands::add_files(&filenames).await?;
        }

        CommandOptions::Replace { filenames } => {
            Commands::replace_playlist(&filenames).await?;
        }

        CommandOptions::Prop { properties } => {
            Commands::get_properties(&properties).await?;
        }

        CommandOptions::Interactive => {
            InteractiveMode::run().await?;
        }
    }

    Ok(())
}

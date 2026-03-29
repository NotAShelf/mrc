use std::{
    io::{self, Write},
    path::PathBuf,
};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use mpvrc::{MrcError, Result, SOCKET_PATH, commands::Commands, interactive::InteractiveMode};
use tracing::{debug, error};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(short, long, global = true, help = "Path to MPV socket")]
    socket: Option<String>,

    #[arg(short, long, global = true, help = "Skip confirmation prompts")]
    yes: bool,

    #[arg(short, long, global = true, help = "Enable debug output")]
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

    /// Generate shell completions
    Completion {
        #[arg(value_enum, default_value_t = Shell::Bash)]
        shell: Shell,
    },
}

#[expect(clippy::enum_variant_names)]
#[derive(Clone, ValueEnum)]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    PowerShell,
    Zsh,
}

impl Shell {
    pub fn generate(&self, app: &mut clap::Command) {
        match self {
            Self::Bash => {
                clap_complete::generate(
                    clap_complete::Shell::Bash,
                    app,
                    "mpvrc",
                    &mut io::stdout(),
                );
            }

            Self::Elvish => {
                clap_complete::generate(
                    clap_complete::Shell::Elvish,
                    app,
                    "mpvrc",
                    &mut io::stdout(),
                );
            }

            Self::Fish => {
                clap_complete::generate(
                    clap_complete::Shell::Fish,
                    app,
                    "mpvrc",
                    &mut io::stdout(),
                );
            }

            Self::PowerShell => clap_complete::generate(
                clap_complete::Shell::PowerShell,
                app,
                "mpvrc",
                &mut io::stdout(),
            ),

            Self::Zsh => {
                clap_complete::generate(clap_complete::Shell::Zsh, app, "mpvrc", &mut io::stdout());
            }
        }
    }
}

fn get_socket_path(cli: &Cli) -> String {
    cli.socket
        .clone()
        .unwrap_or_else(|| SOCKET_PATH.to_string())
}

fn confirm(prompt: &str, yes: bool) -> bool {
    if yes {
        return true;
    }
    print!("{prompt} [y/N] ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().eq_ignore_ascii_case("y")
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    if matches!(cli.command, CommandOptions::Completion { .. }) {
        let mut app = Cli::command();
        if let CommandOptions::Completion { shell } = &cli.command {
            shell.generate(&mut app);
        }
        return Ok(());
    }

    let socket_path = get_socket_path(&cli);

    if !PathBuf::from(&socket_path).exists() {
        debug!(socket_path);
        error!(
            "Error: MPV socket not found at {}. Is MPV running?",
            socket_path
        );
        return Err(MrcError::ConnectionError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "MPV socket not found",
        )));
    }

    match cli.command {
        CommandOptions::Play { index } => {
            Commands::play(index, Some(&socket_path)).await?;
        }

        CommandOptions::Pause => {
            Commands::pause(Some(&socket_path)).await?;
        }

        CommandOptions::Stop => {
            if confirm("This will stop playback and quit MPV. Continue?", cli.yes) {
                Commands::stop(Some(&socket_path)).await?;
            } else {
                println!("Cancelled.");
            }
        }

        CommandOptions::Next => {
            Commands::next(Some(&socket_path)).await?;
        }

        CommandOptions::Prev => {
            Commands::prev(Some(&socket_path)).await?;
        }

        CommandOptions::Seek { seconds } => {
            Commands::seek_to(seconds.into(), Some(&socket_path)).await?;
        }

        CommandOptions::Move { index1, index2 } => {
            Commands::move_item(index1, index2, Some(&socket_path)).await?;
        }

        CommandOptions::Remove { index } => {
            if confirm(
                &format!("This will remove item at index {index:?}. Continue?"),
                cli.yes,
            ) {
                Commands::remove_item(index, Some(&socket_path)).await?;
            } else {
                println!("Cancelled.");
            }
        }

        CommandOptions::Clear => {
            if confirm("This will clear the entire playlist. Continue?", cli.yes) {
                Commands::clear_playlist(Some(&socket_path)).await?;
            } else {
                println!("Cancelled.");
            }
        }

        CommandOptions::List => {
            Commands::list_playlist(Some(&socket_path)).await?;
        }

        CommandOptions::Add { filenames } => {
            Commands::add_files(&filenames, Some(&socket_path)).await?;
        }

        CommandOptions::Replace { filenames } => {
            Commands::replace_playlist(&filenames, Some(&socket_path)).await?;
        }

        CommandOptions::Prop { properties } => {
            Commands::get_properties(&properties, Some(&socket_path)).await?;
        }

        CommandOptions::Interactive => {
            InteractiveMode::run(Some(&socket_path)).await?;
        }

        CommandOptions::Completion { .. } => unreachable!(),
    }

    Ok(())
}

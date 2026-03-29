use crate::commands::Commands;
use crate::{MrcError, Result};
use rustyline::config::EditMode;
use rustyline::{Cmd, Config, Editor, KeyEvent};
use serde_json::json;
use std::io::Error;
use std::path::PathBuf;

pub struct InteractiveMode;

impl InteractiveMode {
    pub async fn run(socket_path: Option<&str>) -> Result<()> {
        let hist_path = dirs::data_local_dir()
            .map(|p| p.join("mrc").join("history.txt"))
            .unwrap_or_else(|| PathBuf::from("history.txt"));

        let config = Config::builder().edit_mode(EditMode::Vi).build();

        let mut rl: Editor<(), _> = Editor::with_config(config)
            .map_err(|e| MrcError::ConnectionError(Error::other(e.to_string())))?;

        rl.bind_sequence(KeyEvent::ctrl('c'), Cmd::Interrupt);
        rl.bind_sequence(KeyEvent::ctrl('l'), Cmd::ClearScreen);

        let _ = rl.load_history(&hist_path);

        println!("Entering interactive mode. Type 'help' for commands or 'exit' to quit.");
        println!("Socket: {}", socket_path.unwrap_or("/tmp/mpvsocket"));

        loop {
            let readline = rl.readline("mpv> ");
            match readline {
                Ok(input) => {
                    let trimmed = input.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    rl.add_history_entry(trimmed).ok();

                    if trimmed.eq_ignore_ascii_case("exit") {
                        println!("Exiting interactive mode.");
                        break;
                    }

                    if trimmed.eq_ignore_ascii_case("help") {
                        Self::show_help();
                        continue;
                    }

                    if let Err(e) = Self::process_command(trimmed, socket_path).await {
                        eprintln!("Error: {}", e);
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("(interrupted)");
                    continue;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("Exiting interactive mode.");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        rl.save_history(&hist_path).ok();
        Ok(())
    }

    fn show_help() {
        println!("Available commands:");
        let commands = [
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
            ("add <files...>", "Add files to the playlist"),
            ("get <property>", "Get the specified property"),
            (
                "set <property> <value>",
                "Set the specified property to a value",
            ),
            ("help", "Show this help message"),
            ("exit", "Quit interactive mode"),
        ];

        for (command, description) in commands {
            println!("  {:<22} - {}", command, description);
        }

        println!("\nKeyboard shortcuts:");
        println!("  Ctrl+C - Interrupt current command");
        println!("  Ctrl+L - Clear screen");
        println!("  Ctrl+D - Exit");
    }

    async fn process_command(input: &str, socket_path: Option<&str>) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["play"] => {
                Commands::play(None, socket_path).await?;
            }

            ["play", index] => {
                if let Ok(idx) = index.parse::<usize>() {
                    Commands::play(Some(idx), socket_path).await?;
                } else {
                    println!("Invalid index: {}", index);
                }
            }

            ["pause"] => {
                Commands::pause(socket_path).await?;
            }

            ["stop"] => {
                Commands::stop(socket_path).await?;
            }

            ["next"] => {
                Commands::next(socket_path).await?;
            }

            ["prev"] => {
                Commands::prev(socket_path).await?;
            }

            ["seek", seconds] => {
                if let Ok(sec) = seconds.parse::<i32>() {
                    Commands::seek_to(sec.into(), socket_path).await?;
                } else {
                    println!("Invalid seconds: {}", seconds);
                }
            }

            ["clear"] => {
                Commands::clear_playlist(socket_path).await?;
            }

            ["list"] => {
                Commands::list_playlist(socket_path).await?;
            }

            ["add", files @ ..] => {
                let file_strings: Vec<String> = files.iter().map(|s| s.to_string()).collect();
                if file_strings.is_empty() {
                    println!("No files provided to add to the playlist");
                } else {
                    Commands::add_files(&file_strings, socket_path).await?;
                }
            }

            ["get", property] => {
                Commands::get_single_property(property, socket_path).await?;
            }

            ["set", property, value] => {
                let json_value = serde_json::from_str::<serde_json::Value>(value)
                    .unwrap_or_else(|_| json!(value));
                Commands::set_single_property(property, &json_value, socket_path).await?;
            }

            _ => {
                println!("Unknown command: {}", input);
                println!("Type 'help' for a list of available commands.");
            }
        }

        Ok(())
    }
}


use crate::commands::Commands;
use crate::{MrcError, Result};
use serde_json::json;
use std::io::{self, Write};

pub struct InteractiveMode;

impl InteractiveMode {
    pub async fn run() -> Result<()> {
        println!("Entering interactive mode. Type 'help' for commands or 'exit' to quit.");
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("mpv> ");
            stdout.flush().map_err(MrcError::ConnectionError)?;

            let mut input = String::new();
            stdin
                .read_line(&mut input)
                .map_err(MrcError::ConnectionError)?;
            let trimmed = input.trim();

            if trimmed.eq_ignore_ascii_case("exit") {
                println!("Exiting interactive mode.");
                break;
            }

            if trimmed.eq_ignore_ascii_case("help") {
                Self::show_help();
                continue;
            }

            if let Err(e) = Self::process_command(trimmed).await {
                eprintln!("Error: {}", e);
            }
        }

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
            ("add <files>", "Add files to the playlist"),
            ("get <property>", "Get the specified property"),
            (
                "set <property> <value>",
                "Set the specified property to a value",
            ),
            ("help", "Show this help message"),
            ("exit", "Quit interactive mode"),
        ];

        for (command, description) in commands {
            println!("  {} - {}", command, description);
        }
    }

    async fn process_command(input: &str) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["play"] => {
                Commands::play(None).await?;
            }

            ["play", index] => {
                if let Ok(idx) = index.parse::<usize>() {
                    Commands::play(Some(idx)).await?;
                } else {
                    println!("Invalid index: {}", index);
                }
            }

            ["pause"] => {
                Commands::pause().await?;
            }

            ["stop"] => {
                Commands::stop().await?;
            }

            ["next"] => {
                Commands::next().await?;
            }

            ["prev"] => {
                Commands::prev().await?;
            }

            ["seek", seconds] => {
                if let Ok(sec) = seconds.parse::<i32>() {
                    Commands::seek_to(sec.into()).await?;
                } else {
                    println!("Invalid seconds: {}", seconds);
                }
            }

            ["clear"] => {
                Commands::clear_playlist().await?;
            }

            ["list"] => {
                Commands::list_playlist().await?;
            }

            ["add", files @ ..] => {
                let file_strings: Vec<String> = files.iter().map(|s| s.to_string()).collect();
                if file_strings.is_empty() {
                    println!("No files provided to add to the playlist");
                } else {
                    Commands::add_files(&file_strings).await?;
                }
            }

            ["get", property] => {
                Commands::get_single_property(property).await?;
            }

            ["set", property, value] => {
                let json_value = serde_json::from_str::<serde_json::Value>(value)
                    .unwrap_or_else(|_| json!(value));
                Commands::set_single_property(property, &json_value).await?;
            }

            _ => {
                println!("Unknown command: {}", input);
                println!("Type 'help' for a list of available commands.");
            }
        }

        Ok(())
    }
}

use crate::{
    MrcError, Result, get_property, loadfile, playlist_clear, playlist_next, playlist_prev, quit,
    seek, set_property,
};
use serde_json::json;
use std::io::{self, Write};
use tracing::info;

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
                info!("Stopping playback and quitting MPV");
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
                    let pretty_json =
                        serde_json::to_string_pretty(&data).map_err(MrcError::ParseError)?;
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
                    println!("{}: {}", property, data);
                }
            }

            ["set", property, value] => {
                let json_value = serde_json::from_str::<serde_json::Value>(value)
                    .unwrap_or_else(|_| json!(value));
                set_property(property, &json_value, None).await?;
                println!("Set {} to {}", property, value);
            }

            _ => {
                println!("Unknown command: {}", input);
                println!("Type 'help' for a list of available commands.");
            }
        }

        Ok(())
    }
}

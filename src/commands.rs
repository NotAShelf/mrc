//! Command processing module for MRC.
//!
//! # Examples
//!
//! ```rust
//! use mrc::commands::Commands;
//!
//! # async fn example() -> mrc::Result<()> {
//! // Play media at a specific playlist index
//! Commands::play(Some(0), None).await;
//!
//! // Pause playback
//! Commands::pause(None).await;
//!
//! // Add files to playlist
//! let files = vec!["movie1.mp4".to_string(), "movie2.mp4".to_string()];
//! Commands::add_files(&files, None).await;
//! # Ok(())
//! # }
//! ```

use serde_json::json;
use tracing::info;

use crate::{
    MrcError, Result, get_property, loadfile, playlist_clear, playlist_move, playlist_next,
    playlist_prev, playlist_remove, quit, seek, set_property,
};

/// Centralized command processing for MPV operations.
pub struct Commands;

impl Commands {
    /// Plays media, optionally at a specific playlist index.
    ///
    /// If an index is provided, seeks to that position in the playlist first.
    /// Always unpauses playback regardless of whether an index is specified.
    ///
    /// # Arguments
    ///
    /// * `index` - Optional playlist index to play from
    pub async fn play(index: Option<usize>, socket_path: Option<&str>) -> Result<()> {
        if let Some(idx) = index {
            info!("Playing media at index: {}", idx);
            set_property("playlist-pos", &json!(idx), socket_path).await?;
        }
        info!("Unpausing playback");
        set_property("pause", &json!(false), socket_path).await?;
        Ok(())
    }

    /// Pauses the currently playing media.
    pub async fn pause(socket_path: Option<&str>) -> Result<()> {
        info!("Pausing playback");
        set_property("pause", &json!(true), socket_path).await?;
        Ok(())
    }

    /// Stops playback and quits MPV.
    ///
    /// This is a destructive operation that will terminate the MPV process.
    pub async fn stop(socket_path: Option<&str>) -> Result<()> {
        info!("Stopping playback and quitting MPV");
        quit(socket_path).await?;
        Ok(())
    }

    /// Advances to the next item in the playlist.
    pub async fn next(socket_path: Option<&str>) -> Result<()> {
        info!("Skipping to next item in the playlist");
        playlist_next(socket_path).await?;
        Ok(())
    }

    /// Goes back to the previous item in the playlist.
    pub async fn prev(socket_path: Option<&str>) -> Result<()> {
        info!("Skipping to previous item in the playlist");
        playlist_prev(socket_path).await?;
        Ok(())
    }

    /// Seeks to a specific position in the currently playing media.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The position in seconds to seek to
    pub async fn seek_to(seconds: f64, socket_path: Option<&str>) -> Result<()> {
        info!("Seeking to {} seconds", seconds);
        seek(seconds, socket_path).await?;
        Ok(())
    }

    /// Moves an item in the playlist from one index to another.
    ///
    /// # Arguments
    ///
    /// * `from_index` - The current index of the item
    /// * `to_index` - The desired new index for the item
    pub async fn move_item(
        from_index: usize,
        to_index: usize,
        socket_path: Option<&str>,
    ) -> Result<()> {
        info!("Moving item from index {} to {}", from_index, to_index);
        playlist_move(from_index, to_index, socket_path).await?;
        Ok(())
    }

    /// Removes an item from the playlist.
    ///
    /// If no index is provided, removes the currently playing item.
    ///
    /// # Arguments
    ///
    /// * `index` - Optional specific index to remove
    pub async fn remove_item(index: Option<usize>, socket_path: Option<&str>) -> Result<()> {
        if let Some(idx) = index {
            info!("Removing item at index {}", idx);
            playlist_remove(Some(idx), socket_path).await?;
        } else {
            info!("Removing current item from playlist");
            playlist_remove(None, socket_path).await?;
        }
        Ok(())
    }

    /// Clears the entire playlist.
    pub async fn clear_playlist(socket_path: Option<&str>) -> Result<()> {
        info!("Clearing the playlist");
        playlist_clear(socket_path).await?;
        Ok(())
    }

    /// Lists all items in the playlist.
    pub async fn list_playlist(socket_path: Option<&str>) -> Result<()> {
        info!("Listing playlist items");
        if let Some(data) = get_property("playlist", socket_path).await? {
            print_playlist(&data);
        }
        Ok(())
    }

    /// Adds files to the playlist.
    ///
    /// # Arguments
    ///
    /// * `filenames` - List of file paths to add
    pub async fn add_files(filenames: &[String], socket_path: Option<&str>) -> Result<()> {
        if filenames.is_empty() {
            let e = "No files provided to add to the playlist";
            return Err(MrcError::InvalidInput(e.to_string()));
        }

        info!("Adding {} files to the playlist", filenames.len());
        for filename in filenames {
            loadfile(filename, true, socket_path).await?;
        }
        Ok(())
    }

    /// Replaces the current playlist with new files.
    ///
    /// # Arguments
    ///
    /// * `filenames` - List of file paths to replace the playlist with
    pub async fn replace_playlist(filenames: &[String], socket_path: Option<&str>) -> Result<()> {
        info!("Replacing current playlist with {} files", filenames.len());
        if let Some(first_file) = filenames.first() {
            loadfile(first_file, false, socket_path).await?;
            for filename in &filenames[1..] {
                loadfile(filename, true, socket_path).await?;
            }
        }
        Ok(())
    }

    /// Fetches multiple properties and prints them.
    ///
    /// # Arguments
    ///
    /// * `properties` - List of property names to fetch
    pub async fn get_properties(properties: &[String], socket_path: Option<&str>) -> Result<()> {
        info!("Fetching properties: {:?}", properties);
        for property in properties {
            if let Some(data) = get_property(property, socket_path).await? {
                println!("{property}: {data}");
            }
        }
        Ok(())
    }

    /// Fetches a single property and prints it.
    ///
    /// # Arguments
    ///
    /// * `property` - The property name to fetch
    pub async fn get_single_property(property: &str, socket_path: Option<&str>) -> Result<()> {
        if let Some(data) = get_property(property, socket_path).await? {
            println!("{property}: {data}");
        }
        Ok(())
    }

    /// Sets a property and prints a confirmation.
    ///
    /// # Arguments
    ///
    /// * `property` - The property name to set
    /// * `value` - The JSON value to set
    pub async fn set_single_property(
        property: &str,
        value: &serde_json::Value,
        socket_path: Option<&str>,
    ) -> Result<()> {
        set_property(property, value, socket_path).await?;
        println!("Set {property} to {value}");
        Ok(())
    }
}

fn print_playlist(data: &serde_json::Value) {
    if let Some(items) = data.as_array() {
        if items.is_empty() {
            println!("Playlist is empty.");
            return;
        }

        for (i, item) in items.iter().enumerate() {
            let title = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let filename = item
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let current = item
                .get("current")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let marker = if current { ">" } else { " " };
            println!("{} {:3} | {} | {}", marker, i, title, filename);
        }
    } else {
        let pretty_json = serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string());
        println!("{}", pretty_json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_files_empty_list() {
        let result = Commands::add_files(&[], None).await;
        assert!(result.is_err());
        if let Err(MrcError::InvalidInput(msg)) = result {
            assert_eq!(msg, "No files provided to add to the playlist");
        } else {
            panic!("Expected InvalidInput error");
        }
    }

    #[tokio::test]
    async fn test_replace_playlist_empty() {
        let result = Commands::replace_playlist(&[], None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_properties_empty() {
        let result = Commands::get_properties(&[], None).await;
        assert!(result.is_ok());
    }
}
//! Command processing module for MRC.
//!
//! # Examples
//!
//! ```rust
//! use mrc::commands::Commands;
//!
//! # async fn example() -> mrc::Result<()> {
//! // Play media at a specific playlist index
//! Commands::play(Some(0)).await?;
//!
//! // Pause playback
//! Commands::pause().await?;
//!
//! // Add files to playlist
//! let files = vec!["movie1.mp4".to_string(), "movie2.mp4".to_string()];
//! Commands::add_files(&files).await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    MrcError, Result, get_property, loadfile, playlist_clear, playlist_move, playlist_next,
    playlist_prev, playlist_remove, quit, seek, set_property,
};
use serde_json::json;
use tracing::info;

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
    pub async fn play(index: Option<usize>) -> Result<()> {
        if let Some(idx) = index {
            info!("Playing media at index: {}", idx);
            set_property("playlist-pos", &json!(idx), None).await?;
        }
        info!("Unpausing playback");
        set_property("pause", &json!(false), None).await?;
        Ok(())
    }

    /// Pauses the currently playing media.
    pub async fn pause() -> Result<()> {
        info!("Pausing playback");
        set_property("pause", &json!(true), None).await?;
        Ok(())
    }

    /// Stops playback and quits MPV.
    ///
    /// This is a destructive operation that will terminate the MPV process.
    pub async fn stop() -> Result<()> {
        info!("Stopping playback and quitting MPV");
        quit(None).await?;
        Ok(())
    }

    /// Advances to the next item in the playlist.
    pub async fn next() -> Result<()> {
        info!("Skipping to next item in the playlist");
        playlist_next(None).await?;
        Ok(())
    }

    /// Goes back to the previous item in the playlist.
    pub async fn prev() -> Result<()> {
        info!("Skipping to previous item in the playlist");
        playlist_prev(None).await?;
        Ok(())
    }

    /// Seeks to a specific time position in the current media.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The time position to seek to, in seconds
    pub async fn seek_to(seconds: f64) -> Result<()> {
        info!("Seeking to {} seconds", seconds);
        seek(seconds, None).await?;
        Ok(())
    }

    /// Moves a playlist item from one index to another.
    ///
    /// # Arguments
    ///
    /// * `from_index` - The current index of the item to move
    /// * `to_index` - The target index to move the item to
    pub async fn move_item(from_index: usize, to_index: usize) -> Result<()> {
        info!("Moving item from index {} to {}", from_index, to_index);
        playlist_move(from_index, to_index, None).await?;
        Ok(())
    }

    /// Removes an item from the playlist.
    ///
    /// # Arguments
    ///
    /// * `index` - Optional index of the item to remove. If `None`, removes the current item
    pub async fn remove_item(index: Option<usize>) -> Result<()> {
        if let Some(idx) = index {
            info!("Removing item at index {}", idx);
            playlist_remove(Some(idx), None).await?;
        } else {
            info!("Removing current item from playlist");
            playlist_remove(None, None).await?;
        }
        Ok(())
    }

    /// Clears all items from the playlist.
    pub async fn clear_playlist() -> Result<()> {
        info!("Clearing the playlist");
        playlist_clear(None).await?;
        Ok(())
    }

    /// Lists all items in the playlist.
    ///
    /// This outputs the playlist as formatted JSON to stdout.
    pub async fn list_playlist() -> Result<()> {
        info!("Listing playlist items");
        if let Some(data) = get_property("playlist", None).await? {
            let pretty_json = serde_json::to_string_pretty(&data).map_err(MrcError::ParseError)?;
            println!("{}", pretty_json);
        }
        Ok(())
    }

    /// Adds multiple files to the playlist.
    ///
    /// # Arguments
    ///
    /// * `filenames` - A slice of file paths to add to the playlist
    ///
    /// # Errors
    ///
    /// Returns [`MrcError::InvalidInput`] if the filenames slice is empty.
    pub async fn add_files(filenames: &[String]) -> Result<()> {
        if filenames.is_empty() {
            let e = "No files provided to add to the playlist";
            return Err(MrcError::InvalidInput(e.to_string()));
        }

        info!("Adding {} files to the playlist", filenames.len());
        for filename in filenames {
            loadfile(filename, true, None).await?;
        }
        Ok(())
    }

    /// Replaces the current playlist with new files.
    ///
    /// The first file replaces the current playlist, and subsequent files are appended.
    ///
    /// # Arguments
    ///
    /// * `filenames` - A slice of file paths to replace the playlist with
    pub async fn replace_playlist(filenames: &[String]) -> Result<()> {
        info!("Replacing current playlist with {} files", filenames.len());
        if let Some(first_file) = filenames.first() {
            loadfile(first_file, false, None).await?;
            for filename in &filenames[1..] {
                loadfile(filename, true, None).await?;
            }
        }
        Ok(())
    }

    /// Retrieves and displays multiple MPV properties.
    ///
    /// # Arguments
    ///
    /// * `properties` - A slice of property names to retrieve
    pub async fn get_properties(properties: &[String]) -> Result<()> {
        info!("Fetching properties: {:?}", properties);
        for property in properties {
            if let Some(data) = get_property(property, None).await? {
                println!("{property}: {data}");
            }
        }
        Ok(())
    }

    /// Retrieves and displays a single MPV property.
    ///
    /// # Arguments
    ///
    /// * `property` - The name of the property to retrieve
    pub async fn get_single_property(property: &str) -> Result<()> {
        if let Some(data) = get_property(property, None).await? {
            println!("{property}: {data}");
        }
        Ok(())
    }

    /// Sets an MPV property to a specific value.
    ///
    /// # Arguments
    ///
    /// * `property` - The name of the property to set
    /// * `value` - The JSON value to set the property to
    pub async fn set_single_property(property: &str, value: &serde_json::Value) -> Result<()> {
        set_property(property, value, None).await?;
        println!("Set {property} to {value}");
        Ok(())
    }
}

//! JSON metadata only. Never inspect ROMs or decode previews here.
use std::collections::BTreeSet;

use serde::Deserialize;

pub use imanlib::igui::{valid_hash, valid_image_name, valid_image_path};

use crate::model::{Game, Preview};

#[derive(Deserialize)]
struct Catalogue {
    games: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    hash: String,
    platform: String,
    region: Option<Vec<String>>,
    image: String,
    title: String,
    year: Option<u16>,
    players: Option<u8>,
    genre: Option<String>,
    description: Option<String>,
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Game>, String> {
    let catalogue: Catalogue = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut names = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    catalogue
        .games
        .into_iter()
        .map(|entry| {
            if !valid_hash(&entry.hash) || !hashes.insert(entry.hash.clone()) {
                return Err(format!("invalid or duplicate hash for {}", entry.image));
            }
            if !valid_image_path(&entry.image) || !names.insert(entry.image.to_lowercase()) {
                return Err(format!(
                    "invalid or duplicate image filename: {}",
                    entry.image
                ));
            }
            if entry.title.trim().is_empty()
                || entry.title.chars().any(char::is_control)
                || entry.platform.is_empty()
                || entry.platform.len() > 6
                || !entry
                    .platform
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                || entry.year.is_some_and(|y| !(1..=9999).contains(&y))
                || entry.players == Some(0)
                || entry.region.as_ref().is_some_and(|regions| {
                    regions.iter().map(String::len).sum::<usize>() > 128
                        || regions.iter().any(|s| {
                            s.trim().is_empty()
                                || s.chars().any(char::is_control)
                                || s.contains(',')
                                || matches!(s.trim(), "?" | "-")
                                || s.trim().eq_ignore_ascii_case("World")
                        })
                })
                || entry
                    .genre
                    .as_ref()
                    .is_some_and(|g| g.trim().is_empty() || g.chars().any(char::is_control))
                || entry.description.as_ref().is_some_and(|s| {
                    s.len() > 16 * 1024
                        || s.chars().any(|c| c.is_control() && !"\r\n\t".contains(c))
                })
            {
                return Err(format!("invalid metadata for {}", entry.image));
            }
            Ok(Game {
                preview: Preview::Hash(entry.hash.clone()),
                hash: entry.hash,
                image: entry.image,
                title: entry.title,
                platform: entry.platform,
                region: entry.region,
                year: entry.year,
                players: entry.players,
                genre: entry.genre,
                description: entry.description.map(|s| s.replace("\r\n", "\n")),
            })
        })
        .collect()
}

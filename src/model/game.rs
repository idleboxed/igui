// SPDX-License-Identifier: BSD-3-Clause
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preview {
    Missing,
    Hash(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    pub hash: String,
    pub image: String,
    pub title: String,
    pub platform: String,
    /// None: unknown; an empty list: no regional restrictions.
    pub region: Option<Vec<String>>,
    pub year: Option<u16>,
    pub genre: Option<String>,
    pub players: Option<u8>,
    pub description: Option<String>,
    /// Image reference only; catalog construction must not preload screenshots.
    pub preview: Preview,
}

impl Game {
    /// Create a game with its required identity and display fields, leaving optional metadata empty.
    pub fn new(hash: &str, image: &str, title: &str, platform: &str) -> Self {
        Self {
            hash: hash.into(),
            image: image.into(),
            title: title.into(),
            platform: platform.into(),
            region: None,
            year: None,
            genre: None,
            players: None,
            description: None,
            preview: Preview::Missing,
        }
    }

    /// Return the title's alphabet-filter initial, using the non-letter bucket when appropriate.
    pub fn initial(&self) -> char {
        match self.title.trim().chars().next() {
            Some(character) if character.is_numeric() => '#',
            Some(character) => character.to_uppercase().next().unwrap_or(character),
            None => '#',
        }
    }
}

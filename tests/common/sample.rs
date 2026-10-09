// SPDX-License-Identifier: BSD-3-Clause
//! Synthetic catalogue and artwork used only by host tests.
use igui::{model::{Game, Preview}, render::Renderer};

pub const CATALOGUE: &[u8] = include_bytes!("../_fixtures/games/catalogue.json");

pub fn sample_catalogue() -> Vec<Game> {
    let mut games = igui::catalogue::parse(CATALOGUE).expect("valid synthetic catalogue");
    for game in &mut games {
        game.preview = if game.title == "River Puzzle" {
            Preview::Missing
        } else {
            let hash = match game.genre.as_deref() {
                Some("Racing") => CIRCUIT,
                Some("Arcade") => SPACE,
                _ => HILLS,
            };
            Preview::Hash(hash.into())
        };
    }
    games
}

pub fn renderer() -> Renderer {
    Renderer::with_previews(|hash| {
        let bytes: &[u8] = match hash {
            HILLS => include_bytes!("../_fixtures/previews/hills.png"),
            CIRCUIT => include_bytes!("../_fixtures/previews/circuit.png"),
            SPACE => include_bytes!("../_fixtures/previews/space.png"),
            _ => return None,
        };
        Some(bytes.to_vec())
    })
}

const HILLS: &str = "c8cabf8e5aa1d9256942289d63ceb2775f8fa7c7871e86d2ca29dca9ff7e75cc";
const CIRCUIT: &str = "0af4705cc29b4a5b881e9467f5c53e5012461156b9145ed601cba870439ae3ce";
const SPACE: &str = "c1b80c42e3348ad06b3bfbbafe86f476f6238f87eb886c018dfb2a78f12a8814";

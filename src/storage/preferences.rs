// SPDX-License-Identifier: BSD-3-Clause
//! Explicit, synchronized preference replacement. No mount changes or automatic saves.
use std::{collections::BTreeSet, fs::{self, File, OpenOptions}, io::Write, path::{Path, PathBuf}};
use serde_json::{Map, Value};
use crate::{catalogue, locales, paths::PREFERENCES_FILE};
use super::resources::{read_regular, PREFS_LIMIT};

pub struct Preferences {
    root: PathBuf,
    original: Option<Vec<u8>>,
    document: Map<String, Value>,
    favourites: BTreeSet<String>,
    trash: Vec<String>,
    failed: bool,
}

impl Preferences {
    /// Read bounded preferences without migration or writing, retaining original bytes for change checks.
    pub fn load(root: &Path) -> Result<Self, String> {
        let original = read_regular(&root.join(PREFERENCES_FILE), PREFS_LIMIT)?;
        let document: Map<String, Value> = match &original {
            Some(bytes) => {
                serde_json::from_slice(bytes).map_err(|error_item| format!("{PREFERENCES_FILE}: {error_item}"))?
            }
            None => Map::from_iter([("favourites".into(), Value::Array(vec![]))]),
        };
        let entries = document
            .get("favourites")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{PREFERENCES_FILE}: favourites must be an array"))?;
        let mut favourites = BTreeSet::new();
        for value in entries {
            let name = value
                .as_str()
                .filter(|text| catalogue::valid_hash(text))
                .ok_or_else(|| format!("{PREFERENCES_FILE}: invalid favourite SHA-256"))?;
            favourites.insert(name.to_owned());
        }
        // Existing preferences without trash are valid; never migrate on load.
        let mut trash = Vec::new();
        if let Some(value) = document.get("trash") {
            let mut seen = BTreeSet::new();
            for value in value
                .as_array()
                .ok_or_else(|| format!("{PREFERENCES_FILE}: trash must be an array"))?
            {
                let hash = value
                    .as_str()
                    .filter(|text| catalogue::valid_hash(text))
                    .ok_or_else(|| format!("{PREFERENCES_FILE}: invalid trash SHA-256"))?;
                if seen.insert(hash) {
                    trash.push(hash.to_owned());
                }
            }
        }
        Ok(Self {
            root: root.to_owned(),
            original,
            document,
            favourites,
            trash,
            failed: false,
        })
    }

    /// Borrow the loaded or successfully saved favourite hashes.
    pub fn favourites(&self) -> &BTreeSet<String> {
        &self.favourites
    }

    /// Confirmed deletions in newest-first order, including hashes absent from the catalogue.
    pub fn trash(&self) -> &[String] {
        &self.trash
    }

    /// Return the valid requested language code, defaulting to English.
    pub fn language(&self) -> &str {
        self.document
            .get("lang")
            .and_then(Value::as_str)
            .filter(|code| locales::valid_code(code))
            .unwrap_or("en")
    }

    /// Invalid/stale selection metadata must not prevent browsing the catalogue.
    pub fn current_item(&self) -> Option<&str> {
        self.document
            .get("current_item")
            .and_then(Value::as_str)
            .filter(|hash| catalogue::valid_hash(hash))
    }

    /// Explicit launch only; an unchanged hash does not rewrite preferences.
    pub fn save_current_item(&mut self, hash: &str) -> Result<(), String> {
        if !catalogue::valid_hash(hash) {
            return Err("invalid current item SHA-256".into());
        }
        let mut document = self.document.clone();
        document.insert("current_item".into(), Value::String(hash.to_owned()));
        self.save_document(document)
    }

    /// Explicit user action only. Publish to the model after this succeeds.
    /// Same-directory replacement + fsync is not proof of SD power-loss safety.
    pub fn save_favourite(&mut self, hash: &str, enabled: bool) -> Result<(), String> {
        if self.failed {
            return Err(format!(
                "{PREFERENCES_FILE}: saving disabled after an error; restart and check the file"
            ));
        }
        if !catalogue::valid_hash(hash) {
            return Err("invalid favourite SHA-256".into());
        }
        if self.favourites.contains(hash) == enabled {
            return Ok(());
        }
        let mut favourites = self.favourites.clone();
        if enabled {
            favourites.insert(hash.to_owned());
        } else {
            favourites.remove(hash);
        }
        let mut document = self.document.clone();
        document.insert(
            "favourites".into(),
            serde_json::to_value(&favourites).map_err(|error_item| error_item.to_string())?,
        );
        self.save_document(document)?;
        self.favourites = favourites;
        Ok(())
    }

    /// Explicit soft deletion/restoration. ROMs, previews and favourites remain intact.
    pub fn save_trash(&mut self, hash: &str, enabled: bool) -> Result<(), String> {
        if self.failed {
            return Err(format!(
                "{PREFERENCES_FILE}: saving disabled after an error; restart and check the file"
            ));
        }
        if !catalogue::valid_hash(hash) {
            return Err("invalid trash SHA-256".into());
        }
        if self.trash.iter().any(|entry| entry == hash) == enabled {
            return Ok(());
        }
        let mut trash = self.trash.clone();
        if enabled {
            trash.insert(0, hash.to_owned());
        } else {
            trash.retain(|entry| entry != hash);
        }
        let mut document = self.document.clone();
        document.insert(
            "trash".into(),
            serde_json::to_value(&trash).map_err(|error_item| error_item.to_string())?,
        );
        self.save_document(document)?;
        self.trash = trash;
        Ok(())
    }

    fn save_document(&mut self, document: Map<String, Value>) -> Result<(), String> {
        if self.failed {
            return Err(format!(
                "{PREFERENCES_FILE}: saving disabled after an error; restart and check the file"
            ));
        }
        if document == self.document {
            return Ok(());
        }
        let mut bytes = serde_json::to_vec_pretty(&document).map_err(|error_item| error_item.to_string())?;
        bytes.push(b'\n');
        if bytes.len() > PREFS_LIMIT {
            return Err(format!("{PREFERENCES_FILE}: file too large"));
        }
        if let Err(error) = self.replace(&bytes) {
            // A late sync failure can mean rename already happened. Do not
            // claim rollback or silently retry with an obsolete memory state.
            self.failed = true;
            return Err(format!("{PREFERENCES_FILE}: save not confirmed: {error}"));
        }
        self.original = Some(bytes);
        self.document = document;
        Ok(())
    }

    fn replace(&self, bytes: &[u8]) -> Result<(), String> {
        let path = self.root.join(PREFERENCES_FILE);
        // create_new serialises cooperating writers; never delete someone
        // else's temporary file or a stale one left by an interrupted process.
        let temp = self.root.join(format!(".{PREFERENCES_FILE}.tmp"));
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|error_item| format!("{}: {error_item}", temp.display()))?;
        let result = (|| {
            if read_regular(&path, PREFS_LIMIT)? != self.original {
                return Err("preferences changed externally; reload before saving".into());
            }
            output
                .write_all(bytes)
                .and_then(|()| output.sync_all())
                .map_err(|error_item| error_item.to_string())?;
            drop(output);
            fs::rename(&temp, &path).map_err(|error_item| error_item.to_string())?;
            File::open(&self.root)
                .and_then(|dir| dir.sync_all())
                .map_err(|error_item| error_item.to_string())
        })();
        if result.is_err() {
            // Only our own create_new file is eligible for cleanup.
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

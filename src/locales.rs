//! One English-keyed translation dictionary; built-in English is the fallback.
use std::collections::BTreeMap;

pub fn valid_code(code: &str) -> bool {
    (2..=3).contains(&code.len()) && code.bytes().all(|c| c.is_ascii_lowercase())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locale {
    code: String,
    strings: BTreeMap<String, String>,
}

impl Locale {
    pub fn parse(code: &str, bytes: &[u8]) -> Result<Self, String> {
        if !valid_code(code) || code == "en" {
            return Err("locale filename must be a language code other than en".into());
        }
        let strings: BTreeMap<String, String> =
            serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if strings.iter().any(|(key, value)| {
            key.trim().is_empty()
                || value.trim().is_empty()
                || key.chars().any(char::is_control)
                || value.chars().any(char::is_control)
        }) {
            return Err("translations must be nonempty single-line strings".into());
        }
        Ok(Self {
            code: code.into(),
            strings,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Locales {
    translation: Option<Locale>,
}

impl Locales {
    pub fn new(translation: Locale) -> Self {
        Self {
            translation: Some(translation),
        }
    }

    pub fn language(&self) -> &str {
        self.translation
            .as_ref()
            .map_or("en", |locale| &locale.code)
    }

    pub fn text<'a>(&'a self, english: &'a str) -> &'a str {
        self.translation
            .as_ref()
            .and_then(|locale| locale.strings.get(english))
            .map_or(english, String::as_str)
    }
}

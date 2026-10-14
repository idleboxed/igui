// SPDX-License-Identifier: BSD-3-Clause
//! Bounded resource reads and read-only filesystem queries.
use std::{fs::{self, File}, io::{self, Read}, path::Path};
use crate::{catalogue, locales::{self, Locale, Locales}, model::Game, paths};

const CATALOGUE_LIMIT: usize = 16 * 1024 * 1024;
pub(super) const PREFS_LIMIT: usize = 1024 * 1024;

/// Available to this user on the filesystem containing games, not the GUI binary.
/// Read-only query, performed on load and after an explicit preference save.
#[cfg(target_os = "linux")]
pub fn available_bytes(root: &Path) -> Option<u64> {
    use std::{ffi::CString, mem::MaybeUninit, os::unix::ffi::OsStrExt};
    let path = CString::new(root.as_os_str().as_bytes()).ok()?;
    let mut info = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: path is NUL-terminated; statvfs initializes info on success.
    if unsafe { libc::statvfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
        return None;
    }
    let info = unsafe { info.assume_init() };
    u64::try_from(u128::from(info.f_bavail) * u128::from(info.f_frsize)).ok()
}

#[cfg(not(target_os = "linux"))]
/// Return None when this platform has no supported free-space query.
pub fn available_bytes(_root: &Path) -> Option<u64> {
    None
}

/// Read a bounded regular resource file, returning None if it does not exist.
pub(super) fn read_regular(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Err(error_item) if error_item.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error_item) => return Err(error_item.to_string()),
        Ok(meta) if !meta.is_file() || meta.len() > limit as u64 => {
            return Err(format!(
                "not a regular file or too large: {}",
                path.display()
            ));
        }
        Ok(_) => {}
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(limit as u64 + 1).read_to_end(&mut bytes))
        .map_err(|error_item| error_item.to_string())?;
    if bytes.len() > limit {
        return Err(format!("file too large: {}", path.display()));
    }
    Ok(Some(bytes))
}

/// Read and validate the game catalogue from the selected games directory.
pub fn load_catalogue(root: &Path) -> Result<Vec<Game>, String> {
    let path = root.join(paths::CATALOGUE_FILE);
    let bytes = read_regular(&path, CATALOGUE_LIMIT)?
        .ok_or_else(|| format!("missing {}", path.display()))?;
    catalogue::parse(&bytes).map_err(|error_item| format!("{}: {error_item}", path.display()))
}

/// Read only the requested dictionary, without discovering other languages.
/// Missing/invalid preferences or an unavailable translation use built-in English.
pub fn load_locales(root: &Path, code: &str) -> Locales {
    if code == "en" || !locales::valid_code(code) {
        return Locales::default();
    }
    let path = root.join(format!("{code}.json"));
    match read_regular(&path, 64 * 1024)
        .and_then(|bytes| bytes.map(|bytes| Locale::parse(code, &bytes)).transpose())
    {
        Ok(Some(locale)) => Locales::new(locale),
        Ok(None) => Locales::default(),
        Err(error) => {
            eprintln!("IGUI locale {}: {error}; using English", path.display());
            Locales::default()
        }
    }
}

/// Called only for the selected row. Escaping symlinks are not followed for data.
pub fn load_preview(root: &Path, hash: &str) -> Option<Vec<u8>> {
    if !catalogue::valid_hash(hash) {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let thumbs = root.join(paths::THUMBS_DIR);
    let path = thumbs
        .join(&hash[..2])
        .join(format!("{hash}.png"))
        .canonicalize()
        .ok()?;
    if !path.starts_with(&thumbs) {
        return None;
    }
    read_regular(&path, 1024 * 1024).ok()?
}

// SPDX-License-Identifier: BSD-3-Clause
//! Explicit PNG output: one temporary RAM capture, or a caller-selected new file.
use imanlib::surface::Surface;
use imanlib::filesystem::read_filesystem;
use std::{
    fs::{File, OpenOptions},
    io,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

pub struct Screenshot {
    _file: NamedTempFile,
    // Keep the anchored path alive until NamedTempFile has removed its entry.
    _directory: File,
    pub path: PathBuf,
}
impl Screenshot {
    pub fn capture(socket: &Path, surface: &Surface) -> io::Result<Self> {
        let parent = socket
            .parent()
            .filter(|p| p.is_absolute())
            .ok_or_else(|| io::Error::other("IPC socket path must be absolute"))?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(parent)?;
        let metadata = directory.metadata()?;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
            return Err(io::Error::other(
                "Screenshot requires a private session directory",
            ));
        }
        if !read_filesystem(&directory)?.is_tmpfs {
            return Err(io::Error::other(
                "Screenshot requires tmpfs; no disk fallback",
            ));
        }
        let mut file = tempfile::Builder::new()
            .prefix("screenshot-")
            .suffix(".png")
            .tempfile_in(format!("/proc/self/fd/{}", directory.as_raw_fd()))?;
        let path = parent.join(file.path().file_name().unwrap());
        write_png(file.as_file_mut(), surface)?;
        Ok(Self {
            _file: file,
            _directory: directory,
            path,
        })
    }
}

/// Save explicitly, without creating directories or replacing an existing entry.
/// Publish only a complete PNG; a post-publication sync failure leaves the file.
pub fn save(path: &Path, surface: &Surface) -> io::Result<()> {
    let raw_name = path
        .as_os_str()
        .as_encoded_bytes()
        .rsplit(|byte| *byte == b'/')
        .next()
        .unwrap_or_default();
    let filename = path
        .file_name()
        .filter(|_| {
            path.is_absolute()
                && !raw_name.is_empty()
                && raw_name != b"."
                && raw_name != b".."
                && !path
                    .components()
                    .any(|p| matches!(p, std::path::Component::ParentDir))
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Screenshot path must be absolute and include a filename, without '..'",
            )
        })?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "Screenshot path has no parent")
    })?;
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(parent)?;
    let anchored = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
    let destination = anchored.join(filename);
    match std::fs::symlink_metadata(&destination) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Screenshot destination already exists; refusing to overwrite",
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut file = tempfile::Builder::new()
        .prefix(".screenshot-")
        .suffix(".partial")
        .tempfile_in(&anchored)?;
    write_png(file.as_file_mut(), surface)?;
    file.as_file().sync_all()?;
    // persist_noclobber closes the race with a concurrently created destination.
    file.persist_noclobber(&destination)
        .map_err(|error| error.error)?;
    directory.sync_all().map_err(|error| {
        io::Error::other(format!(
            "PNG exists but directory synchronization failed: {error}"
        ))
    })
}

fn write_png(file: &mut File, surface: &Surface) -> io::Result<()> {
    let mut encoder = png::Encoder::new(file, surface.width(), surface.height());
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png = encoder.write_header()?;
    png.write_image_data(&surface.rgb_bytes())?;
    png.finish()?;
    Ok(())
}

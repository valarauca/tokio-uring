use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use libc::{
    S_IFMT,
    S_IFREG,
    S_IFDIR,
    S_IFLNK,
    S_IFSOCK,
    S_IFIFO,
    S_IFBLK,
    S_IFCHR,
};

use super::File;

/// A structure representing a file type.
#[derive(Copy, Clone)]
pub struct FileType {
    stx_mode: u16,
}

/// Methods for querying the type of a file.
pub trait FileKind {
    /// Returns `true` if this is a regular file.
    fn is_file(&self) -> bool;
    /// Returns `true` if this is a directory.
    fn is_dir(&self) -> bool;
    /// Returns `true` if this is a symbolic link.
    fn is_symlink(&self) -> bool;
    /// Returns `true` if this is a socket.
    fn is_socket(&self) -> bool;
    /// Returns `true` if this is a FIFO.
    fn is_fifo(&self) -> bool;
    /// Returns `true` if this is a block device.
    fn is_block_dev(&self) -> bool;
    /// Returns `true` if this is a character device.
    fn is_char_dev(&self) -> bool;
}

impl FileKind for FileType {
    fn is_file(&self) -> bool {
        self.stx_mode as u32 & S_IFMT == S_IFREG
    }
    fn is_dir(&self) -> bool {
        self.stx_mode as u32 & S_IFMT == S_IFDIR
    }
    fn is_symlink(&self) -> bool {
        self.stx_mode as u32 & S_IFMT == S_IFLNK
    }
    fn is_socket(&self) -> bool {
        self.stx_mode as u32 & S_IFMT == S_IFSOCK
    }
    fn is_fifo(&self) -> bool {
        self.stx_mode as u32 & S_IFMT == S_IFIFO
    }
    fn is_block_dev(&self) -> bool {
        self.stx_mode as u32 & S_IFMT == S_IFBLK
    }
    fn is_char_dev(&self) -> bool {
        self.stx_mode as u32 & S_IFMT == S_IFCHR
    }
}

/// Metadata returned by [`metadata`], [`symlink_metadata`], or [`File::metadata`].
pub struct Metadata {
    inner: libc::statx,
}

impl Metadata {
    pub(crate) fn from_statx(inner: libc::statx) -> Self {
        Self { inner }
    }

    /// Returns the file type for this metadata.
    pub fn file_type(&self) -> FileType {
        FileType {
            stx_mode: self.inner.stx_mode,
        }
    }

    /// Returns `true` if this metadata is for a directory.
    pub fn is_dir(&self) -> bool {
        FileKind::is_dir(self)
    }

    /// Returns `true` if this metadata is for a regular file.
    pub fn is_file(&self) -> bool {
        FileKind::is_file(self)
    }

    /// Returns `true` if this metadata is for a symbolic link.
    pub fn is_symlink(&self) -> bool {
        FileKind::is_symlink(self)
    }

    /// Returns the size of the file in bytes.
    pub fn len(&self) -> u64 {
        self.inner.stx_size
    }

    /// Returns the last modification time of the file.
    pub fn modified(&self) -> std::io::Result<SystemTime> {
        if self.inner.stx_mask & libc::STATX_MTIME == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
        }
        Ok(statx_ts_to_system_time(&self.inner.stx_mtime))
    }

    /// Returns the last access time of the file.
    pub fn accessed(&self) -> std::io::Result<SystemTime> {
        if self.inner.stx_mask & libc::STATX_ATIME == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
        }
        Ok(statx_ts_to_system_time(&self.inner.stx_atime))
    }

    /// Returns the creation time of the file.
    pub fn created(&self) -> std::io::Result<SystemTime> {
        if self.inner.stx_mask & libc::STATX_BTIME == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
        }
        Ok(statx_ts_to_system_time(&self.inner.stx_btime))
    }
}

impl FileKind for Metadata {
    fn is_file(&self) -> bool {
        self.inner.stx_mode as u32 & S_IFMT == S_IFREG
    }
    fn is_dir(&self) -> bool {
        self.inner.stx_mode as u32 & S_IFMT == S_IFDIR
    }
    fn is_symlink(&self) -> bool {
        self.inner.stx_mode as u32 & S_IFMT == S_IFLNK
    }
    fn is_socket(&self) -> bool {
        self.inner.stx_mode as u32 & S_IFMT == S_IFSOCK
    }
    fn is_fifo(&self) -> bool {
        self.inner.stx_mode as u32 & S_IFMT == S_IFIFO
    }
    fn is_block_dev(&self) -> bool {
        self.inner.stx_mode as u32 & S_IFMT == S_IFBLK
    }
    fn is_char_dev(&self) -> bool {
        self.inner.stx_mode as u32 & S_IFMT == S_IFCHR
    }
}

fn statx_ts_to_system_time(ts: &libc::statx_timestamp) -> SystemTime {
    if ts.tv_sec >= 0 {
        UNIX_EPOCH + Duration::new(ts.tv_sec as u64, ts.tv_nsec)
    } else {
        let neg_sec = (-ts.tv_sec) as u64;
        if ts.tv_nsec == 0 {
            UNIX_EPOCH - Duration::from_secs(neg_sec)
        } else {
            UNIX_EPOCH - Duration::from_secs(neg_sec) + Duration::from_nanos(ts.tv_nsec as u64)
        }
    }
}

fn makedev(major: u32, minor: u32) -> u64 {
    let major = major as u64;
    let minor = minor as u64;
    ((major & 0xfffff000) << 32)
        | ((major & 0xfff) << 8)
        | ((minor & 0xffffff00) << 12)
        | (minor & 0xff)
}

#[allow(deprecated)]
impl std::os::linux::fs::MetadataExt for Metadata {
    fn as_raw_stat(&self) -> &std::os::linux::raw::stat {
        unimplemented!()
    }

    fn st_dev(&self) -> u64 {
        makedev(self.inner.stx_dev_major, self.inner.stx_dev_minor)
    }

    fn st_ino(&self) -> u64 {
        self.inner.stx_ino
    }

    fn st_mode(&self) -> u32 {
        self.inner.stx_mode as u32
    }

    fn st_nlink(&self) -> u64 {
        self.inner.stx_nlink as u64
    }

    fn st_uid(&self) -> u32 {
        self.inner.stx_uid
    }

    fn st_gid(&self) -> u32 {
        self.inner.stx_gid
    }

    fn st_rdev(&self) -> u64 {
        makedev(self.inner.stx_rdev_major, self.inner.stx_rdev_minor)
    }

    fn st_size(&self) -> u64 {
        self.inner.stx_size
    }

    fn st_atime(&self) -> i64 {
        self.inner.stx_atime.tv_sec
    }

    fn st_atime_nsec(&self) -> i64 {
        self.inner.stx_atime.tv_nsec as i64
    }

    fn st_mtime(&self) -> i64 {
        self.inner.stx_mtime.tv_sec
    }

    fn st_mtime_nsec(&self) -> i64 {
        self.inner.stx_mtime.tv_nsec as i64
    }

    fn st_ctime(&self) -> i64 {
        self.inner.stx_ctime.tv_sec
    }

    fn st_ctime_nsec(&self) -> i64 {
        self.inner.stx_ctime.tv_nsec as i64
    }

    fn st_blksize(&self) -> u64 {
        self.inner.stx_blksize as u64
    }

    fn st_blocks(&self) -> u64 {
        self.inner.stx_blocks
    }
}

/// Returns metadata for the given path without following symlinks.
pub async fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let mut builder = super::statx::StatxBuilder::new();
    builder
        .flags(libc::AT_SYMLINK_NOFOLLOW)
        .pathname(path)?
        .statx()
        .await
        .map(Metadata::from_statx)
}

/// Returns metadata for the given path, following symlinks.
pub async fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let mut builder = super::statx::StatxBuilder::new();
    builder
        .flags(libc::AT_STATX_SYNC_AS_STAT)
        .pathname(path)?
        .statx()
        .await
        .map(Metadata::from_statx)
}

impl File {
    /// Returns metadata for this open file.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        self.statx().await.map(Metadata::from_statx)
    }
}

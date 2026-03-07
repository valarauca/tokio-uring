use std::time::{Duration, SystemTime, UNIX_EPOCH};

const S_IFMT: u16 = 0o170000;
const S_IFREG: u16 = 0o100000;
const S_IFDIR: u16 = 0o040000;
const S_IFLNK: u16 = 0o120000;
const S_IFSOCK: u16 = 0o140000;
const S_IFIFO: u16 = 0o010000;
const S_IFBLK: u16 = 0o060000;
const S_IFCHR: u16 = 0o020000;

#[derive(Copy, Clone)]
pub struct FileType {
    stx_mode: u16,
}

pub trait FileKind {
    fn is_file(&self) -> bool;
    fn is_dir(&self) -> bool;
    fn is_symlink(&self) -> bool;
    fn is_socket(&self) -> bool;
    fn is_fifo(&self) -> bool;
    fn is_block_dev(&self) -> bool;
    fn is_char_dev(&self) -> bool;
}

impl FileKind for FileType {
    fn is_file(&self) -> bool {
        self.stx_mode & S_IFMT == S_IFREG
    }
    fn is_dir(&self) -> bool {
        self.stx_mode & S_IFMT == S_IFDIR
    }
    fn is_symlink(&self) -> bool {
        self.stx_mode & S_IFMT == S_IFLNK
    }
    fn is_socket(&self) -> bool {
        self.stx_mode & S_IFMT == S_IFSOCK
    }
    fn is_fifo(&self) -> bool {
        self.stx_mode & S_IFMT == S_IFIFO
    }
    fn is_block_dev(&self) -> bool {
        self.stx_mode & S_IFMT == S_IFBLK
    }
    fn is_char_dev(&self) -> bool {
        self.stx_mode & S_IFMT == S_IFCHR
    }
}

pub struct Metadata {
    inner: libc::statx,
}

impl Metadata {
    pub(crate) fn from_statx(inner: libc::statx) -> Self {
        Self { inner }
    }

    pub fn file_type(&self) -> FileType {
        FileType {
            stx_mode: self.inner.stx_mode,
        }
    }

    pub fn is_dir(&self) -> bool {
        FileKind::is_dir(self)
    }

    pub fn is_file(&self) -> bool {
        FileKind::is_file(self)
    }

    pub fn is_symlink(&self) -> bool {
        FileKind::is_symlink(self)
    }

    pub fn len(&self) -> u64 {
        self.inner.stx_size
    }

    pub fn modified(&self) -> std::io::Result<SystemTime> {
        if self.inner.stx_mask & libc::STATX_MTIME == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
        }
        Ok(statx_ts_to_system_time(&self.inner.stx_mtime))
    }

    pub fn accessed(&self) -> std::io::Result<SystemTime> {
        if self.inner.stx_mask & libc::STATX_ATIME == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
        }
        Ok(statx_ts_to_system_time(&self.inner.stx_atime))
    }

    pub fn created(&self) -> std::io::Result<SystemTime> {
        if self.inner.stx_mask & libc::STATX_BTIME == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::Unsupported));
        }
        Ok(statx_ts_to_system_time(&self.inner.stx_btime))
    }
}

impl FileKind for Metadata {
    fn is_file(&self) -> bool {
        self.inner.stx_mode & S_IFMT == S_IFREG
    }
    fn is_dir(&self) -> bool {
        self.inner.stx_mode & S_IFMT == S_IFDIR
    }
    fn is_symlink(&self) -> bool {
        self.inner.stx_mode & S_IFMT == S_IFLNK
    }
    fn is_socket(&self) -> bool {
        self.inner.stx_mode & S_IFMT == S_IFSOCK
    }
    fn is_fifo(&self) -> bool {
        self.inner.stx_mode & S_IFMT == S_IFIFO
    }
    fn is_block_dev(&self) -> bool {
        self.inner.stx_mode & S_IFMT == S_IFBLK
    }
    fn is_char_dev(&self) -> bool {
        self.inner.stx_mode & S_IFMT == S_IFCHR
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

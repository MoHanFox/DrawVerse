//! Volatile scratch storage. This is not an autosave or a hostile-user sandbox.
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};
const MAGIC: &[u8] = b"DrawVerse scratch namespace v1\n";
const LEASE: &[u8] = b"DrawVerse scratch run v1\n";
const CHUNK: u64 = 1024 * 1024;
#[derive(Clone, Debug)]
pub struct Config {
    pub directory: PathBuf,
    pub min_free_bytes: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            directory: std::env::temp_dir(),
            min_free_bytes: 512 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub directory: PathBuf,
    pub available_bytes: u64,
    pub removed_runs: u32,
    pub removed_bytes: u64,
    pub skipped_runs: u32,
}
#[derive(Debug)]
struct Run {
    path: PathBuf,
    lease: Option<File>,
    report: Report,
}
impl Drop for Run {
    fn drop(&mut self) {
        // Named temp files hold the manager and are released first. Never
        // recursively delete unexpected material placed in a run directory.
        drop(self.lease.take());
        let _ = fs::remove_file(self.path.join(".lease"));
        let _ = fs::remove_dir(&self.path);
    }
}
#[derive(Debug)]
pub struct ScratchSpace {
    config: Config,
    run: Mutex<Option<Run>>,
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn regular(path: &Path) -> io::Result<bool> {
    Ok(fs::symlink_metadata(path)?.file_type().is_file())
}
fn marker(file: &mut File, expected: &[u8]) -> io::Result<bool> {
    if file.metadata()?.len() != expected.len() as u64 {
        return Ok(false);
    }
    let mut bytes = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.take(128).read_to_end(&mut bytes)?;
    Ok(bytes == expected)
}
fn random_name(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .is_some_and(|tail| tail.len() == 16 && tail.bytes().all(|b| b.is_ascii_alphanumeric()))
}
fn owned_file(name: &str) -> bool {
    random_name(name, "pages-") || random_name(name, "history-")
}
fn sweep(root: &Path) -> io::Result<Report> {
    let mut report = Report::default();
    for entry in fs::read_dir(root)?.take(256) {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !random_name(name, "run-") {
            continue;
        }
        let path = entry.path();
        let result = (|| -> io::Result<Option<u64>> {
            if !fs::symlink_metadata(&path)?.file_type().is_dir() || path.canonicalize()? != path {
                return Ok(None);
            }
            let lock_path = path.join(".lease");
            if !regular(&lock_path)? {
                return Ok(None);
            }
            let mut lock = OpenOptions::new().read(true).write(true).open(&lock_path)?;
            if FileExt::try_lock_exclusive(&lock).is_err() || !marker(&mut lock, LEASE)? {
                return Ok(None);
            }
            let mut files = Vec::new();
            let mut total = 0u64;
            for (index, child) in fs::read_dir(&path)?.take(2050).enumerate() {
                if index >= 2049 {
                    return Ok(None);
                }
                let child = child?;
                let child_name = child.file_name();
                let Some(child_name) = child_name.to_str() else {
                    return Ok(None);
                };
                if child_name == ".lease" {
                    continue;
                }
                if !owned_file(child_name) || !regular(&child.path())? || files.len() >= 2048 {
                    return Ok(None);
                }
                total = total.saturating_add(child.metadata()?.len());
                files.push(child.path());
            }
            for file in files {
                // Recheck bounds/type immediately before deleting known files.
                if file.parent() != Some(path.as_path())
                    || !regular(&file)?
                    || path.canonicalize()? != path
                {
                    return Ok(None);
                }
                fs::remove_file(file)?;
            }
            drop(lock);
            fs::remove_file(lock_path)?;
            fs::remove_dir(path)?;
            Ok(Some(total))
        })();
        match result {
            Ok(Some(bytes)) => {
                report.removed_runs += 1;
                report.removed_bytes += bytes;
            }
            _ => report.skipped_runs += 1,
        }
    }
    Ok(report)
}
impl ScratchSpace {
    /// Lazy: blank documents need no file or disk query.
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config,
            run: Mutex::new(None),
        })
    }
    pub fn system() -> Arc<Self> {
        static DEFAULT: OnceLock<Mutex<Weak<ScratchSpace>>> = OnceLock::new();
        let mut cache = DEFAULT
            .get_or_init(|| Mutex::new(Weak::new()))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(space) = cache.upgrade() {
            return space;
        }
        let space = Self::new(Config::default());
        *cache = Arc::downgrade(&space);
        space
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    fn initialize(&self) -> io::Result<Run> {
        let directory = self.config.directory.canonicalize()?;
        if !directory.is_dir() {
            return Err(invalid("scratch path must be an existing directory"));
        }
        self.check_space(&directory, 0)?;
        let root = directory.join(".drawverse-scratch-v1");
        match fs::create_dir(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        if !fs::symlink_metadata(&root)?.file_type().is_dir() || root.canonicalize()? != root {
            return Err(invalid(
                "scratch namespace is a link or outside its directory",
            ));
        }
        let lock_path = root.join(".maintenance");
        if let Err(e) = fs::symlink_metadata(&lock_path) {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e);
            }
            if fs::read_dir(&root)?.next().transpose()?.is_some() {
                return Err(invalid("unmarked nonempty scratch namespace"));
            }
        }
        let (mut lock, created) = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(file) => (file, true),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                if !regular(&lock_path)? {
                    return Err(invalid("scratch maintenance marker is not a regular file"));
                }
                (
                    OpenOptions::new().read(true).write(true).open(&lock_path)?,
                    false,
                )
            }
            Err(e) => return Err(e),
        };
        FileExt::try_lock_exclusive(&lock).map_err(|_| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "scratch directory maintenance is busy; retry later",
            )
        })?;
        if created {
            lock.write_all(MAGIC)?;
            lock.flush()?;
        }
        if !marker(&mut lock, MAGIC)? {
            return Err(invalid("scratch namespace marker mismatch"));
        }
        let mut report = sweep(&root)?;
        let path = tempfile::Builder::new()
            .prefix("run-")
            .rand_bytes(16)
            .tempdir_in(&root)?
            .keep();
        let lease_path = path.join(".lease");
        let lease_result = (|| -> io::Result<File> {
            let mut lease = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&lease_path)?;
            FileExt::try_lock_exclusive(&lease)?;
            lease.write_all(LEASE)?;
            lease.flush()?;
            Ok(lease)
        })();
        let lease = match lease_result {
            Ok(file) => file,
            Err(error) => {
                let _ = fs::remove_file(lease_path);
                let _ = fs::remove_dir(path);
                return Err(error);
            }
        };
        report.directory = directory;
        report.available_bytes = fs2::available_space(&report.directory)?;
        Ok(Run {
            path,
            lease: Some(lease),
            report,
        })
    }
    fn check_space(&self, path: &Path, growth: u64) -> io::Result<u64> {
        let available = fs2::available_space(path)?;
        if available < self.config.min_free_bytes.saturating_add(growth) {
            return Err(io::Error::other(format!("scratch free space {available} bytes is below the reserved {} bytes plus growth {growth}; choose another disk or lower the reserve",self.config.min_free_bytes)));
        }
        Ok(available)
    }
    pub fn inspect(&self) -> io::Result<Report> {
        let mut run = self.run.lock().unwrap_or_else(|e| e.into_inner());
        if run.is_none() {
            *run = Some(self.initialize()?);
        }
        let mut report = run.as_ref().expect("initialized run").report.clone();
        report.available_bytes = self.check_space(&report.directory, 0)?;
        Ok(report)
    }
    pub fn tempfile(self: &Arc<Self>, kind: &str) -> io::Result<ManagedFile> {
        if kind != "pages" && kind != "history" {
            return Err(invalid("unknown scratch file kind"));
        }
        let mut run = self.run.lock().unwrap_or_else(|e| e.into_inner());
        if run.is_none() {
            *run = Some(self.initialize()?);
        }
        let file = tempfile::Builder::new()
            .prefix(&format!("{kind}-"))
            .rand_bytes(16)
            .tempfile_in(&run.as_ref().expect("initialized run").path)?;
        Ok(ManagedFile {
            file,
            space: Arc::clone(self),
            position: 0,
            length: 0,
            checked_end: 0,
            last_check: Instant::now(),
        })
    }
}
#[derive(Debug)]
pub struct ManagedFile {
    file: tempfile::NamedTempFile,
    space: Arc<ScratchSpace>,
    position: u64,
    length: u64,
    checked_end: u64,
    last_check: Instant,
}
impl ManagedFile {
    pub fn path(&self) -> &Path {
        self.file.path()
    }
    pub fn reopen(&self) -> io::Result<File> {
        self.file.reopen()
    }
    pub fn as_file_mut(&mut self) -> &mut File {
        self.file.as_file_mut()
    }
}
impl Read for ManagedFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read(bytes)?;
        self.position += n as u64;
        Ok(n)
    }
}
impl Seek for ManagedFile {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.position = self.file.seek(from)?;
        Ok(self.position)
    }
}
impl Write for ManagedFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self
            .position
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| invalid("scratch offset overflow"))?;
        if end > self.length
            && (end > self.checked_end || self.last_check.elapsed() >= Duration::from_secs(1))
        {
            let available = self
                .space
                .check_space(self.file.path(), end - self.length)?;
            let credit =
                (available - self.space.config.min_free_bytes).min((end - self.length).max(CHUNK));
            self.checked_end = self.length.saturating_add(credit);
            self.last_check = Instant::now();
        }
        let n = self.file.write(bytes)?;
        self.position += n as u64;
        self.length = self.length.max(self.position);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

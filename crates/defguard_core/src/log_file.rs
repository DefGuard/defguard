use std::{
    fs::{self, OpenOptions},
    io::{self, SeekFrom, Write},
    path::{Path, PathBuf, absolute},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use defguard_common::config::LogFileRotation;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

const LOG_TAIL_MAX_BYTES: u64 = 10 * 1024 * 1024;
const BACKUP_EXTENSION: &str = "bak";

fn log_file_location(path: &Path) -> Option<(&Path, &str)> {
    let file_name = path.file_name()?.to_str()?;
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    Some((directory, file_name))
}

/// With rotation, logs go to dated files next to path, and path is a symlink to the current one.
pub fn open_log_writer(
    path: &str,
    rotation: LogFileRotation,
    max_files: Option<usize>,
) -> anyhow::Result<Box<dyn Write + Send>> {
    let Some((directory, file_name)) = log_file_location(Path::new(path)) else {
        bail!("Invalid log file path {path}");
    };
    // The appender links to directory.join(file), which resolves relative to the symlink itself,
    // so a relative directory would leave the symlink dangling.
    let directory = absolute(directory)
        .with_context(|| format!("Failed to resolve log file directory of {path}"))?;

    // tracing-appender has no file mode option, so umask keeps rotated logs private.
    #[cfg(unix)]
    nix::sys::stat::umask(nix::sys::stat::Mode::from_bits_truncate(0o077));

    fs::create_dir_all(&directory).with_context(|| {
        format!(
            "Failed to create log file directory {}",
            directory.display()
        )
    })?;
    let is_symlink = fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink());

    let rotation = match rotation {
        LogFileRotation::Never => None,
        LogFileRotation::Hourly => Some(Rotation::HOURLY),
        LogFileRotation::Daily => Some(Rotation::DAILY),
        LogFileRotation::Weekly => Some(Rotation::WEEKLY),
    };
    let Some(rotation) = rotation else {
        if is_symlink {
            fs::remove_file(path)
                .with_context(|| format!("Failed to remove log file symlink {path}"))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("Failed to open log file {path}"))?;
        return Ok(Box::new(file));
    };

    // The appender deletes whatever is at the symlink path, so keep a log left by a run without
    // rotation.
    if !is_symlink && Path::new(path).exists() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let backup = format!("{path}.{timestamp}.{BACKUP_EXTENSION}");
        fs::rename(path, &backup)
            .with_context(|| format!("Failed to move log file {path} to {backup}"))?;
    }

    let mut builder = RollingFileAppender::builder()
        .rotation(rotation)
        .filename_prefix(file_name)
        .latest_symlink(file_name);
    if let Some(max_files) = max_files {
        builder = builder.max_log_files(max_files);
    }
    let appender = builder
        .build(directory)
        .with_context(|| format!("Failed to open log file {path}"))?;
    Ok(Box::new(appender))
}

/// The first partial line is dropped when the logs are longer than the limit.
pub(crate) async fn read_log_tail(path: &str) -> io::Result<String> {
    let files = log_files_newest_first(Path::new(path)).await?;
    read_files_tail(&files, LOG_TAIL_MAX_BYTES).await
}

/// A rotation symlink stands for every dated file next to it, a plain file only for itself.
async fn log_files_newest_first(path: &Path) -> io::Result<Vec<PathBuf>> {
    if !tokio::fs::symlink_metadata(path)
        .await?
        .file_type()
        .is_symlink()
    {
        return Ok(vec![path.to_path_buf()]);
    }
    let Some((directory, file_name)) = log_file_location(path) else {
        return Ok(vec![path.to_path_buf()]);
    };
    let prefix = format!("{file_name}.");
    let backup_suffix = format!(".{BACKUP_EXTENSION}");

    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(directory).await?;
    while let Some(entry) = entries.next_entry().await? {
        let is_rotated_log = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(&prefix) && !name.ends_with(&backup_suffix));
        if is_rotated_log && entry.file_type().await?.is_file() {
            files.push(entry.path());
        }
    }
    if files.is_empty() {
        files.push(path.to_path_buf());
    }
    // Rotated file names end with the date, so the newest sorts last.
    files.sort_unstable_by(|left, right| right.cmp(left));
    Ok(files)
}

async fn read_files_tail(newest_first: &[PathBuf], max_bytes: u64) -> io::Result<String> {
    let mut chunks = Vec::new();
    let mut remaining = max_bytes;
    let mut truncated = false;
    for path in newest_first {
        if remaining == 0 {
            break;
        }
        let mut file = File::open(path).await?;
        let start = file.metadata().await?.len().saturating_sub(remaining);
        file.seek(SeekFrom::Start(start)).await?;
        let mut bytes = Vec::new();
        file.take(remaining).read_to_end(&mut bytes).await?;
        remaining = remaining.saturating_sub(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        chunks.push(bytes);
        if start > 0 {
            truncated = true;
            break;
        }
    }

    let mut bytes: Vec<u8> = chunks.into_iter().rev().flatten().collect();
    if truncated && let Some(first_newline) = bytes.iter().position(|&byte| byte == b'\n') {
        bytes.drain(..=first_newline);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use std::{env, fs, io::Write, path::PathBuf};

    use defguard_common::config::LogFileRotation;

    use super::{LOG_TAIL_MAX_BYTES, open_log_writer, read_files_tail, read_log_tail};

    fn temp_log_dir(files: &[(&str, &str)]) -> PathBuf {
        let dir = env::temp_dir().join(format!("defguard-log-dir-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).expect("create log dir");
        for (name, contents) in files {
            fs::write(dir.join(name), contents).expect("write log file");
        }
        dir
    }

    struct RestoreCwd(PathBuf);

    impl Drop for RestoreCwd {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }

    // Changes the process wide cwd, so plain cargo test must not run it beside tests that use
    // relative paths.
    #[test]
    fn test_rotated_log_symlink_resolves_for_relative_path() {
        let root = temp_log_dir(&[]);
        let cwd = RestoreCwd(env::current_dir().expect("read cwd"));
        env::set_current_dir(&root).expect("enter temp dir");

        let mut writer = open_log_writer("logs/defguard.log", LogFileRotation::Daily, None)
            .expect("open log file");
        writer.write_all(b"first line\n").expect("write log line");
        writer.flush().expect("flush log file");
        drop(writer);
        let logged = fs::read_to_string("logs/defguard.log");
        drop(cwd);
        fs::remove_dir_all(&root).expect("remove temp dir");

        assert_eq!(logged.expect("read through symlink"), "first line\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_log_files_are_private() {
        use std::os::unix::fs::PermissionsExt;

        for rotation in [LogFileRotation::Never, LogFileRotation::Daily] {
            let dir = temp_log_dir(&[]);
            let path = dir.join("defguard.log");
            let mut writer = open_log_writer(path.to_str().expect("utf-8 path"), rotation, None)
                .expect("open log file");
            writer.write_all(b"line\n").expect("write log line");
            drop(writer);
            let mode = fs::metadata(&path)
                .expect("log file metadata")
                .permissions()
                .mode();
            fs::remove_dir_all(&dir).expect("remove temp dir");

            assert_eq!(mode & 0o777, 0o600, "{rotation:?} log file is not private");
        }
    }

    async fn read_tail_of(contents: &str) -> String {
        let path = env::temp_dir().join(format!("defguard-log-tail-{}.log", uuid::Uuid::new_v4()));
        fs::write(&path, contents).expect("write log file");
        let tail = read_log_tail(path.to_str().expect("utf-8 path")).await;
        fs::remove_file(&path).expect("remove log file");
        tail.expect("read log tail")
    }

    #[tokio::test]
    async fn test_read_log_tail_returns_whole_lines_within_limit() {
        let line = format!("{}\n", "x".repeat(1023));
        let line_count = usize::try_from(LOG_TAIL_MAX_BYTES / 1024).unwrap() + 3;
        let contents = line.repeat(line_count) + "last line\n";

        let tail = read_tail_of(&contents).await;
        assert!(u64::try_from(tail.len()).unwrap() <= LOG_TAIL_MAX_BYTES);
        assert!(
            tail.lines()
                .take_while(|line| *line != "last line")
                .all(|line| line.len() == 1023)
        );
        assert!(tail.ends_with("last line\n"));
    }

    #[tokio::test]
    async fn test_read_log_tail_returns_small_file_unchanged() {
        assert_eq!(read_tail_of("first\nsecond\n").await, "first\nsecond\n");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_read_log_tail_joins_rotated_files() {
        let dir = temp_log_dir(&[
            ("defguard.log.2026-10-15", "older\n"),
            ("defguard.log.2026-10-16", "newer\n"),
            ("defguard.log.1760000000.bak", "backup\n"),
        ]);
        std::os::unix::fs::symlink(
            dir.join("defguard.log.2026-10-16"),
            dir.join("defguard.log"),
        )
        .expect("create log symlink");

        let tail = read_log_tail(dir.join("defguard.log").to_str().expect("utf-8 path")).await;
        fs::remove_dir_all(&dir).expect("remove log dir");
        assert_eq!(tail.expect("read log tail"), "older\nnewer\n");
    }

    #[tokio::test]
    async fn test_read_files_tail_drops_partial_line_of_oldest_file() {
        let dir = temp_log_dir(&[("older", "aaa\nbbb\n"), ("newer", "ccc\n")]);

        let tail = read_files_tail(&[dir.join("newer"), dir.join("older")], 10).await;
        fs::remove_dir_all(&dir).expect("remove log dir");
        assert_eq!(tail.expect("read log tail"), "bbb\nccc\n");
    }
}

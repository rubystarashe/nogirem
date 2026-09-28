use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 16 * 1024;
static WRITE_LOCK: Mutex<()> = Mutex::new(());

struct HelperLog {
    path: PathBuf,
    previous: PathBuf,
    limit: u64,
}

impl HelperLog {
    fn new(path: &Path, limit: u64) -> Self {
        Self {
            path: path.into(),
            previous: path.with_extension("previous.log"),
            limit,
        }
    }

    // Also bound files left by older builds, even if the helper emits no new stderr.
    fn trim(&self, path: &Path) -> io::Result<()> {
        let mut file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let size = file.metadata()?.len();
        if size <= self.limit {
            return Ok(());
        }
        file.seek(SeekFrom::Start(size - self.limit))?;
        let mut tail = vec![0; self.limit as usize];
        file.read_exact(&mut tail)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&tail)?;
        file.set_len(self.limit)
    }

    fn prepare(&self) -> io::Result<()> {
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        self.trim(&self.path)?;
        self.trim(&self.previous)
    }

    fn append(&self, line: &[u8], truncated: bool) -> io::Result<()> {
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut record = format!(
            "{} {}{}\n",
            crate::app_services::iso(),
            String::from_utf8_lossy(line),
            if truncated { " [truncated]" } else { "" }
        )
        .into_bytes();
        if record.len() as u64 > self.limit {
            record.truncate(self.limit as usize);
            if let Some(last) = record.last_mut() {
                *last = b'\n';
            }
        }
        self.trim(&self.path)?;
        self.trim(&self.previous)?;
        let size = match fs::metadata(&self.path) {
            Ok(meta) => meta.len(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
            Err(e) => return Err(e),
        };
        if size + record.len() as u64 > self.limit {
            match fs::remove_file(&self.previous) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            fs::rename(&self.path, &self.previous)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?
            .write_all(&record)
    }
}

// Unlike BufRead::lines(), this never accumulates an unbounded unterminated line.
// Keep draining after logging errors so a full disk cannot block the child pipe.
fn drain(mut reader: impl BufRead, log: &HelperLog) -> io::Result<()> {
    let mut line = Vec::with_capacity(MAX_LINE_BYTES);
    let mut truncated = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if !line.is_empty() || truncated {
                let _ = log.append(&line, truncated);
            }
            return Ok(());
        }
        let end = available.iter().position(|b| *b == b'\n');
        let count = end.unwrap_or(available.len());
        let kept = count.min(MAX_LINE_BYTES - line.len());
        line.extend_from_slice(&available[..kept]);
        truncated |= kept < count;
        reader.consume(count + usize::from(end.is_some()));
        if end.is_some() {
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let _ = log.append(&line, truncated);
            line.clear();
            truncated = false;
        }
    }
}

pub fn capture(reader: impl BufRead, path: &Path) {
    let log = HelperLog::new(path, MAX_FILE_BYTES);
    let _ = log.prepare();
    let _ = drain(reader, &log);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{BufReader, Cursor},
    };
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p =
                std::env::temp_dir().join(format!("nogirem-helper-log-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
        fn log(&self, limit: u64) -> HelperLog {
            HelperLog::new(&self.0.join("helper-stderr.log"), limit)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn repeated_rotation_keeps_only_two_bounded_files_and_latest_record() {
        let f = Fixture::new();
        let log = f.log(256);
        for i in 0..1000 {
            log.append(
                format!("event-{i:04} abcdefghijklmnopqrstuvwxyz").as_bytes(),
                false,
            )
            .unwrap();
            assert!(fs::metadata(&log.path).unwrap().len() <= 256);
            if log.previous.exists() {
                assert!(fs::metadata(&log.previous).unwrap().len() <= 256);
            }
        }
        assert_eq!(fs::read_dir(&f.0).unwrap().count(), 2);
        assert!(
            fs::read_to_string(&log.path)
                .unwrap()
                .contains("event-0999")
        );
        assert!(
            !fs::read_to_string(&log.previous)
                .unwrap()
                .contains("event-0000")
        );
    }
    #[test]
    fn oversized_legacy_files_are_trimmed_even_without_new_output() {
        let f = Fixture::new();
        let log = f.log(MAX_FILE_BYTES);
        for p in [&log.path, &log.previous] {
            let mut file = File::create(p).unwrap();
            file.set_len(MAX_FILE_BYTES * 3).unwrap();
            file.seek(SeekFrom::End(-4)).unwrap();
            file.write_all(b"TAIL").unwrap();
        }
        capture(Cursor::new(b""), &log.path);
        for p in [&log.path, &log.previous] {
            let bytes = fs::read(p).unwrap();
            assert_eq!(bytes.len(), MAX_FILE_BYTES as usize);
            assert!(bytes.ends_with(b"TAIL"));
        }
    }
    #[test]
    fn long_unterminated_and_invalid_utf8_lines_do_not_stop_capture() {
        let f = Fixture::new();
        let log = f.log(MAX_FILE_BYTES);
        let mut input = vec![b'x'; 2 * 1024 * 1024];
        input.extend_from_slice(b"\n\xff\xfe\nnext\r\nlast");
        drain(BufReader::with_capacity(73, Cursor::new(input)), &log).unwrap();
        let text = fs::read_to_string(&log.path).unwrap();
        assert!(text.len() < MAX_LINE_BYTES + 512);
        assert!(text.contains("[truncated]"));
        assert!(text.contains("next\n"));
        assert!(text.ends_with("last\n"));
    }
    #[test]
    fn rotation_failure_never_appends_past_limit_and_pipe_is_drained() {
        let f = Fixture::new();
        let log = f.log(64);
        fs::write(&log.path, vec![b'a'; 64]).unwrap();
        fs::create_dir(&log.previous).unwrap();
        assert!(log.append(b"failure", false).is_err());
        let mut input = Cursor::new(b"one\ntwo\nthree\n");
        drain(&mut input, &log).unwrap();
        assert_eq!(input.position(), input.get_ref().len() as u64);
        assert_eq!(fs::metadata(&log.path).unwrap().len(), 64);
    }
}

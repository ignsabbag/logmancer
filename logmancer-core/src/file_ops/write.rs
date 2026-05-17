use crate::models::log_file::LogFile;
use crate::models::search::{SearchMatch, SearchPhase, SearchSession};
use memmap2::Mmap;
use regex::Regex;
use std::cmp::min;
use std::fs::{File, metadata};
use std::io;
use std::path::Path;
use std::sync::{Arc, RwLock};

const INDEX_MAX_BYTES: usize = 1024 * 1024; // 1MB
const INDEX_MAX_LINES: usize = 1000;
pub const SEARCH_BATCH_MAX_LINES: usize = 1000;

pub struct FileWriteOps {
    log_file: Arc<RwLock<LogFile>>,
}

impl FileWriteOps {
    pub fn new(log_file: Arc<RwLock<LogFile>>) -> Self {
        FileWriteOps { log_file }
    }

    /// Checks the file size and resets mmap and size
    pub fn reload(&mut self) -> io::Result<()> {
        let mut file_lock = self.log_file.write().unwrap();
        let current_size = metadata(Path::new(&file_lock.path))?.len();
        if current_size > file_lock.size {
            let file = File::open(&file_lock.path)?;
            file_lock.mmap = unsafe { Mmap::map(&file)? };
            file_lock.size = current_size;
            Ok(())
        } else if current_size < file_lock.size {
            Err(io::Error::new(io::ErrorKind::InvalidData, "File changed"))
        } else {
            Ok(())
        }
    }

    /// Indexes lines up to a maximum of INDEX_MAX_BYTES bytes. Returns false unless the end of the file is reached.
    pub fn index_lines(&mut self) -> io::Result<bool> {
        let file_lock = self.log_file.read().unwrap();
        let start_pos = *file_lock.index.last().unwrap();
        let end_pos = min(file_lock.mmap.len(), start_pos + INDEX_MAX_BYTES);
        let end_reached = file_lock.mmap.len() <= start_pos + INDEX_MAX_BYTES;
        let content = file_lock.mmap[start_pos..end_pos].to_vec();
        drop(file_lock);

        let mut index = Vec::new();
        for (pos, byte) in content.iter().enumerate() {
            if *byte == b'\n' {
                index.push(start_pos + pos + 1);
            }
        }

        let mut file_lock = self.log_file.write().unwrap();
        if *file_lock.index.last().unwrap() == start_pos {
            file_lock.index.extend(index);
            Ok(end_reached)
        } else {
            Ok(false)
        }
    }

    /// Sets the regex pattern and resets the filter index if a pattern is provided.
    pub fn filter(&mut self, pattern: Option<String>) -> io::Result<()> {
        if let Some(pat) = pattern {
            let mut file_lock = self.log_file.write().unwrap();
            file_lock.regex = Some(pat);
            file_lock.filter.clear();
        }
        Ok(())
    }

    /// Indexes filtered lines up to a maximun of INDEX_MAX_LINES lines. Returns false unless the end of the file is reached.
    pub fn index_filter(&mut self) -> io::Result<bool> {
        let file_lock = self.log_file.read().unwrap();
        let pattern = match &file_lock.regex {
            Some(pat) => pat.clone(),
            None => return Ok(true),
        };
        let re =
            Regex::new(&pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let start_line = file_lock.filter.len();
        let total_lines = file_lock.index.len();
        let end_line = min(total_lines.saturating_sub(1), start_line + INDEX_MAX_LINES);
        let mut batch = Vec::with_capacity(end_line.saturating_sub(start_line));
        for i in start_line..end_line {
            let start_pos = file_lock.index[i];
            let end_pos = file_lock.index[i + 1];
            let line = &file_lock.mmap[start_pos..end_pos];
            let match_filter = std::str::from_utf8(line).is_ok_and(|text| re.is_match(text));
            batch.push(match_filter);
        }
        drop(file_lock);

        let mut file_lock = self.log_file.write().unwrap();
        if file_lock.filter.len() != start_line {
            return Ok(false);
        }
        file_lock.filter.extend(batch);
        Ok(end_line == total_lines.saturating_sub(1))
    }

    pub fn begin_search(&mut self, generation: u64, query: String, origin_line: usize) {
        let mut file_lock = self.log_file.write().unwrap();
        file_lock.search.session = Some(SearchSession::indexing(generation, query, origin_line));
    }

    pub fn log_file(&self) -> Arc<RwLock<LogFile>> {
        Arc::clone(&self.log_file)
    }

    pub fn merge_search_batch(
        &mut self,
        generation: u64,
        mut batch: Vec<SearchMatch>,
        mark_ready: bool,
    ) -> bool {
        let mut file_lock = self.log_file.write().unwrap();
        let Some(session) = file_lock.search.session.as_mut() else {
            return false;
        };
        if session.generation != generation {
            return false;
        }

        session.matches.append(&mut batch);
        session
            .matches
            .sort_by_key(|m| (m.line_index, m.start, m.end));
        for (idx, item) in session.matches.iter_mut().enumerate() {
            item.ordinal = idx;
        }

        session.first_match = session.matches.first().cloned();
        if session.current_ordinal.is_none() && !session.matches.is_empty() {
            session.current_ordinal = Some(0);
        }
        if mark_ready {
            session.phase = SearchPhase::Ready;
            session.total_matches_final = true;
        } else {
            session.phase = SearchPhase::Indexing;
            session.total_matches_final = false;
        }
        true
    }

    pub fn clear_search(&mut self) {
        let mut file_lock = self.log_file.write().unwrap();
        file_lock.search.clear();
    }

    pub fn search_next(&mut self) {
        let mut file_lock = self.log_file.write().unwrap();
        if let Some(session) = file_lock.search.session.as_mut() {
            session.next();
        }
    }

    pub fn search_previous(&mut self) {
        let mut file_lock = self.log_file.write().unwrap();
        if let Some(session) = file_lock.search.session.as_mut() {
            session.previous();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workers::{SearchCommand, spawn_search_worker};
    use crossbeam_channel::unbounded;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("logmancer-write-{name}-{suffix}.log"))
    }

    fn wait_until<F: Fn() -> bool>(predicate: F) {
        for _ in 0..100 {
            if predicate() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn search_worker_scans_circular_from_origin_and_marks_ready() {
        let path = temp_file_path("search-circular");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "foo-first").unwrap();
        writeln!(file, "none").unwrap();
        writeln!(file, "foo-origin").unwrap();
        writeln!(file, "foo-after").unwrap();
        drop(file);

        let log_file = Arc::new(RwLock::new(
            LogFile::new(path.to_string_lossy().into_owned()).unwrap(),
        ));
        let mut write_ops = FileWriteOps::new(Arc::clone(&log_file));
        while !write_ops.index_lines().unwrap() {}

        let worker_ops = FileWriteOps::new(Arc::clone(&log_file));
        let (tx, rx) = unbounded::<SearchCommand>();
        spawn_search_worker(worker_ops, rx);

        let generation = 1u64;
        write_ops.begin_search(generation, "foo".to_string(), 2);
        tx.send(SearchCommand::Start {
            generation,
            query: "foo".to_string(),
            origin_line: 2,
            indexed_lines: log_file.read().unwrap().index.len(),
        })
        .unwrap();

        wait_until(|| log_file.read().unwrap().search.status().is_ready);
        let status = log_file.read().unwrap().search.status();
        assert!(status.is_ready);
        assert!(status.total_matches_final);
        assert_eq!(status.total_matches, 3);
        assert_eq!(status.first.unwrap().line_index, 0);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stale_generation_batches_are_rejected() {
        let path = temp_file_path("stale-generation");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "foo").unwrap();
        drop(file);

        let log_file = Arc::new(RwLock::new(
            LogFile::new(path.to_string_lossy().into_owned()).unwrap(),
        ));
        let mut write_ops = FileWriteOps::new(Arc::clone(&log_file));
        while !write_ops.index_lines().unwrap() {}

        write_ops.begin_search(2, "foo".to_string(), 0);
        let merged = write_ops.merge_search_batch(
            1,
            vec![SearchMatch {
                line_index: 0,
                start: 0,
                end: 3,
                ordinal: 0,
            }],
            true,
        );

        assert!(!merged);
        let status = log_file.read().unwrap().search.status();
        assert_eq!(status.generation, 2);
        assert_eq!(status.total_matches, 0);

        std::fs::remove_file(path).unwrap();
    }
}

use crate::models::log_file::LogFile;
use crate::models::search::{PageSearchResult, SearchMatch, SearchStatus};
use regex::Regex;
use std::collections::HashSet;
use std::io;
use std::sync::RwLockReadGuard;

pub struct FileReadOps<'a> {
    log_file: RwLockReadGuard<'a, LogFile>,
}

const LINE_MAX_BYTES: usize = 10 * 1024;

impl<'a> FileReadOps<'a> {
    pub fn new(log_file: RwLockReadGuard<'a, LogFile>) -> Self {
        FileReadOps { log_file }
    }

    pub fn file_path(&self) -> String {
        self.log_file.path.clone()
    }

    /// Reads the line number `line_number` from the file.
    pub fn read_line(&self, line_number: usize) -> io::Result<String> {
        if line_number >= self.log_file.index.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Unexpected end of file",
            ));
        }

        let start_pos = self.log_file.index[line_number];
        let line_size = if line_number + 1 == self.log_file.index.len() {
            self.log_file.mmap.len() // Last line
        } else {
            self.log_file.index[line_number + 1]
        }
        .saturating_sub(start_pos);
        let end_pos = line_size
            .min(LINE_MAX_BYTES) // Max line size validation
            .saturating_add(start_pos) // End position
            .min(self.log_file.mmap.len()); // Max file size validation

        let bytes = &self.log_file.mmap[start_pos..end_pos];

        Ok(String::from_utf8_lossy(bytes).trim_end().to_owned())
    }

    pub fn read_filter_line(&self, line_number: usize) -> io::Result<Option<String>> {
        if line_number >= self.log_file.filter.len() {
            return Ok(None);
        }

        if self.log_file.filter[line_number] {
            Ok(Some(self.read_line(line_number)?))
        } else {
            Ok(None)
        }
    }

    /// Returns the total number of lines indexed.
    /// This may not be the total number of lines in the file if indexing is in progress.
    pub fn total_lines(&self) -> io::Result<usize> {
        Ok(self.log_file.index.len())
    }

    /// Returns the total number of lines indexed.
    /// This may not be the total number of matches if filter indexing is in progress.
    pub fn filtered_lines(&self) -> io::Result<usize> {
        Ok(self
            .log_file
            .filter
            .iter()
            .filter(|matched| **matched)
            .count())
    }

    /// Returns how many source lines have already been processed by the filter worker.
    pub fn processed_filter_lines(&self) -> io::Result<usize> {
        Ok(self.log_file.filter.len())
    }

    pub fn indexing_progress(&self) -> io::Result<f64> {
        let file_size = self.log_file.mmap.len();
        if file_size == 0 {
            return Ok(1.0);
        }
        let indexed = *self.log_file.index.last().unwrap();
        Ok(indexed as f64 / file_size as f64)
    }

    pub fn filter_indexing_progress(&self) -> io::Result<f64> {
        let file_size = self.log_file.mmap.len();
        if file_size == 0 {
            return Ok(1.0);
        }
        let indexed = self.log_file.index[self.log_file.filter.len()];
        Ok(indexed as f64 / file_size as f64)
    }

    pub fn search_status(&self) -> SearchStatus {
        self.log_file.search.status()
    }

    pub fn page_search_result(&self, from_line: usize, to_line: usize) -> Option<PageSearchResult> {
        let session = self.log_file.search.session.as_ref()?;
        let page_matches = session
            .matches
            .iter()
            .filter(|m| m.line_index >= from_line && m.line_index < to_line)
            .cloned()
            .collect::<Vec<SearchMatch>>();

        Some(PageSearchResult {
            query: session.query.clone(),
            total_matches: session.matches.len(),
            total_matches_final: session.total_matches_final,
            is_indexing: !matches!(session.phase, crate::models::search::SearchPhase::Ready),
            first: session.first_match.clone(),
            current: session.current_match().cloned(),
            page_matches,
        })
    }

    pub fn page_search_result_for_lines(&self, line_indexes: &[usize]) -> Option<PageSearchResult> {
        let session = self.log_file.search.session.as_ref()?;
        let visible_lines = line_indexes.iter().copied().collect::<HashSet<_>>();
        let page_matches = session
            .matches
            .iter()
            .filter(|m| visible_lines.contains(&m.line_index))
            .cloned()
            .collect::<Vec<SearchMatch>>();

        Some(PageSearchResult {
            query: session.query.clone(),
            total_matches: session.matches.len(),
            total_matches_final: session.total_matches_final,
            is_indexing: !matches!(session.phase, crate::models::search::SearchPhase::Ready),
            first: session.first_match.clone(),
            current: session.current_match().cloned(),
            page_matches,
        })
    }

    pub fn compute_search_batch(
        log_file: &LogFile,
        query: &str,
        start_line: usize,
        end_line: usize,
    ) -> io::Result<Vec<SearchMatch>> {
        let re = Regex::new(query).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut batch = Vec::new();
        for i in start_line..end_line {
            let start_pos = log_file.index[i];
            let end_pos = log_file.index[i + 1];
            let line = &log_file.mmap[start_pos..end_pos];
            if let Ok(text) = std::str::from_utf8(line) {
                for found in re.find_iter(text) {
                    batch.push(SearchMatch {
                        line_index: i,
                        start: found.start(),
                        end: found.end(),
                        ordinal: 0,
                    });
                }
            }
        }
        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_ops::write::FileWriteOps;
    use crate::models::log_file::LogFile;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::RwLock;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("logmancer-{prefix}-{unique}.log"))
    }

    #[test]
    fn test_read_line() {
        // Prepara un archivo temporal
        let path = "test_log.txt";
        let mut file = File::create(path).unwrap();
        writeln!(file, "line1").unwrap();

        let log_file = LogFile::new(path.to_string()).unwrap();
        let lock = RwLock::new(log_file);
        let read_ops = FileReadOps::new(lock.read().unwrap());

        assert_eq!(read_ops.read_line(0).unwrap(), "line1");

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn compute_search_batch_collects_multiple_matches_per_line() {
        let path = "test_log_search_batch.txt";
        let mut file = File::create(path).unwrap();
        writeln!(file, "foo foo").unwrap();
        writeln!(file, "bar").unwrap();
        writeln!(file, "foo").unwrap();

        let log_file = RwLock::new(LogFile::new(path.to_string()).unwrap());
        let mut write_ops = FileWriteOps::new(std::sync::Arc::new(log_file));
        while !write_ops.index_lines().unwrap() {}

        let shared = write_ops.log_file();
        let read_guard = shared.read().unwrap();
        let batch = FileReadOps::compute_search_batch(&read_guard, "foo", 0, 3).unwrap();

        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].line_index, 0);
        assert_eq!(batch[1].line_index, 0);
        assert_eq!(batch[2].line_index, 2);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn read_filter_line_returns_none_for_unprocessed_filter_bounds() {
        let path = temp_file_path("read-filter-line-unprocessed-bounds");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "alpha").unwrap();
        write!(file, "beta").unwrap();
        drop(file);

        let log_file = RwLock::new(LogFile::new(path.to_string_lossy().into_owned()).unwrap());
        let mut write_ops = FileWriteOps::new(std::sync::Arc::new(log_file));
        while !write_ops.index_lines().unwrap() {}

        let shared = write_ops.log_file();
        let read_guard = shared.read().unwrap();
        let read_ops = FileReadOps::new(read_guard);

        assert_eq!(read_ops.total_lines().unwrap(), 2);
        assert_eq!(read_ops.processed_filter_lines().unwrap(), 0);
        assert_eq!(read_ops.read_filter_line(0).unwrap(), None);
        assert_eq!(read_ops.read_filter_line(2).unwrap(), None);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn page_search_result_for_lines_keeps_only_visible_matches() {
        let path = temp_file_path("page-search-visible-lines");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "foo").unwrap();
        writeln!(file, "bar foo").unwrap();
        writeln!(file, "baz").unwrap();

        let log_file = RwLock::new(LogFile::new(path.to_string_lossy().into_owned()).unwrap());
        let mut write_ops = FileWriteOps::new(std::sync::Arc::new(log_file));
        while !write_ops.index_lines().unwrap() {}
        write_ops.begin_search(1, "foo".to_string(), 0);
        write_ops.merge_search_batch(
            1,
            vec![
                SearchMatch {
                    line_index: 0,
                    start: 0,
                    end: 3,
                    ordinal: 0,
                },
                SearchMatch {
                    line_index: 1,
                    start: 4,
                    end: 7,
                    ordinal: 1,
                },
            ],
            true,
        );

        let shared = write_ops.log_file();
        let read_guard = shared.read().unwrap();
        let read_ops = FileReadOps::new(read_guard);
        let search = read_ops
            .page_search_result_for_lines(&[1])
            .expect("search metadata");

        assert_eq!(search.page_matches.len(), 1);
        assert_eq!(search.page_matches[0].line_index, 1);
        assert_eq!(search.page_matches[0].start, 4);
        assert_eq!(search.current.as_ref().map(|m| m.line_index), Some(0));

        std::fs::remove_file(path).unwrap();
    }
}

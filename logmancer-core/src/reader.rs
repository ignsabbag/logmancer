use crate::handler::LogFileHandler;
use crate::models::{FileInfo, PageLine, PageResult, SearchStatus};
use log::debug;
use std::cmp::min;
use std::io::{self};

pub struct LogReader {
    handler: LogFileHandler,
    current_view_start: usize,
}

impl LogReader {
    pub fn new(path: String) -> io::Result<Self> {
        let file_log_handler = LogFileHandler::new(path)?;
        Ok(LogReader {
            handler: file_log_handler,
            current_view_start: 0,
        })
    }

    /// Return file_id, path and other info about the open file
    pub fn file_info(&self) -> io::Result<FileInfo> {
        let read_ops = self.handler.read_ops();
        let file_info = FileInfo {
            path: read_ops.file_path(),
            total_lines: read_ops.total_lines()?,
            indexing_progress: read_ops.indexing_progress()?,
        };
        debug!("{file_info:?}");
        Ok(file_info)
    }

    /// Reads a page from the file, starting at `start_line` and reading up to `max_lines` lines.
    pub fn read_page(&mut self, start_line: usize, max_lines: usize) -> io::Result<PageResult> {
        debug!("Reading from line {start_line} to max {max_lines}");
        let read_ops = self.handler.read_ops();
        let to_line = min(start_line + max_lines, read_ops.total_lines()?);
        let from_line = to_line.saturating_sub(max_lines);
        let mut lines = Vec::with_capacity(max_lines);
        for current_line in from_line..to_line {
            lines.push(PageLine {
                number: current_line + 1,
                text: read_ops.read_line(current_line)?,
            });
        }
        let page = PageResult {
            lines,
            start_line: from_line,
            total_lines: read_ops.total_lines()?,
            indexing_progress: read_ops.indexing_progress()?,
            search: read_ops.page_search_result(from_line, to_line),
        };
        self.current_view_start = page.start_line;
        Ok(page)
    }

    // Reads the last `max_lines` lines from the file. If `follow` is true the file is reloaded
    pub fn tail(&mut self, max_lines: usize, follow: bool) -> io::Result<PageResult> {
        debug!("Reading last {max_lines} lines to the end");
        if follow {
            self.handler.reload();
        }
        let read_ops = self.handler.read_ops();
        let total_lines = read_ops.total_lines()?;
        let start_line = total_lines.saturating_sub(max_lines);
        let mut lines = Vec::with_capacity(max_lines);
        for current_line in start_line..total_lines {
            lines.push(PageLine {
                number: current_line + 1,
                text: read_ops.read_line(current_line)?,
            });
        }
        let page = PageResult {
            lines,
            start_line,
            total_lines,
            indexing_progress: read_ops.indexing_progress()?,
            search: read_ops.page_search_result(start_line, total_lines),
        };
        self.current_view_start = page.start_line;
        Ok(page)
    }

    pub fn filter(&mut self, regex: String) {
        self.handler.filter(Some(regex));
    }

    pub fn read_filter(&mut self, start_line: usize, max_lines: usize) -> io::Result<PageResult> {
        debug!("Reading filter from line {start_line} to max {max_lines}");
        let read_ops = self.handler.read_ops();

        let total_lines = read_ops.filtered_lines()?;
        let processed_lines = read_ops.processed_filter_lines()?;
        let mut matched_lines = 0;
        let mut current_line = 0;
        let mut lines = Vec::with_capacity(max_lines);

        while lines.len() < max_lines && current_line < processed_lines {
            if let Some(line) = read_ops.read_filter_line(current_line)? {
                if matched_lines >= start_line {
                    lines.push(PageLine {
                        number: current_line + 1,
                        text: line,
                    });
                }
                matched_lines += 1;
            }
            current_line += 1;
        }
        let page = PageResult {
            lines,
            start_line,
            total_lines,
            indexing_progress: read_ops.filter_indexing_progress()?,
            search: None,
        };
        self.current_view_start = page.start_line;
        Ok(page)
    }

    pub fn tail_filter(&mut self, max_lines: usize, follow: bool) -> io::Result<PageResult> {
        debug!("Reading last {max_lines} lines to the end");
        if follow {
            self.handler.filter(None);
        }
        let read_ops = self.handler.read_ops();
        let mut lines = Vec::with_capacity(max_lines);
        let mut current_line = read_ops.total_lines()?;

        while lines.len() < max_lines && current_line > 0 {
            current_line -= 1;
            if let Some(line) = read_ops.read_filter_line(current_line)? {
                lines.push(PageLine {
                    number: current_line + 1,
                    text: line,
                });
            }
        }
        lines.reverse();
        let page = PageResult {
            lines,
            start_line: current_line,
            total_lines: read_ops.total_lines()?,
            indexing_progress: read_ops.filter_indexing_progress()?,
            search: None,
        };
        self.current_view_start = page.start_line;
        Ok(page)
    }

    pub fn apply_search(&mut self, query: String, max_lines: usize) -> io::Result<PageResult> {
        self.handler.apply_search(query, self.current_view_start)?;
        let status = self.search_status();
        let start = status
            .current
            .map(|m| m.line_index.saturating_sub(max_lines / 2))
            .unwrap_or(self.current_view_start);
        self.read_page(start, max_lines)
    }

    pub fn clear_search(&mut self) {
        self.handler.clear_search();
    }

    pub fn search_status(&self) -> SearchStatus {
        self.handler.read_ops().search_status()
    }

    pub fn search_next(&mut self, max_lines: usize) -> io::Result<PageResult> {
        self.handler.search_next();
        self.search_positioned_page(max_lines)
    }

    pub fn search_previous(&mut self, max_lines: usize) -> io::Result<PageResult> {
        self.handler.search_previous();
        self.search_positioned_page(max_lines)
    }

    fn search_positioned_page(&mut self, max_lines: usize) -> io::Result<PageResult> {
        let status = self.search_status();
        let start = status
            .current
            .map(|m| m.line_index.saturating_sub(max_lines / 2))
            .unwrap_or(0);
        self.read_page(start, max_lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::thread::sleep;
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("logmancer-{name}-{suffix}.log"))
    }

    fn wait_search_ready(reader: &LogReader) {
        for _ in 0..40 {
            let status = reader.search_status();
            if status.is_ready {
                return;
            }
            sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn read_filter_uses_matched_line_indexes() {
        let path = temp_file_path("filter-pagination");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "alpha").unwrap();
        writeln!(file, "beta match").unwrap();
        writeln!(file, "gamma").unwrap();
        writeln!(file, "delta match").unwrap();
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        reader.filter("match".to_string());

        let first_page = reader.read_filter(0, 1).unwrap();
        assert_eq!(first_page.total_lines, 2);
        assert_eq!(
            first_page.lines,
            vec![PageLine {
                number: 2,
                text: "beta match".to_string(),
            }]
        );

        let second_page = reader.read_filter(1, 1).unwrap();
        assert_eq!(second_page.total_lines, 2);
        assert_eq!(
            second_page.lines,
            vec![PageLine {
                number: 4,
                text: "delta match".to_string(),
            }]
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn read_page_exposes_real_source_line_numbers() {
        let path = temp_file_path("read-page-line-numbers");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "zero").unwrap();
        writeln!(file, "one").unwrap();
        writeln!(file, "two").unwrap();
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        for _ in 0..10 {
            if reader.file_info().unwrap().total_lines >= 3 {
                break;
            }
            sleep(Duration::from_millis(50));
        }
        let page = reader.read_page(1, 2).unwrap();

        assert_eq!(
            page.lines,
            vec![
                PageLine {
                    number: 2,
                    text: "one".to_string(),
                },
                PageLine {
                    number: 3,
                    text: "two".to_string(),
                },
            ]
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tail_handles_files_smaller_than_requested_page() {
        let path = temp_file_path("tail-underflow");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "first").unwrap();
        write!(file, "second").unwrap();
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        for _ in 0..10 {
            if reader.file_info().unwrap().total_lines >= 2 {
                break;
            }
            sleep(Duration::from_millis(50));
        }

        let page = reader.tail(50, false).unwrap();

        assert_eq!(page.start_line, 0);
        assert_eq!(page.total_lines, 2);
        assert_eq!(
            page.lines,
            vec![
                PageLine {
                    number: 1,
                    text: "first".to_string(),
                },
                PageLine {
                    number: 2,
                    text: "second".to_string(),
                },
            ]
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn search_status_and_page_metadata_include_multi_occurrence_spans() {
        let path = temp_file_path("search-multi-occurrence");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "foo foo").unwrap();
        writeln!(file, "bar").unwrap();
        writeln!(file, "foo").unwrap();
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        for _ in 0..10 {
            if reader.file_info().unwrap().total_lines >= 3 {
                break;
            }
            sleep(Duration::from_millis(50));
        }

        let page = reader.apply_search("foo".to_string(), 10).unwrap();
        let status = reader.search_status();
        if status.is_ready {
            assert!(status.total_matches_final);
        } else {
            assert!(!status.total_matches_final);
        }
        wait_search_ready(&reader);
        let status = reader.search_status();
        assert_eq!(status.total_matches, 3);
        assert!(status.total_matches_final);
        let current = status.current.unwrap();
        assert_eq!(current.line_index, 0);
        assert_eq!(current.start, 0);
        assert_eq!(current.end, 3);
        assert_eq!(current.ordinal, 0);

        let search = page.search.expect("search metadata");
        assert!(search.is_indexing || search.total_matches_final);

        let page_after_ready = reader.read_page(0, 10).unwrap();
        let search = page_after_ready
            .search
            .expect("search metadata after ready");
        assert_eq!(search.page_matches.len(), 3);
        assert_eq!(search.page_matches[0].ordinal, 0);
        assert_eq!(search.page_matches[1].ordinal, 1);
        assert_eq!(search.page_matches[2].ordinal, 2);
        assert_eq!(search.page_matches[1].line_index, 0);
        assert_eq!(search.page_matches[1].start, 4);
        assert_eq!(search.page_matches[1].end, 7);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn search_navigation_wraps_and_positions_page_around_current_match() {
        let path = temp_file_path("search-wrap");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "a").unwrap();
        writeln!(file, "foo").unwrap();
        writeln!(file, "b").unwrap();
        writeln!(file, "foo").unwrap();
        writeln!(file, "c").unwrap();
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        for _ in 0..10 {
            if reader.file_info().unwrap().total_lines >= 5 {
                break;
            }
            sleep(Duration::from_millis(50));
        }

        let first = reader.apply_search("foo".to_string(), 3).unwrap();
        wait_search_ready(&reader);
        assert!(first.search.is_some());
        assert_eq!(reader.search_status().current.as_ref().unwrap().ordinal, 0);

        let second = reader.search_next(3).unwrap();
        assert_eq!(
            second
                .search
                .as_ref()
                .unwrap()
                .current
                .as_ref()
                .unwrap()
                .ordinal,
            1
        );

        let wrapped = reader.search_next(3).unwrap();
        assert_eq!(
            wrapped
                .search
                .as_ref()
                .unwrap()
                .current
                .as_ref()
                .unwrap()
                .ordinal,
            0
        );

        let previous_wrap = reader.search_previous(3).unwrap();
        assert_eq!(
            previous_wrap
                .search
                .as_ref()
                .unwrap()
                .current
                .as_ref()
                .unwrap()
                .ordinal,
            1
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn search_selection_is_independent_from_scroll() {
        let path = temp_file_path("search-scroll-independent");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "foo").unwrap();
        writeln!(file, "a").unwrap();
        writeln!(file, "b").unwrap();
        writeln!(file, "foo").unwrap();
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        for _ in 0..10 {
            if reader.file_info().unwrap().total_lines >= 4 {
                break;
            }
            sleep(Duration::from_millis(50));
        }

        reader.apply_search("foo".to_string(), 2).unwrap();
        wait_search_ready(&reader);
        reader.search_next(2).unwrap();
        let selected = reader.search_status().current.unwrap();
        assert_eq!(selected.ordinal, 1);

        let scrolled_page = reader.read_page(0, 2).unwrap();
        assert!(scrolled_page.search.is_some());
        assert_eq!(reader.search_status().current.unwrap().ordinal, 1);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn apply_search_returns_before_full_indexing_ready() {
        let path = temp_file_path("search-async-fast-return");
        let mut file = File::create(&path).unwrap();
        for i in 0..8_000 {
            writeln!(file, "line {i} foo").unwrap();
        }
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        for _ in 0..40 {
            if reader.file_info().unwrap().total_lines >= 8_000 {
                break;
            }
            sleep(Duration::from_millis(20));
        }

        let _ = reader.apply_search("foo".to_string(), 40).unwrap();
        let status = reader.search_status();
        assert!(!status.is_ready);
        assert!(!status.total_matches_final);

        wait_search_ready(&reader);
        assert!(reader.search_status().is_ready);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn apply_search_waits_briefly_and_can_jump_to_nearby_first_match() {
        let path = temp_file_path("search-bounded-initial-wait");
        let mut file = File::create(&path).unwrap();
        for i in 0..3000 {
            if i == 1200 {
                writeln!(file, "line {i} foo").unwrap();
            } else {
                writeln!(file, "line {i} bar").unwrap();
            }
        }
        drop(file);

        let mut reader = LogReader::new(path.to_string_lossy().into_owned()).unwrap();
        for _ in 0..60 {
            if reader.file_info().unwrap().total_lines >= 3000 {
                break;
            }
            sleep(Duration::from_millis(20));
        }

        let _ = reader.read_page(1195, 10).unwrap();
        let page = reader.apply_search("foo".to_string(), 10).unwrap();

        let search = page.search.expect("search metadata expected");
        assert!(search.current.is_some() || search.first.is_some() || search.is_indexing);
        if let Some(current) = search.current {
            assert!(current.line_index >= 1190 && current.line_index <= 1210);
        }

        std::fs::remove_file(path).unwrap();
    }
}

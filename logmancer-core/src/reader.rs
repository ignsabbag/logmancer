use crate::handler::LogFileHandler;
use crate::models::{FileInfo, PageLine, PageResult};
use log::debug;
use std::cmp::min;
use std::io::{self};

pub struct LogReader {
    handler: LogFileHandler,
}

impl LogReader {
    pub fn new(path: String) -> io::Result<Self> {
        let file_log_handler = LogFileHandler::new(path)?;
        Ok(LogReader {
            handler: file_log_handler,
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
        Ok(PageResult {
            lines,
            start_line: from_line,
            total_lines: read_ops.total_lines()?,
            indexing_progress: read_ops.indexing_progress()?,
        })
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
        Ok(PageResult {
            lines,
            start_line,
            total_lines,
            indexing_progress: read_ops.indexing_progress()?,
        })
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
        Ok(PageResult {
            lines,
            start_line,
            total_lines,
            indexing_progress: read_ops.filter_indexing_progress()?,
        })
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
        Ok(PageResult {
            lines,
            start_line: current_line,
            total_lines: read_ops.total_lines()?,
            indexing_progress: read_ops.filter_indexing_progress()?,
        })
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
}

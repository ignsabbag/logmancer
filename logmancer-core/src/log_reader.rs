use std::cmp::min;
use std::io::{self};
use log::debug;
use crate::log_handler::LogHandler;

pub struct PageResult {
    pub lines: Vec<String>,
    pub start_line: usize,
    pub total_lines: usize,
}

pub struct LogReader {
    log_handler: LogHandler
}

impl LogReader {

    pub fn new(path: String) -> io::Result<Self> {
        let log_handler = LogHandler::new(path)?;
        Ok(LogReader { log_handler })
    }

    /// Reads a page from the file, starting at `start_line` and reading up to `max_lines` lines.
    pub fn read_page(&mut self, start_line: usize, max_lines: usize) -> io::Result<PageResult> {
        debug!("Reading from line {} to max {}", start_line, max_lines);
        let read_ops = self.log_handler.read_ops();
        let to_line = min(start_line + max_lines, read_ops.total_lines()?);
        let from_line = to_line.saturating_sub(max_lines);
        let mut lines = Vec::with_capacity(max_lines);
        for current_line in from_line..to_line {
            lines.push(read_ops.read_line(current_line)?);
        }
        Ok(PageResult { lines, start_line: from_line, total_lines: read_ops.total_lines()? })
    }

    // Reads the last `max_lines` lines from the file. If `follow` is true the file is reloaded
    pub fn tail(&mut self, max_lines: usize, follow: bool) -> io::Result<PageResult> {
        debug!("Reading last {} lines to the end", max_lines);
        if follow {
            self.log_handler.reload();
        }
        let read_ops = self.log_handler.read_ops();
        let start_line = read_ops.total_lines()? - max_lines;
        let mut lines = Vec::with_capacity(max_lines);
        for current_line in start_line..read_ops.total_lines()? {
            lines.push(read_ops.read_line(current_line)?);
        }
        Ok(PageResult {
            lines,
            start_line,
            total_lines: read_ops.total_lines()?,
        })
    }
}
use crate::file_ops::read::FileReadOps;
use crate::file_ops::write::FileWriteOps;
use crate::models::log_file::LogFile;
use crate::timing::{SEARCH_INITIAL_PROGRESS_WAIT, SEARCH_PROGRESS_POLL_INTERVAL};
use crate::workers::{
    SearchCommand, spawn_filter_worker, spawn_reload_worker, spawn_search_worker,
};
use crossbeam_channel::{Sender, unbounded};
use log::info;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::{io, thread, time};

pub struct LogFileHandler {
    log_file: Arc<RwLock<LogFile>>,
    reload_sender: Sender<()>,
    filter_sender: Sender<Option<String>>, // New sender for filter thread
    search_sender: Sender<SearchCommand>,
    search_generation: u64,
    write_ops: FileWriteOps,
}

impl LogFileHandler {
    pub fn new(path: String) -> io::Result<Self> {
        let (reload_sender, reload_receiver) = unbounded::<()>();
        let (filter_sender, filter_receiver) = unbounded::<Option<String>>();
        let (search_sender, search_receiver) = unbounded::<SearchCommand>();
        let log_file = Arc::new(RwLock::new(LogFile::new(path.clone())?));
        info!("File {path} loaded");

        let reload_write_ops = FileWriteOps::new(Arc::clone(&log_file));
        let filter_write_ops = FileWriteOps::new(Arc::clone(&log_file));
        let search_write_ops = FileWriteOps::new(Arc::clone(&log_file));
        let write_ops = FileWriteOps::new(Arc::clone(&log_file));

        spawn_reload_worker(reload_write_ops, reload_receiver, filter_sender.clone());
        spawn_filter_worker(filter_write_ops, filter_receiver);
        spawn_search_worker(search_write_ops, search_receiver);

        reload_sender.send(()).unwrap();

        Ok(LogFileHandler {
            log_file,
            reload_sender,
            filter_sender,
            search_sender,
            search_generation: 0,
            write_ops,
        })
    }

    pub fn reload(&mut self) {
        self.reload_sender.send(()).unwrap();
        Self::wait(500);
    }

    pub fn filter(&mut self, regex: Option<String>) {
        self.filter_sender.send(regex).unwrap(); // Send regex to filter thread
        Self::wait(500);
    }

    fn wait(millis: u64) {
        let ten_millis = Duration::from_millis(millis);
        let now = time::Instant::now();
        while now.elapsed() < ten_millis {
            thread::sleep(ten_millis);
        }
    }

    pub fn read_ops(&self) -> FileReadOps<'_> {
        FileReadOps::new(self.log_file.read().unwrap())
    }

    pub fn apply_search(&mut self, query: String, origin_line: usize) -> io::Result<()> {
        self.search_generation += 1;
        let generation = self.search_generation;
        let indexed_lines = self.read_ops().total_lines()?;
        self.write_ops
            .begin_search(generation, query.clone(), origin_line);
        self.search_sender
            .send(SearchCommand::Start {
                generation,
                query,
                origin_line,
                indexed_lines,
            })
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))?;

        self.wait_search_progress(generation);
        Ok(())
    }

    fn wait_search_progress(&self, generation: u64) {
        let deadline = Instant::now() + SEARCH_INITIAL_PROGRESS_WAIT;
        loop {
            let status = self.read_ops().search_status();
            if status.generation != generation {
                return;
            }
            if status.is_ready || status.current.is_some() || status.first.is_some() {
                return;
            }
            if Instant::now() >= deadline {
                return;
            }
            thread::sleep(SEARCH_PROGRESS_POLL_INTERVAL);
        }
    }

    pub fn clear_search(&mut self) {
        self.write_ops.clear_search();
    }

    pub fn search_next(&mut self) {
        self.write_ops.search_next();
    }

    pub fn search_previous(&mut self) {
        self.write_ops.search_previous();
    }
}

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
    shutdown_senders: Vec<Sender<()>>,
    worker_handles: Vec<thread::JoinHandle<()>>,
    #[cfg(test)]
    drop_join_started: Option<Sender<()>>,
    search_generation: u64,
    write_ops: FileWriteOps,
}

impl LogFileHandler {
    pub fn new(path: String) -> io::Result<Self> {
        Self::new_with_reload_batch_sync(
            path,
            #[cfg(test)]
            None,
            #[cfg(test)]
            None,
        )
    }

    fn new_with_reload_batch_sync(
        path: String,
        #[cfg(test)] reload_batch_sync: Option<(Sender<()>, crossbeam_channel::Receiver<()>)>,
        #[cfg(test)] drop_join_started: Option<Sender<()>>,
    ) -> io::Result<Self> {
        let (reload_sender, reload_receiver) = unbounded::<()>();
        let (filter_sender, filter_receiver) = unbounded::<Option<String>>();
        let (search_sender, search_receiver) = unbounded::<SearchCommand>();
        let (reload_shutdown_sender, reload_shutdown_receiver) = unbounded::<()>();
        let (filter_shutdown_sender, filter_shutdown_receiver) = unbounded::<()>();
        let (search_shutdown_sender, search_shutdown_receiver) = unbounded::<()>();
        let log_file = Arc::new(RwLock::new(LogFile::new(path.clone())?));
        info!("File {path} loaded");

        let reload_write_ops = FileWriteOps::new(Arc::clone(&log_file));
        let filter_write_ops = FileWriteOps::new(Arc::clone(&log_file));
        let search_write_ops = FileWriteOps::new(Arc::clone(&log_file));
        let write_ops = FileWriteOps::new(Arc::clone(&log_file));

        let reload_worker = spawn_reload_worker(
            reload_write_ops,
            reload_receiver,
            filter_sender.clone(),
            reload_shutdown_receiver,
            #[cfg(test)]
            reload_batch_sync,
        );
        let filter_worker = spawn_filter_worker(
            filter_write_ops,
            filter_receiver,
            filter_shutdown_receiver,
            #[cfg(test)]
            None,
        );
        let search_worker = spawn_search_worker(
            search_write_ops,
            search_receiver,
            search_shutdown_receiver,
            #[cfg(test)]
            None,
        );

        reload_sender.send(()).unwrap();

        Ok(LogFileHandler {
            log_file,
            reload_sender,
            filter_sender,
            search_sender,
            shutdown_senders: vec![
                reload_shutdown_sender,
                filter_shutdown_sender,
                search_shutdown_sender,
            ],
            worker_handles: vec![reload_worker, filter_worker, search_worker],
            #[cfg(test)]
            drop_join_started,
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

impl Drop for LogFileHandler {
    fn drop(&mut self) {
        for sender in &self.shutdown_senders {
            let _ = sender.send(());
        }
        #[cfg(test)]
        if let Some(sender) = &self.drop_join_started {
            let _ = sender.send(());
        }
        for worker in self.worker_handles.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::TryRecvError;

    #[test]
    fn drop_joins_workers_while_initial_reload_is_active() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "line\n".repeat(3_000_000)).unwrap();

        let (batch_started_sender, batch_started) = crossbeam_channel::bounded(1);
        let (resume_sender, resume_receiver) = crossbeam_channel::bounded(1);
        let (drop_join_started_sender, drop_join_started) = crossbeam_channel::bounded(1);
        let handler = LogFileHandler::new_with_reload_batch_sync(
            file.path().display().to_string(),
            Some((batch_started_sender, resume_receiver)),
            Some(drop_join_started_sender),
        )
        .unwrap();

        batch_started.recv().unwrap();

        let (drop_finished_sender, drop_finished) = crossbeam_channel::bounded(1);
        let drop_thread = std::thread::spawn(move || {
            drop(handler);
            drop_finished_sender.send(()).unwrap();
        });

        drop_join_started.recv().unwrap();
        assert_eq!(drop_finished.try_recv(), Err(TryRecvError::Empty));
        resume_sender.send(()).unwrap();
        drop_finished.recv().unwrap();
        drop_thread.join().unwrap();
    }
}

use crate::file_ops::read::FileReadOps;
use crate::file_ops::write::FileWriteOps;
use crate::models::log_file::LogFile;
use crate::worker::{spawn_reload_worker, spawn_filter_worker};
use crossbeam_channel::{unbounded, Sender};
use log::info;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread, time};

pub struct LogFileHandler {
    log_file: Arc<RwLock<LogFile>>,
    reload_sender: Sender<()>,
    filter_sender: Sender<Option<String>>, // New sender for filter thread
}

impl LogFileHandler {

    pub fn new(path: String) -> io::Result<Self> {
        let (reload_sender, reload_receiver) = unbounded::<()>();
        let (filter_sender, filter_receiver) = unbounded::<Option<String>>();
        let log_file = Arc::new(RwLock::new(LogFile::new(path.clone())?));
        info!("File {path} loaded");

        let reload_write_ops = FileWriteOps::new(Arc::clone(&log_file));
        let filter_write_ops = FileWriteOps::new(Arc::clone(&log_file));

        spawn_reload_worker(reload_write_ops, reload_receiver, filter_sender.clone());
        spawn_filter_worker(filter_write_ops, filter_receiver);

        reload_sender.send(()).unwrap();

        Ok(LogFileHandler { log_file, reload_sender, filter_sender })
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

}

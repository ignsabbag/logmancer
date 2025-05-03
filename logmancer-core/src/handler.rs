use crate::file_ops::read::FileReadOps;
use crate::file_ops::write::FileWriteOps;
use crate::models::log_file::LogFile;
use crossbeam_channel::{select, unbounded, Sender};
use log::info;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread, time};

const BLOCK_BYTES: usize = 1024 * 1024; // 1MB

pub struct LogFileHandler {
    log_file: Arc<RwLock<LogFile>>,
    sender: Sender<()>
}

impl LogFileHandler {

    pub fn new(path: String) -> io::Result<Self> {
        let (sender, receiver) = unbounded::<()>();
        let log_file = Arc::new(RwLock::new(LogFile::new(path.clone())?));
        info!("File {} loaded", path);

        let mut write_ops = FileWriteOps::new(Arc::clone(&log_file));

        thread::spawn(move || {
            loop {
                select! {
                    recv(receiver) -> _ => {
                        match write_ops.reload() {
                            Ok(()) => {
                                loop {
                                    match write_ops.index_max(BLOCK_BYTES) {
                                        Ok(end_reached) => {
                                            if end_reached {
                                                break;
                                            }
                                            Self::wait(1);
                                        }
                                        Err(error) => {
                                            panic!("Error indexing file: {}", error)
                                        }
                                    }
                                }
                            }
                            Err(error) => {
                                panic!("Error reloading file: {}", error)
                            }
                        }
                    }
                    default(Duration::from_secs(5)) => {
                    }
                }
            }
        });

        sender.send(()).unwrap();

        Ok(LogFileHandler { log_file, sender })
    }

    pub fn reload(&mut self) {
        self.sender.send(()).unwrap();
        Self::wait(500);
    }

    fn wait(millis: u64) {
        let ten_millis = Duration::from_millis(millis);
        let now = time::Instant::now();
        while now.elapsed() < ten_millis {
            thread::sleep(ten_millis);
        }
    }

    pub fn read_ops(&self) -> FileReadOps {
        FileReadOps::new(self.log_file.read().unwrap())
    }

}
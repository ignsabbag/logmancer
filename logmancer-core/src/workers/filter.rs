use crate::file_ops::write::FileWriteOps;
use crate::workers::common::wait;
#[cfg(test)]
use crossbeam_channel::Sender;
use crossbeam_channel::{Receiver, TryRecvError, select};
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(test)]
pub(crate) struct BatchSync {
    started: Sender<()>,
    resume: Receiver<()>,
}

pub fn spawn_filter_worker(
    mut write_ops: FileWriteOps,
    filter_receiver: Receiver<Option<String>>,
    shutdown_receiver: Receiver<()>,
    #[cfg(test)] batch_sync: Option<BatchSync>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            select! {
                recv(shutdown_receiver) -> _ => break,
                recv(filter_receiver) -> msg => {
                    match msg {
                        Ok(pattern) => {
                            write_ops.filter(pattern).unwrap();
                            if !index_filter_until_complete(
                                &mut write_ops,
                                &shutdown_receiver,
                                #[cfg(test)]
                                batch_sync.as_ref(),
                            ) {
                                return;
                            }
                        }
                        Err(_) => break,
                    }
                }
                default(Duration::from_secs(5)) => {}
            }
        }
    })
}

fn index_filter_until_complete(
    write_ops: &mut FileWriteOps,
    shutdown_receiver: &Receiver<()>,
    #[cfg(test)] batch_sync: Option<&BatchSync>,
) -> bool {
    loop {
        match write_ops.index_filter() {
            Ok(end_reached) => {
                if end_reached {
                    return true;
                }
                #[cfg(test)]
                if let Some(batch_sync) = batch_sync {
                    let _ = batch_sync.started.send(());
                    let _ = batch_sync.resume.recv();
                }
                if !matches!(shutdown_receiver.try_recv(), Err(TryRecvError::Empty)) {
                    return false;
                }
                wait(1);
            }
            Err(error) => panic!("Error indexing filtered lines: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::log_file::LogFile;
    use std::sync::{Arc, RwLock};

    #[test]
    fn stops_spawned_worker_during_an_active_filter_batch() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "matching line\n".repeat(2_001)).unwrap();
        let log_file = Arc::new(RwLock::new(
            LogFile::new(file.path().display().to_string()).unwrap(),
        ));
        let mut write_ops = FileWriteOps::new(log_file);
        while !write_ops.index_lines().unwrap() {}
        let (filter_sender, filter_receiver) = crossbeam_channel::unbounded();
        let (shutdown_sender, shutdown_receiver) = crossbeam_channel::unbounded();
        let (batch_started_sender, batch_started) = crossbeam_channel::bounded(1);
        let (resume_sender, resume_receiver) = crossbeam_channel::bounded(1);
        let worker = spawn_filter_worker(
            write_ops,
            filter_receiver,
            shutdown_receiver,
            Some(BatchSync {
                started: batch_started_sender,
                resume: resume_receiver,
            }),
        );

        filter_sender.send(Some("matching".to_string())).unwrap();
        batch_started.recv().unwrap();
        shutdown_sender.send(()).unwrap();
        resume_sender.send(()).unwrap();
        worker.join().unwrap();
    }
}

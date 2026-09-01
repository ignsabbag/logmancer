use crate::file_ops::write::FileWriteOps;
use crate::workers::common::wait;
use crossbeam_channel::{Receiver, Sender, TryRecvError, select};
use std::thread::JoinHandle;
use std::time::Duration;

pub fn spawn_reload_worker(
    mut write_ops: FileWriteOps,
    reload_receiver: Receiver<()>,
    filter_sender: Sender<Option<String>>,
    shutdown_receiver: Receiver<()>,
    #[cfg(test)] batch_sync: Option<(Sender<()>, Receiver<()>)>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            select! {
                recv(shutdown_receiver) -> _ => break,
                recv(reload_receiver) -> message => {
                    if message.is_err() {
                        break;
                    }
                    match write_ops.reload() {
                        Ok(()) => {
                            if !index_lines_until_complete(
                                &mut write_ops,
                                &filter_sender,
                                &shutdown_receiver,
                                #[cfg(test)]
                                batch_sync.as_ref(),
                            ) {
                                return;
                            }
                        }
                        Err(error) => {
                            panic!("Error reloading file: {error}")
                        }
                    }
                }
                default(Duration::from_secs(5)) => {}
            }
        }
    })
}

fn index_lines_until_complete(
    write_ops: &mut FileWriteOps,
    filter_sender: &Sender<Option<String>>,
    shutdown_receiver: &Receiver<()>,
    #[cfg(test)] batch_sync: Option<&(Sender<()>, Receiver<()>)>,
) -> bool {
    loop {
        match write_ops.index_lines() {
            Ok(end_reached) => {
                if filter_sender.send(None).is_err() {
                    return false;
                }
                if end_reached {
                    return true;
                }
                #[cfg(test)]
                if let Some(batch_sync) = batch_sync {
                    let _ = batch_sync.0.send(());
                    let _ = batch_sync.1.recv();
                }
                if !matches!(shutdown_receiver.try_recv(), Err(TryRecvError::Empty)) {
                    return false;
                }
                wait(1);
            }
            Err(error) => panic!("Error indexing file: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::log_file::LogFile;
    use std::sync::{Arc, RwLock};

    #[test]
    fn stops_spawned_worker_during_an_active_index_batch() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "line\n".repeat(3_000_000)).unwrap();
        let log_file = Arc::new(RwLock::new(
            LogFile::new(file.path().display().to_string()).unwrap(),
        ));
        let write_ops = FileWriteOps::new(log_file);
        let (reload_sender, reload_receiver) = crossbeam_channel::unbounded();
        let (filter_sender, _filter_receiver) = crossbeam_channel::unbounded();
        let (shutdown_sender, shutdown_receiver) = crossbeam_channel::unbounded();
        let (batch_started_sender, batch_started) = crossbeam_channel::bounded(1);
        let (resume_sender, resume_receiver) = crossbeam_channel::bounded(1);
        let worker = spawn_reload_worker(
            write_ops,
            reload_receiver,
            filter_sender,
            shutdown_receiver,
            Some((batch_started_sender, resume_receiver)),
        );

        reload_sender.send(()).unwrap();
        batch_started.recv().unwrap();
        shutdown_sender.send(()).unwrap();
        resume_sender.send(()).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn exits_when_reload_channel_is_disconnected() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let log_file = Arc::new(RwLock::new(
            LogFile::new(file.path().display().to_string()).unwrap(),
        ));
        let write_ops = FileWriteOps::new(log_file);
        let (reload_sender, reload_receiver) = crossbeam_channel::unbounded();
        let (filter_sender, _) = crossbeam_channel::unbounded();
        let (_shutdown_sender, shutdown_receiver) = crossbeam_channel::unbounded();
        let worker = spawn_reload_worker(
            write_ops,
            reload_receiver,
            filter_sender,
            shutdown_receiver,
            None,
        );

        drop(reload_sender);
        worker.join().unwrap();
    }
}

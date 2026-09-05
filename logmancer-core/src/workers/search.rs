use crate::file_ops::read::FileReadOps;
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

struct ShutdownContext<'a> {
    receiver: &'a Receiver<()>,
    #[cfg(test)]
    batch_sync: Option<&'a BatchSync>,
}

pub enum SearchCommand {
    Start {
        generation: u64,
        query: String,
        origin_line: usize,
        indexed_lines: usize,
    },
}

pub fn spawn_search_worker(
    mut write_ops: FileWriteOps,
    search_receiver: Receiver<SearchCommand>,
    shutdown_receiver: Receiver<()>,
    #[cfg(test)] batch_sync: Option<BatchSync>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            select! {
                recv(shutdown_receiver) -> _ => break,
                recv(search_receiver) -> msg => {
                    match msg {
                        Ok(SearchCommand::Start { generation, query, origin_line, indexed_lines }) => {
                            if indexed_lines <= 1 {
                                write_ops.merge_search_batch(generation, Vec::new(), true);
                                continue;
                            }

                            let total_content_lines = indexed_lines - 1;
                            let origin = origin_line.min(total_content_lines.saturating_sub(1));

                            if !process_range(
                                &mut write_ops,
                                generation,
                                &query,
                                origin,
                                total_content_lines,
                                false,
                                ShutdownContext {
                                    receiver: &shutdown_receiver,
                                    #[cfg(test)]
                                    batch_sync: batch_sync.as_ref(),
                                },
                            ) {
                                return;
                            }
                            if origin > 0 {
                                if !process_range(
                                    &mut write_ops,
                                    generation,
                                    &query,
                                    0,
                                    origin,
                                    true,
                                    ShutdownContext {
                                        receiver: &shutdown_receiver,
                                        #[cfg(test)]
                                        batch_sync: batch_sync.as_ref(),
                                    },
                                ) {
                                    return;
                                }
                            } else {
                                write_ops.merge_search_batch(generation, Vec::new(), true);
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

fn process_range(
    write_ops: &mut FileWriteOps,
    generation: u64,
    query: &str,
    start: usize,
    end: usize,
    finalize_last_batch: bool,
    shutdown: ShutdownContext<'_>,
) -> bool {
    let mut cursor = start;
    while cursor < end {
        let batch_end = usize::min(cursor + crate::file_ops::write::SEARCH_BATCH_MAX_LINES, end);
        let batch = {
            let log_file = write_ops.log_file();
            let file_lock = log_file.read().unwrap();
            match FileReadOps::compute_search_batch(&file_lock, query, cursor, batch_end) {
                Ok(batch) => batch,
                Err(error) => panic!("Error indexing search batch: {error}"),
            }
        };
        let mark_ready = finalize_last_batch && batch_end == end;
        let merged = write_ops.merge_search_batch(generation, batch, mark_ready);
        if !merged {
            break;
        }
        cursor = batch_end;
        #[cfg(test)]
        if let Some(batch_sync) = shutdown.batch_sync {
            let _ = batch_sync.started.send(());
            let _ = batch_sync.resume.recv();
        }
        if !matches!(shutdown.receiver.try_recv(), Err(TryRecvError::Empty)) {
            return false;
        }
        wait(1);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::log_file::LogFile;
    use std::sync::{Arc, RwLock};

    #[test]
    fn stops_spawned_worker_during_an_active_search_batch() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "match\n".repeat(2_001)).unwrap();
        let log_file = Arc::new(RwLock::new(
            LogFile::new(file.path().display().to_string()).unwrap(),
        ));
        let mut write_ops = FileWriteOps::new(log_file);
        while !write_ops.index_lines().unwrap() {}
        write_ops.begin_search(1, "match".to_string(), 0);
        let (search_sender, search_receiver) = crossbeam_channel::unbounded();
        let (shutdown_sender, shutdown_receiver) = crossbeam_channel::unbounded();
        let (batch_started_sender, batch_started) = crossbeam_channel::bounded(1);
        let (resume_sender, resume_receiver) = crossbeam_channel::bounded(1);
        let worker = spawn_search_worker(
            write_ops,
            search_receiver,
            shutdown_receiver,
            Some(BatchSync {
                started: batch_started_sender,
                resume: resume_receiver,
            }),
        );

        search_sender
            .send(SearchCommand::Start {
                generation: 1,
                query: "match".to_string(),
                origin_line: 0,
                indexed_lines: 2_002,
            })
            .unwrap();
        batch_started.recv().unwrap();
        shutdown_sender.send(()).unwrap();
        resume_sender.send(()).unwrap();
        worker.join().unwrap();
    }
}

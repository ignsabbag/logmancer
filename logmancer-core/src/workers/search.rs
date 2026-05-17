use crate::file_ops::read::FileReadOps;
use crate::file_ops::write::FileWriteOps;
use crate::workers::common::wait;
use crossbeam_channel::{Receiver, select};
use std::time::Duration;

pub enum SearchCommand {
    Start {
        generation: u64,
        query: String,
        origin_line: usize,
        indexed_lines: usize,
    },
}

pub fn spawn_search_worker(mut write_ops: FileWriteOps, search_receiver: Receiver<SearchCommand>) {
    std::thread::spawn(move || {
        loop {
            select! {
                recv(search_receiver) -> msg => {
                    match msg {
                        Ok(SearchCommand::Start { generation, query, origin_line, indexed_lines }) => {
                            if indexed_lines <= 1 {
                                write_ops.merge_search_batch(generation, Vec::new(), true);
                                continue;
                            }

                            let total_content_lines = indexed_lines - 1;
                            let origin = origin_line.min(total_content_lines.saturating_sub(1));

                            process_range(
                                &mut write_ops,
                                generation,
                                &query,
                                origin,
                                total_content_lines,
                                false,
                            );
                            if origin > 0 {
                                process_range(&mut write_ops, generation, &query, 0, origin, true);
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
    });
}

fn process_range(
    write_ops: &mut FileWriteOps,
    generation: u64,
    query: &str,
    start: usize,
    end: usize,
    finalize_last_batch: bool,
) {
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
        wait(1);
    }
}

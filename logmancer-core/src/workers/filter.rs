use crate::file_ops::write::FileWriteOps;
use crate::workers::common::wait;
use crossbeam_channel::{Receiver, select};
use std::time::Duration;

pub fn spawn_filter_worker(mut write_ops: FileWriteOps, filter_receiver: Receiver<Option<String>>) {
    std::thread::spawn(move || {
        loop {
            select! {
                recv(filter_receiver) -> msg => {
                    match msg {
                        Ok(pattern) => {
                            write_ops.filter(pattern).unwrap();
                            loop {
                                match write_ops.index_filter() {
                                    Ok(end_reached) => {
                                        if end_reached {
                                            break;
                                        }
                                        wait(1);
                                    }
                                    Err(error) => {
                                        panic!("Error indexing filtered lines: {error}")
                                    }
                                }
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

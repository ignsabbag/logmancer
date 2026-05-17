use crate::file_ops::write::FileWriteOps;
use crate::workers::common::wait;
use crossbeam_channel::{Receiver, Sender, select};
use std::time::Duration;

pub fn spawn_reload_worker(
    mut write_ops: FileWriteOps,
    reload_receiver: Receiver<()>,
    filter_sender: Sender<Option<String>>,
) {
    std::thread::spawn(move || {
        loop {
            select! {
                recv(reload_receiver) -> _ => {
                    match write_ops.reload() {
                        Ok(()) => {
                            loop {
                                match write_ops.index_lines() {
                                    Ok(end_reached) => {
                                        filter_sender.send(None).unwrap();
                                        if end_reached {
                                            break;
                                        }
                                        wait(1);
                                    }
                                    Err(error) => {
                                        panic!("Error indexing file: {error}")
                                    }
                                }
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
    });
}

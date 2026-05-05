use crate::file_ops::write::FileWriteOps;
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

pub fn spawn_filter_worker(
    mut write_ops: FileWriteOps,
    filter_receiver: Receiver<Option<String>>,
) {
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

fn wait(millis: u64) {
    let ten_millis = Duration::from_millis(millis);
    let now = std::time::Instant::now();
    while now.elapsed() < ten_millis {
        std::thread::sleep(ten_millis);
    }
}

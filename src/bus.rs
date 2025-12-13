//! Bus protocol for connecting consumers to producers
//!
//! The Bus has an internal bytes buffer which it uses to more efficiently
//! distribute data. Ownership-wise, the Bus is designed to own the consumers
//! but not to own the producers. It's essentially treated as an open well
//! that you throw data into and hope it reaches the right location.
use crate::{conf::ConsumerKind, consumer::Consumer};
use std::{sync::mpsc, thread};

pub type Worker = (mpsc::SyncSender<Box<[u8]>>, thread::JoinHandle<()>);

pub struct Bus {
    bound: usize,
    workers: Vec<Worker>,
}

impl Bus {
    pub fn new(bufsize: usize) -> Self {
        Self {
            bound: bufsize,
            workers: Vec::new(),
        }
    }

    pub fn add_consumer(&mut self, conf: ConsumerKind) {
        let (tx, tr) = mpsc::sync_channel::<Box<[u8]>>(self.bound);

        let handle = thread::spawn(move || {
            let mut consumer = Consumer::from_conf(conf).unwrap();

            while let Ok(bytes) = tr.recv() {
                let _ = consumer.write(&bytes);
            }
        });

        self.workers.push((tx, handle));
    }

    pub fn consume(&mut self, data: &[u8]) {
        let bytes = data.to_vec().into_boxed_slice();

        for (tx, _) in &mut self.workers {
            let _ = tx.send(bytes.clone());
        }
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        for (tx, handle) in self.workers.drain(..) {
            drop(tx);

            let _ = handle.join();
        }
    }
}

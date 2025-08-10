//! Bus protocol for connecting consumers to producers
//!
//! The Bus has an internal bytes buffer which it uses to more efficiently
//! distribute data. Ownership-wise, the Bus is designed to own the consumers
//! but not to own the producers. It's essentially treated as an open well
//! that you throw data into and hope it reaches the right location.
use crate::consumer::Consumer;
use std::io;

pub struct Bus {
    buffer: Box<[u8]>,
    curr_len: usize,
    consumers: Vec<Consumer>,
}

impl Bus {
    pub fn new(bufsize: usize) -> Self {
        let buffer = unsafe { Box::new_uninit_slice(bufsize).assume_init() };
        Self {
            buffer,
            curr_len: 0,
            consumers: Vec::new(),
        }
    }

    pub fn add_consumer(&mut self, consumer: Consumer) {
        self.consumers.push(consumer)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.curr_len == 0 {
            return Ok(());
        }

        // Execute all callbacks on the current buffer
        for consumer in &mut self.consumers {
            consumer.write(&self.buffer[..self.curr_len])?;
        }

        // Reset the buffer after flushing
        self.curr_len = 0;
        Ok(())
    }

    pub fn consume(&mut self, data: &[u8]) -> io::Result<()> {
        if self.buffer.len() == 0 {
            for consumer in &mut self.consumers {
                consumer.write(data)?;
            }
            return Ok(());
        }

        let num_bytes = data.len();
        let mut bytes_left = num_bytes;

        // if self.read_buffer.len() == self.write_buffer.len() this loop is bounded to
        // at most 2 iterations.
        while bytes_left > 0 {
            // Ensure that we don't write beyond the buffer's capacity
            let available_space = self.buffer.len() - self.curr_len;
            let bytes_to_consume = std::cmp::min(available_space, bytes_left);

            // slice out the part of the log we're going to use
            let start_idx = num_bytes - bytes_left;
            let slice = &data[start_idx..start_idx + bytes_to_consume];

            // Copy the data into the buffer
            self.buffer[self.curr_len..self.curr_len + bytes_to_consume].copy_from_slice(slice);

            // Update the buffer length
            self.curr_len += bytes_to_consume;

            // If the buffer is full, flush it automatically
            if self.curr_len == self.buffer.len() {
                self.flush()?;
            }

            // update the log amount left
            bytes_left -= bytes_to_consume;
        }
        Ok(())
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        // Ensure the buffer is flushed when the struct is dropped
        if self.curr_len > 0 {
            match self.flush() {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Failed to flush buffer: {}", e);
                }
            }
        }
    }
}

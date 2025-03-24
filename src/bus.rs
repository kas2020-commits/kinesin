//! Bus protocol for connecting consumers to producers
//!
//! The Bus has an internal bytes buffer which it uses to more efficiently
//! distribute data. Ownership-wise, the Bus is designed to own the consumers
//! but not to own the producers. It's essentially treated as an open well
//! that you throw data into and hope it reaches the right location.
use crate::consumer::Consumer;
use std::io;

const IO_BUFSIZE: usize = 10;

pub struct Bus {
    buffer: [u8; IO_BUFSIZE],
    len: usize,
    consumers: Vec<Consumer>,
}

impl Bus {
    pub fn new(consumers: Vec<Consumer>) -> Self {
        Self {
            buffer: [0; IO_BUFSIZE],
            len: 0,
            consumers,
        }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.len == 0 {
            return Ok(());
        }

        // Execute all callbacks on the current buffer
        for consumer in &mut self.consumers {
            consumer.write(&self.buffer[..self.len])?;
        }

        // Reset the buffer after flushing
        self.len = 0;
        Ok(())
    }

    pub fn consume(&mut self, data: &[u8]) -> io::Result<()> {
        let num_bytes = data.len();
        let mut bytes_left = num_bytes;

        // if self.read_buffer.len() == self.write_buffer.len() this loop is bounded to
        // at most 2 iterations.
        while bytes_left > 0 {
            // Ensure that we don't write beyond the buffer's capacity
            let available_space = IO_BUFSIZE - self.len;
            let bytes_to_consume = std::cmp::min(available_space, bytes_left);

            // slice out the part of the log we're going to use
            let start_idx = num_bytes - bytes_left;
            let slice = &data[start_idx..start_idx + bytes_to_consume];

            // Copy the data into the buffer
            self.buffer[self.len..self.len + bytes_to_consume].copy_from_slice(slice);

            // Update the buffer length
            self.len += bytes_to_consume;

            // If the buffer is full, flush it automatically
            if self.len == IO_BUFSIZE {
                self.flush()?;
            }

            // update the log amount left
            bytes_left -= bytes_to_consume;
        }
        Ok(())
    }
}

// impl Drop for Bus {
//     fn drop(&mut self) {
//         // Ensure the buffer is flushed when the struct is dropped
//         if self.len > 0 {
//             match self.flush() {
//                 Ok(_) => (),
//                 Err(e) => {
//                     eprintln!("Failed to flush buffer: {}", e);
//                 }
//             }
//         }
//     }
// }

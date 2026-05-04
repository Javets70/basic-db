// log.rs
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

// DEFAULTS TO
// BATCH SIZE 10
pub struct LogWriter {
    writer: BufWriter<File>,
    batch_count: usize,
    batch_size: usize,
}

impl LogWriter {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new().append(true).create(true).open(path)?;
        Ok(LogWriter {
            writer: BufWriter::new(file),
            batch_count: 0,
            batch_size: 10,
        })
    }

    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
    }

    pub fn write_entry(&mut self, key: &[u8], value: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(&(key.len() as u32).to_be_bytes())?;
        self.writer.write_all(&(value.len() as u32).to_be_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(value)?;

        self.batch_count += 1;
        self.maybe_sync()?;

        Ok(())
    }

    pub fn maybe_sync(&mut self) -> std::io::Result<()> {
        if self.batch_count >= self.batch_size && self.batch_size > 0 {
            self.batch_count = 0;
            self.flush_and_sync()?;
        }
        Ok(())
    }

    pub fn flush_and_sync(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }
}

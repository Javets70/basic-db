// log.rs
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

pub struct LogWriter {
    writer: BufWriter<File>,
}

impl LogWriter {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new().append(true).create(true).open(path)?;
        Ok(LogWriter {
            writer: BufWriter::new(file),
        })
    }

    pub fn write_entry(&mut self, key: &[u8], value: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(&(key.len() as u32).to_be_bytes())?;
        self.writer.write_all(&(value.len() as u32).to_be_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(value)?;

        Ok(())
    }

    pub fn flush_and_sync(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }
}

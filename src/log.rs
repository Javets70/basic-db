// log.rs
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, Write};

// DEFAULTS TO
// BATCH SIZE 10
pub struct LogWriter {
    writer: BufWriter<File>,
    batch_count: usize,
    batch_size: usize,
    current_offset: u64,
}

// NOW WE ARE STORING VALUE_OFFSET AND VALUE_LENGTH
// INSTEAD OF THE ACTUAL STRING TYPE VALUE IN THE LOG
// HashMap<Key bytes , (value offset , value length)>

impl LogWriter {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new().append(true).create(true).open(path)?;
        let current_offset = file.metadata()?.len();
        Ok(LogWriter {
            writer: BufWriter::new(file),
            batch_count: 0,
            batch_size: 10,
            current_offset,
        })
    }

    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
    }

    pub fn write_entry(&mut self, key: &[u8], value: &[u8]) -> std::io::Result<(u64, usize)> {
        // 4 BYTES FOR KEY LENGTH (u32)
        // 4 BYTES FOR 4 VALUE LENGTH (u32)
        // REST FOR ACTUAL KEY (SIZE COMPUTED AT EXECUTION)
        let value_offset = self.current_offset + 4 + 4 + (key.len() as u64);
        self.writer.write_all(&(key.len() as u32).to_be_bytes())?;
        self.writer.write_all(&(value.len() as u32).to_be_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(value)?;

        // VALUE OFFSET ALREADY ACCOUNTS FOR
        // ALL THE BYTES EXCEPT ACTUAL VALUE
        self.current_offset += value_offset + (value.len() as u64);

        self.batch_count += 1;
        self.maybe_sync()?;

        Ok((value_offset, value.len()))
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

// log.rs
use crate::get_log_path;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, Write};

// DEFAULTS TO
// BATCH SIZE 10
//
pub struct LogWriter {
    writer: BufWriter<File>,
    batch_count: usize,
    batch_size: usize,
    current_offset: u64,
    // MAX LENGTH OF A LOG FILE IN BYTES
    max_segment_size: u64,
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
            max_segment_size: 50,
        })
    }

    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
    }

    pub fn set_segment_size(&mut self, value: u64) {
        self.max_segment_size = value;
    }

    pub fn update_writer(&mut self, current_segment_number: &u32) -> std::io::Result<()> {
        let new_log_path = get_log_path(&(current_segment_number + 1u32));
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(new_log_path)?;
        let current_offset = file.metadata()?.len();
        self.writer = BufWriter::new(file);
        self.current_offset = current_offset;

        Ok(())
    }

    pub fn write_entry(
        &mut self,
        key: &[u8],
        value: &[u8],
        current_segment_number: &u32,
    ) -> std::io::Result<(u64, usize, bool)> {
        // 4 BYTES FOR KEY LENGTH (u32)
        // 4 BYTES FOR 4 VALUE LENGTH (u32)
        // ACTUAL KEY LENGTH (SIZE COMPUTED AT EXECUTION)
        // ACTUAL VALUE LENGTH (SIZE COMPUTED AT EXECUTION)
        let data_length = 4 + 4 + (key.len() as u64) + (value.len() as u64);

        // CURRENT STRATEGY FOR MAX_SEGMENT_SIZE IS
        // WE CHECK IF THE CURRENT WRITE WILL EXCEED THE
        // MAX_SEGMENT_SIZE , IF IT DOES THEN WE INCREMENT
        // SEGMENT_ID IN INDEX AND MOVE ONTO NEXT FILE

        let mut writer_updated = false;
        if data_length + self.current_offset > self.max_segment_size {
            // flush old writer
            self.flush_and_sync()?;
            writer_updated = true;
            self.update_writer(current_segment_number)?;
        }

        // 4 BYTES FOR KEY LENGTH (u32)
        // 4 BYTES FOR 4 VALUE LENGTH (u32)
        // REST FOR ACTUAL LENGTH (SIZE COMPUTED AT EXECUTION)
        let value_offset = self.current_offset + 4 + 4 + (key.len() as u64);
        self.writer.write_all(&(key.len() as u32).to_be_bytes())?;
        self.writer.write_all(&(value.len() as u32).to_be_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(value)?;

        self.current_offset += data_length;

        self.batch_count += 1;
        self.maybe_sync()?;

        Ok((value_offset, value.len(), writer_updated))
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

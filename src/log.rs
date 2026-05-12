// log.rs
use crate::get_log_path;
use crate::BASE_LOG_NAME;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

// DEFAULTS TO
// BATCH SIZE 10
//
pub struct LogWriter {
    writer: BufWriter<File>,
    batch_count: usize,
    batch_size: usize,
    current_offset: u64,
    max_segment_size: u64, // MAX SIZE OF LOG FILE IN BYTES LENGTH
    segment_number: u32,
}

// NOW WE ARE STORING VALUE_OFFSET AND VALUE_LENGTH
// INSTEAD OF THE ACTUAL STRING TYPE VALUE IN THE LOG
// HashMap<Key bytes , (value offset , value length)>

impl LogWriter {
    pub fn new() -> std::io::Result<Self> {
        let segment_number = LogWriter::get_highest_segment_number();
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(get_log_path(&segment_number))?;
        let current_offset = file.metadata()?.len();

        Ok(LogWriter {
            writer: BufWriter::new(file),
            batch_count: 0,
            batch_size: 10,
            current_offset,
            max_segment_size: 50,
            segment_number,
        })
    }

    pub fn get_segment_number(&self) -> u32 {
        self.segment_number
    }

    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
    }

    pub fn set_segment_size(&mut self, value: u64) {
        self.max_segment_size = value;
    }

    // SCANS THE CURRENT DIRECTORY FOR LOGS
    // AND GETS THE HIGHEST SEGMENT NUMBER
    fn get_highest_segment_number() -> u32 {
        let current_dir = match std::fs::read_dir(".") {
            Ok(dir) => dir,
            Err(_) => return 0,
        };

        let mut max_number = 0;

        for entry in current_dir.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with(BASE_LOG_NAME) && filename.ends_with(".log") {
                    let number_part = &filename[BASE_LOG_NAME.len()..filename.len() - 4];
                    if let Ok(number) = number_part.parse::<u32>() {
                        max_number = max_number.max(number);
                    }
                }
            }
        }
        max_number
    }

    /// INCREASE SEGMENT NUMBER AND
    /// UPDATE self.writer
    pub fn update_writer(&mut self) -> std::io::Result<()> {
        self.segment_number += 1;
        let new_log_path = get_log_path(&self.segment_number);
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(new_log_path)?;
        let current_offset = file.metadata()?.len();
        self.writer = BufWriter::new(file);
        self.current_offset = current_offset;

        Ok(())
    }

    pub fn write_entry(&mut self, key: &[u8], value: &[u8]) -> std::io::Result<(u64, usize)> {
        // 4 BYTES FOR KEY LENGTH (u32)
        // 4 BYTES FOR 4 VALUE LENGTH (u32)
        // ACTUAL KEY LENGTH (SIZE COMPUTED AT EXECUTION)
        // ACTUAL VALUE LENGTH (SIZE COMPUTED AT EXECUTION)
        let data_length = 4 + 4 + (key.len() as u64) + (value.len() as u64);

        // CURRENT STRATEGY FOR MAX_SEGMENT_SIZE IS
        // WE CHECK IF THE CURRENT WRITE WILL EXCEED THE
        // MAX_SEGMENT_SIZE , IF IT DOES THEN WE INCREMENT
        // SEGMENT_NUMBER AND MOVE ONTO NEXT FILE

        if data_length + self.current_offset > self.max_segment_size {
            // flush old writer
            self.flush_and_sync()?;
            self.update_writer()?;
        }

        // 4 BYTES FOR u32 (KEY LENTGH)
        // 4 BYTES FOR u32 (VALUE LENGTH)
        // REST FOR ACTUAL KEY BYTES LENGTH
        let value_offset = self.current_offset + 4 + 4 + (key.len() as u64);
        self.writer.write_all(&(key.len() as u32).to_be_bytes())?;
        self.writer.write_all(&(value.len() as u32).to_be_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(value)?;

        self.current_offset += data_length;

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

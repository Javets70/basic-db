// main.rs
use crate::log::LogWriter;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{stdin, BufWriter, ErrorKind, Read, Seek, Write};
use std::os::windows::fs::FileExt;

pub mod log;

struct Index(
    // HashMap<KEY BYTES ,
    // (VALUE_OFFSET IN CURRENT SEGMENT , VALUE SIZE IN CURRENT SEGMENT , SEGMENT IN WHICH VALUE IS PRESENT)>
    HashMap<Vec<u8>, (u64, usize, u32)>,
);

impl Index {
    pub fn new() -> Self {
        Index(HashMap::new())
    }
    pub fn insert(
        &mut self,
        key: Vec<u8>,
        value_offset: u64,
        value_len: usize,
        segment_number: u32,
    ) {
        self.0
            .insert(key, (value_offset, value_len, segment_number));
    }
    pub fn get(&self, key: &[u8]) -> Option<&(u64, usize, u32)> {
        self.0.get(key)
    }
    pub fn build(&mut self, segment_number: &u32) -> std::io::Result<()> {
        println!("SEGMET NUMBER:{segment_number}");
        for seg_id in 0..=*segment_number {
            let log_path = get_log_path(&seg_id);
            let mut file = match OpenOptions::new().read(true).open(log_path) {
                Ok(f) => f,
                Err(e) if e.kind() == ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            loop {
                let key_len = match read_u32(&mut file) {
                    Ok(n) => n as usize,
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        // eprintln!("Incomplete or partial record entry at EOF; discarding");
                        break;
                    }
                    Err(e) => return Err(e),
                };
                let val_len = match read_u32(&mut file) {
                    Ok(n) => n as usize,
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        eprintln!("Incomplete or partial record entry in log; discarding");
                        break;
                    }
                    Err(e) => return Err(e),
                };
                let key = match read_bytes(&mut file, key_len)? {
                    Some(data) => data,
                    None => {
                        eprintln!("Incomplete or partial record entry in log; discarding");
                        break;
                    }
                };

                let value_offset = file.stream_position()?;
                // moves the cursor to end of value bytes so next read doesnt
                // start form the middle of value bytes
                file.seek(std::io::SeekFrom::Current(val_len as i64))?;
                self.insert(key, value_offset, val_len, seg_id);
            }
        }
        println!("{:?} INDEX FILLED MAP", self.0);
        Ok(())
    }
}

pub const BASE_LOG_NAME: &str = "basicdb";

fn main() -> std::io::Result<()> {
    let mut index = Index::new();

    let mut log_writer = LogWriter::new()?;
    log_writer.set_batch_size(1);

    index.build(&log_writer.get_segment_number())?;

    // LOOPS OVER STRING INPUT
    // IF INPUT IS "QUIT" THEN BREAKS
    // OUT OF LOOP
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let mut parts = input.splitn(3, ' ');
        match parts.next() {
            Some("SET") => {
                let key = parts.next().unwrap_or_default().to_string();
                let value = parts.next().unwrap_or_default().to_string();

                let (value_offset, value_len) =
                    log_writer.write_entry(&key.clone().into_bytes(), &value.into_bytes())?;

                index.insert(
                    key.clone().into_bytes(),
                    value_offset,
                    value_len,
                    log_writer.get_segment_number(),
                );
                log_writer.maybe_sync()?;
            }
            Some("GET") => {
                let key = parts.next().unwrap_or_default().to_string();
                match index.get(&key.clone().into_bytes()) {
                    Some((val_offset, val_len, segment_number)) => {
                        let mut _file = OpenOptions::new()
                            .read(true)
                            .open(get_log_path(segment_number))?;
                        let mut buf = vec![0u8; *val_len];
                        _file.seek(std::io::SeekFrom::Start(*val_offset))?;
                        _file.read_exact(&mut buf)?;

                        println!("{:?}", buf);
                        println!("{:?}", String::from_utf8_lossy(&buf));
                    }
                    None => eprintln!("Value for key '{}' not found", key),
                }
            }

            Some("COMPACT") => {
                log_writer.flush_and_sync()?;
                let compact_file = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(get_compact_log_path())?;

                {
                    let mut compact_file_writer = BufWriter::new(compact_file);
                    for (key, val) in index.0.iter() {
                        let (val_offset, val_len, segment_num) = val;

                        let mut segment_file = OpenOptions::new()
                            .read(true)
                            .open(get_log_path(segment_num))?;
                        let mut value_buf = vec![0u8; *val_len];
                        segment_file.seek(std::io::SeekFrom::Start(*val_offset))?;
                        segment_file.read_exact(&mut value_buf)?;

                        compact_file_writer.write_all(&(key.len() as u32).to_be_bytes())?;
                        compact_file_writer.write_all(&(*val_len as u32).to_be_bytes())?;
                        compact_file_writer.write_all(key)?;
                        compact_file_writer.write_all(&value_buf)?;
                    }
                    compact_file_writer.flush()?;
                    compact_file_writer.get_ref().sync_all()?;
                }
                // RENAME COMPACT LOG TO basicdb0.log
                std::fs::rename(get_compact_log_path(), get_log_path(&0))
                    .inspect_err(|e| eprintln!("Unable to rename compact log \nCause {e}"))?;

                let current_segment_number = log_writer.get_segment_number();
                // REMOVE OLD LOGS
                for seg_num in 1..=current_segment_number {
                    let log_path = get_log_path(&seg_num);
                    std::fs::remove_file(&log_path).inspect_err(|e| {
                        eprintln!("Failed to delete file {log_path} \nCause:{e}")
                    })?;
                }

                // CLEAR AND REBUILD INDEX
                index.0.clear();
                index.build(&current_segment_number)?;

                //NEW LOG WRITER
                log_writer = LogWriter::new()?;
            }

            Some("QUIT") => {
                log_writer.flush_and_sync()?;
                break;
            }
            _ => eprintln!("Unknown Command. To exit type QUIT and enter."),
        }
    }
    Ok(())
}

fn read_u32(reader: &mut impl Read) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;

    Ok(u32::from_be_bytes(buf))
}

fn read_bytes(reader: &mut impl Read, len: usize) -> std::io::Result<Option<Vec<u8>>> {
    let mut buf = vec![0u8; len];
    match reader.read_exact(&mut buf) {
        Ok(_) => Ok(Some(buf)),
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_log_path(segment_number: &u32) -> String {
    format!("{}{}.log", BASE_LOG_NAME, segment_number)
}

pub fn get_compact_log_path() -> String {
    format!("{}_compact.log", BASE_LOG_NAME)
}

// main.rs
use crate::log::LogWriter;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{stdin, ErrorKind, Read, Seek};
use std::os::windows::fs::FileExt;

pub mod log;

struct Index {
    // HashMap<KEY BYTES ,
    // (VALUE_OFFSET IN CURRENT SEGMENT , VALUE SIZE IN CURRENT SEGMENT , SEGMENT IN WHICH VALUE IS PRESENT)>
    map: HashMap<Vec<u8>, (u64, usize, u32)>,
    segment_id: u32,
}

impl Index {
    pub fn new() -> Self {
        Index {
            map: HashMap::new(),
            segment_id: 0,
        }
    }
    pub fn insert(
        &mut self,
        key: Vec<u8>,
        value_offset: u64,
        value_len: usize,
        segment_number: u32,
    ) {
        self.map
            .insert(key, (value_offset, value_len, segment_number));
    }
    pub fn get(&self, key: &[u8]) -> Option<&(u64, usize, u32)> {
        self.map.get(key)
    }

    pub fn set_segment_id(&mut self) {
        self.segment_id = self.get_highest_segment_id();
    }
    pub fn get_segment_id(&self) -> u32 {
        self.segment_id
    }
    pub fn increment_segment_id(&mut self) {
        self.segment_id += 1
    }

    fn get_highest_segment_id(&self) -> u32 {
        let current_dir = match fs::read_dir(".") {
            Ok(dir) => dir,
            Err(_) => return 0,
        };

        let mut max_number = 0;

        for entry in current_dir {
            if let Ok(entry) = entry {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.starts_with(BASE_LOG_NAME) && filename.ends_with(".log") {
                        let number_part = &filename[BASE_LOG_NAME.len()..filename.len() - 4];
                        if let Ok(number) = number_part.parse::<u32>() {
                            max_number = max_number.max(number);
                        }
                    }
                }
            }
        }
        max_number
    }
}

pub const BASE_LOG_NAME: &str = "basicdb";

fn main() -> std::io::Result<()> {
    let mut index = Index::new();
    index.set_segment_id();

    for seg_id in 0..=index.get_segment_id() {
        let log_path = get_log_path(&seg_id);
        let mut file = match OpenOptions::new().read(true).open(log_path) {
            Ok(f) => f,
            Err(e) if e.kind() == ErrorKind::NotFound => break,
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
            index.insert(key, value_offset, val_len, seg_id);
        }
    }
    println!("{:?} INDEX FILLED MAP", index.map);

    let log_path = get_log_path(&index.segment_id);
    let mut log_writer = LogWriter::new(&log_path)?;
    log_writer.set_batch_size(1);

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

                let (value_offset, value_len, writer_updated) = log_writer.write_entry(
                    &key.clone().into_bytes(),
                    &value.into_bytes(),
                    &index.get_segment_id(),
                )?;

                if writer_updated {
                    index.increment_segment_id();
                }

                index.insert(
                    key.clone().into_bytes(),
                    value_offset,
                    value_len,
                    index.get_segment_id(),
                );
                log_writer.maybe_sync()?;
            }
            Some("GET") => {
                let key = parts.next().unwrap_or_default().to_string();
                match index.get(&key.clone().into_bytes()) {
                    Some((val_offset, val_len, segment_number)) => {
                        let _file = OpenOptions::new()
                            .read(true)
                            .open(get_log_path(segment_number))?;
                        let mut buf = vec![0u8; *val_len];
                        _file.seek_read(&mut buf, *val_offset)?;
                        println!("{:?}", buf);
                        println!("{:?}", String::from_utf8_lossy(&buf));
                    }
                    None => eprintln!("Value for key '{}' not found", key),
                }
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

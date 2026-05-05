// main.rs
use crate::log::LogWriter;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{stdin, ErrorKind, Read, Seek};
use std::os::windows::fs::FileExt;

pub mod log;
// pub mod store;

// HashMap<Key bytes , (value offset , value length)>
struct Index(HashMap<Vec<u8>, (u64, usize)>);

impl Index {
    pub fn new() -> Self {
        Index(HashMap::new())
    }
    pub fn insert(&mut self, key: Vec<u8>, value_offset: u64, value_len: usize) {
        self.0.insert(key, (value_offset, value_len));
    }
    pub fn get(&self, key: &[u8]) -> Option<&(u64, usize)> {
        self.0.get(key)
    }
}

fn main() -> std::io::Result<()> {
    let log_path = "kestrel.log";
    let mut index = Index::new();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(log_path)?;
    loop {
        let key_len = match read_u32(&mut file) {
            Ok(n) => n as usize,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                eprintln!("Incomplete or partial record entry at EOF; discarding");
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
        index.insert(key, value_offset, val_len);
    }

    let mut log_writer = LogWriter::new(log_path)?;
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

                let (value_offset, value_len) =
                    log_writer.write_entry(&key.clone().into_bytes(), &value.into_bytes())?;

                index.insert(key.clone().into_bytes(), value_offset, value_len);
                log_writer.maybe_sync()?;
            }
            Some("GET") => {
                let key = parts.next().unwrap_or_default().to_string();
                match index.get(&key.clone().into_bytes()) {
                    Some((val_offset, val_len)) => {
                        let _file = OpenOptions::new().read(true).open(log_path)?;
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

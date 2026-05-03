// main.rs
use crate::log::LogWriter;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, stdin, BufRead, BufReader, ErrorKind, Read, Write};

pub mod log;

fn main() -> std::io::Result<()> {
    let log_path = "kestrel.log";
    let mut store = HashMap::new();

    OpenOptions::new().create(true).write(true).open(log_path)?;
    let mut file = OpenOptions::new().read(true).open(log_path)?;
    loop {
        let key_len = match read_u32(&mut file) {
            Ok(n) => n as usize,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                eprintln!("Incomplete or partial record entry at EOF; discarding");
                break;
            }
            Err(e) => return Err(e),
        };
        // let val_len = read_u32(&mut file)? as usize;
        let val_len = match read_u32(&mut file) {
            Ok(n) => n as usize,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                eprintln!("Incomplete or partial record entry at EOF; discarding");
                break;
            }
            Err(e) => return Err(e),
        };

        let mut key = vec![0u8; key_len];
        file.read_exact(&mut key)?;
        let mut value = vec![0u8; val_len];
        file.read_exact(&mut value)?;

        let key_str = String::from_utf8_lossy(&key).into_owned();
        let val_str = String::from_utf8_lossy(&value).into_owned();
        store.insert(key_str, val_str);
    }

    let mut log_writer = LogWriter::new(log_path)?;
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

                // update store
                store.insert(key.clone(), value.clone());

                // let mut file = OpenOptions::new()
                //     .append(true)
                //     .create(true)
                //     .open(log_path)?;
                // write_entry(&mut file, &key.into_bytes(), &value.into_bytes())?;
                // file.sync_all()?;
                log_writer.write_entry(&key.into_bytes(), &value.into_bytes())?;
                log_writer.flush_and_sync()?;
            }
            Some("GET") => {
                let key = parts.next().unwrap_or_default().to_string();
                match store.get(&key) {
                    Some(val) => println!("{}", val),
                    None => eprintln!("Value for key '{}' not found", key),
                }
            }

            Some("QUIT") => break,
            _ => eprintln!("Unknown Command. To exit type QUIT and enter."),
        }
    }
    Ok(())
}

// fn write_entry(file: &mut std::fs::File, key: &[u8], value: &[u8]) -> std::io::Result<()> {
//     file.write_all(&(key.len() as u32).to_be_bytes())?;
//     file.write_all(&(value.len() as u32).to_be_bytes())?;
//     file.write_all(key)?;
//     file.write_all(value)?;

//     Ok(())
// }

fn read_u32(reader: &mut impl Read) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;

    Ok(u32::from_be_bytes(buf))
}

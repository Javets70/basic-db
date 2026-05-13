use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: client SET key value | GET key | COMPACT");
        return;
    }
    let cmd = &args[1];
    let key = &args[2];
    let mut request = Vec::new();

    match cmd.as_str() {
        "SET" => {
            if args.len() != 4 {
                eprintln!("SET needs key and value");
                return;
            }
            let value = &args[3];
            request.push(0x02u8);
            request.extend_from_slice(&(key.len() as u32).to_be_bytes());
            request.extend_from_slice(key.as_bytes());
            request.extend_from_slice(&(value.len() as u32).to_be_bytes());
            request.extend_from_slice(value.as_bytes());
        }
        "GET" => {
            request.push(0x01u8);
            request.extend_from_slice(&(key.len() as u32).to_be_bytes());
            request.extend_from_slice(key.as_bytes());
            request.extend_from_slice(&0u32.to_be_bytes()); // val_len = 0
        }
        "COMPACT" => {
            request.push(0x03u8);
            request.extend_from_slice(&0u32.to_be_bytes()); // key_len=0
            request.extend_from_slice(&0u32.to_be_bytes()); // val_len=0
        }
        _ => {
            eprintln!("Unknown command");
            return;
        }
    }

    let mut stream = TcpStream::connect("127.0.0.1:8888").expect("connect failed");
    stream.write_all(&request).expect("write failed");
    stream.shutdown(std::net::Shutdown::Write).unwrap();

    // Read response
    let mut status = [0u8; 1];
    stream.read_exact(&mut status).expect("read status");
    let mut val_len_buf = [0u8; 4];
    stream.read_exact(&mut val_len_buf).expect("read val_len");
    let val_len = u32::from_be_bytes(val_len_buf) as usize;
    let mut value = vec![0u8; val_len];
    stream.read_exact(&mut value).expect("read value");

    match status[0] {
        0x00 => println!("OK {}", String::from_utf8_lossy(&value)),
        0x01 => println!("NOT FOUND"),
        0xFF => println!("ERROR"),
        _ => println!("UNKNOWN STATUS"),
    }
}

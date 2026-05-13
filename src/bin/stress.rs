use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Instant;

fn build_request(cmd: &str, key: &str, value: Option<&str>) -> Vec<u8> {
    let mut request = Vec::new();
    match cmd {
        "SET" => {
            request.push(0x02u8);
            request.extend_from_slice(&(key.len() as u32).to_be_bytes());
            request.extend_from_slice(key.as_bytes());
            request.extend_from_slice(&(value.expect("VALUE EXPECTED").len() as u32).to_be_bytes());
            request.extend_from_slice(value.expect("VALUE EXPECTED").as_bytes());
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
        }
    }
    request
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: stress <concurrency> <operations> <cmd> [key_prefix] [value_prefix]");
        eprintln!("Example: stress 10 100 SET mykey myvalue");
        return;
    }
    let concurrency: usize = args[1].parse().unwrap();
    let total_ops: usize = args[2].parse().unwrap();
    let cmd = args[3].clone();
    let key_prefix = if args.len() > 4 {
        args[4].clone()
    } else {
        "k".to_string()
    };
    let val_prefix = if args.len() > 5 {
        args[5].clone()
    } else {
        "v".to_string()
    };

    let start = Instant::now();
    let mut handles = vec![];
    let ops_per_thread = total_ops / concurrency;

    for t in 0..concurrency {
        let cmd = cmd.clone();
        let key_prefix = key_prefix.clone();
        let val_prefix = val_prefix.clone();
        handles.push(thread::spawn(move || {
            let mut successes = 0;
            let mut failures = 0;
            for i in 0..ops_per_thread {
                let key = format!("{}_{}_{}", key_prefix, t, i);
                let value = format!("{}_{}_{}", val_prefix, t, i);
                let request = build_request(&cmd, &key, Some(&value)); // adjust if GET
                match TcpStream::connect("127.0.0.1:8888") {
                    Ok(mut stream) => {
                        if stream.write_all(&request).is_err() {
                            failures += 1;
                            continue;
                        }
                        stream.shutdown(std::net::Shutdown::Write).unwrap();
                        if cmd == "SET" {
                            // Read response: 5 bytes (0x00 + 4 bytes of 0)
                            let mut buf = [0u8; 5];
                            if stream.read_exact(&mut buf).is_err() {
                                failures += 1;
                                continue;
                            }
                            if buf[0] == 0x00 {
                                successes += 1;
                            } else {
                                failures += 1;
                            }
                        }
                        // For GET you'd read more; but simple SET stress is enough.
                    }
                    Err(_) => {
                        failures += 1;
                    }
                }
            }
            (successes, failures)
        }));
    }

    let (mut total_success, mut total_fail) = (0, 0);
    for h in handles {
        let (s, f) = h.join().unwrap();
        total_success += s;
        total_fail += f;
    }
    let elapsed = start.elapsed();
    println!("=== RESULTS ===");
    println!("Total time: {:.2?}", elapsed);
    println!("Successful ops: {}", total_success);
    println!("Failed ops: {}", total_fail);
    if elapsed.as_secs_f64() > 0.0 {
        println!(
            "Throughput: {:.2} ops/sec",
            total_success as f64 / elapsed.as_secs_f64()
        );
    }
}

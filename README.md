# basic-db
A simple write ahead log written in rust

- Binary write‑ahead log with length‑prefixed records
- Crash‑safe recovery that discards partial records
- In‑memory index mapping key → (segment, offset, length) — values live on disk
- Group commit (configurable batch size) balancing durability and throughput
- Log segmentation (rotation when file exceeds threshold)
- Compaction (garbage collection of old values into a fresh log)
- Synchronous TCP server with a binary protocol (command byte + length‑prefixed key/value)
- Multi‑threaded stress‑test client revealing performance limits and recovery gaps

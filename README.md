# basic-db
A simple write ahead log written in rust

1. A basic crash recovery system (discards invalid logs when rebuilding index)
2. Listens over TCP for commands
3. Segments log files into smaller pieces
4. Compacts the logs , rebuilds the logs from index so all the keys are updated from a single read.

## Level 2 (Async)

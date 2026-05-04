use std::{collections::HashMap, fs::OpenOptions, io::Read};

struct Store {
    // HashMap<Key as bytes , (value offest in log , value length)>
    index: HashMap<Vec<u8>, (u64, usize)>,
    reader: std::fs::File,
}

impl Store {
    pub fn new(log_path: &str) -> std::io::Result<Self> {
        let reader = OpenOptions::new().read(true).open(log_path)?;
        Ok(Store {
            index: HashMap::new(),
            reader,
        })
    }

    pub fn get(&mut self, key: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
        if let Some((val_offset, val_len)) = self.index.get(key) {
            use std::io::Seek;
            self.reader.seek(std::io::SeekFrom::Start(*val_offset))?;
            let mut buf = vec![0u8; *val_len];
            self.reader.read_exact(&mut buf)?;
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }
}

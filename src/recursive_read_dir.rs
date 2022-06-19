use std::fs::{read_dir, DirEntry, ReadDir};
use std::path::Path;

pub struct RecursiveReadDir {
    recursive: bool,
    entries: ReadDir,
    dir_entry: Option<Box<RecursiveReadDir>>,
}

impl Iterator for RecursiveReadDir {
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut inner) = self.dir_entry {
            if let Some(entry) = inner.next() {
                return Some(entry);
            }
            self.dir_entry = None;
        }
        loop {
            if let Some(entry) = self.entries.next() {
                if let Ok(entry) = entry {
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_file() {
                            return Some(entry);
                        } else if self.recursive && meta.is_dir() {
                            if let Ok(entries) = read_dir(entry.path()) {
                                let inner = RecursiveReadDir {
                                    recursive: self.recursive,
                                    entries,
                                    dir_entry: None,
                                };
                                self.dir_entry = Some(Box::new(inner));
                                return self.dir_entry.as_mut().unwrap().next();
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }
        None
    }
}

pub fn read_dir_recursive<P: AsRef<Path>>(
    path: P,
    recursive: bool,
) -> std::io::Result<RecursiveReadDir> {
    Ok(RecursiveReadDir {
        recursive,
        entries: read_dir(path)?,
        dir_entry: None,
    })
}

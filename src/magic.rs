use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use lazy_static::lazy_static;

const FILE_MAGIC: [(&'static [u8], FileType); 2] = [
    (&[0x1a, 0x45, 0xdf, 0xa3], FileType::MKV),
    (
        &[0x66, 0x74, 0x79, 0x70, 0x69, 0x73, 0x6f, 0x6d],
        FileType::MP4,
    ),
];
lazy_static! {
    static ref SIGNATURE_SIZE: usize = FILE_MAGIC
        .iter()
        .fold(0, |acc, (sig, _)| usize::max(sig.len(), acc));
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FileType {
    Unknown,
    MKV,
    MP4,
}

impl FileType {
    pub fn parse_file<T: Read>(mut file: T) -> std::io::Result<Self> {
        let mut buf = vec![0; *SIGNATURE_SIZE];
        file.read(&mut buf)?;

        for (magic, file_type) in FILE_MAGIC {
            if buf.starts_with(magic) {
                return Ok(file_type);
            }
        }

        Ok(FileType::Unknown)
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).open(path)?;
        FileType::parse_file(file)
    }
}

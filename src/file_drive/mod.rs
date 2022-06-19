use std::fs::metadata;
use std::path::Path;

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod inner;

use inner::same_drive;

pub fn files_on_same_drive<P: AsRef<Path>>(file_a: P, file_b: P) -> std::io::Result<bool> {
    let meta_a = metadata(file_a)?;
    let meta_b = metadata(file_b)?;
    Ok(same_drive(meta_a, meta_b))
}

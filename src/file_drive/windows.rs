use std::os::windows::fs::MetadataExt;

pub fn same_drive<T: MetadataExt>(a: T, b: T) -> bool {
    a.volume_serial_number().is_some()
        && b.volume_serial_number.is_some()
        && a.volume_serial_number() == b.volume_serial_number()
}

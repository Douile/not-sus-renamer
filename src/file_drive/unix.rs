use std::os::unix::fs::MetadataExt;

pub fn same_drive<T: MetadataExt>(a: T, b: T) -> bool {
    a.dev() == b.dev()
}

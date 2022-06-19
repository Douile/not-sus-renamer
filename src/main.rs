#![cfg_attr(windows, feature(windows_by_handle))]

use std::env::current_dir;
use std::fs::{metadata, OpenOptions};
use std::io::ErrorKind;
use std::path::PathBuf;

pub mod file_drive;
pub mod magic;
mod recursive_read_dir;
pub mod types;
use crate::file_drive::files_on_same_drive;
use crate::magic::FileType;
use crate::recursive_read_dir::read_dir_recursive;
use crate::types::{GenericResult, Video};

const DELETE_OLD: bool = false;

struct Options {
    from_directory: PathBuf,
    to_directory: PathBuf,
    delete_old: bool,
    dry_run: bool,
    dont_recurse: bool,
}

fn parse_options() -> std::io::Result<Options> {
    let mut args = std::env::args();
    args.next().expect("arg0");
    let cwd = current_dir()?;

    let mut delete_old = false;
    let mut dry_run = false;
    let mut dont_recurse = false;

    let mut args = args.filter(|arg| match arg.strip_prefix('-') {
        Some(argument) => {
            match argument {
                "-dont-recurse" | "n" => dont_recurse = true,
                "-delete" | "d" => delete_old = true,
                "-dry" => dry_run = true,
                _ => unreachable!("Unknown option {:?}", argument),
            }
            false
        }
        None => true,
    });

    let from_directory = args.next().map(PathBuf::from).unwrap_or(cwd.clone());
    let to_directory = args.next().map(PathBuf::from).unwrap_or(cwd);

    for _ in args {}

    Ok(Options {
        from_directory,
        to_directory,
        delete_old,
        dry_run,
        dont_recurse,
    })
}

fn main() -> GenericResult<()> {
    let Options {
        from_directory,
        to_directory,
        delete_old,
        dry_run,
        dont_recurse,
    } = parse_options()?;

    let same_drive = files_on_same_drive(&from_directory, &to_directory)?;

    eprintln!(
        "Moving videos from {:?} -> {:?}",
        from_directory, to_directory
    );
    eprintln!("  Same drive: {:?}", same_drive);
    eprintln!("  Delete old: {:?}", delete_old);
    eprintln!("  Dry run:    {:?}", dry_run);
    eprintln!("  Recursion:  {:?}", !dont_recurse);

    // TODO: Optimize parsing so only need to open file once
    let files: Vec<_> = read_dir_recursive(&from_directory, !dont_recurse)?
        .filter_map(|entry| match FileType::from_path(entry.path()) {
            Ok(video_type) if video_type != FileType::Unknown => {
                Some(Video::from_path(entry.path(), video_type).unwrap())
            }
            _ => None,
        })
        .collect();

    for file in files {
        let new_file_name = file.generate_file_name();
        let new_file_path = to_directory.clone().join(&new_file_name);
        println!("{:?} -> {:?}", file.path, new_file_path);
        if dry_run {
            continue;
        }

        // TODO: Write metadata
        // TODO: Convert mp4 to mkv
        eprintln!("{:?}", metadata(&new_file_path));
        match metadata(&new_file_path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Ok(_) => {
                eprintln!("Skipping {:?} as file already exists", new_file_name);
                continue;
            }
            _ => todo!(),
        }

        // Use OS builtin API if on same drive as instant
        if same_drive {
            if DELETE_OLD {
                std::fs::rename(&file.path, &new_file_path)?;
            } else {
                std::fs::copy(&file.path, &new_file_path)?;
            }
        } else {
            let mut old_file = OpenOptions::new().read(true).open(&file.path)?;
            let mut new_file = OpenOptions::new().create_new(true).open(&new_file_path)?;
            std::io::copy(&mut old_file, &mut new_file)?;
            // TODO: Add some kind of copy progress
            if DELETE_OLD {
                std::fs::remove_file(&file.path)?;
            }
        }
    }

    Ok(())
}

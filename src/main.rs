#![cfg_attr(windows, feature(windows_by_handle))]

use std::env::current_dir;
use std::fs::{metadata, OpenOptions};
use std::io::ErrorKind;
use std::path::PathBuf;

pub mod file_drive;
#[cfg(feature = "imdb")]
pub mod imdb;
pub mod magic;
mod recursive_read_dir;
pub mod types;

use crate::file_drive::files_on_same_drive;
use crate::magic::FileType;
use crate::recursive_read_dir::read_dir_recursive;
use crate::types::{GenericResult, Video};

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
    #[cfg(feature = "debug")]
    {
        simple_logger::SimpleLogger::new()
            .with_level(log::LevelFilter::max())
            .init()
            .unwrap();
    }

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

    #[cfg(feature = "imdb")]
    let mut searcher = {
        let cwd = std::env::current_dir()?;
        eprintln!("Opening IMDB index");
        let dataset_dir = cwd.join("datasets");
        let index =
            imdb::open_if_exists_or_create_index(dataset_dir.clone(), dataset_dir.join("index"))?;
        imdb::Searcher::new(index)
    };

    for mut file in files {
        let new_file_name = file.generate_file_name();
        let new_file_path = to_directory.clone().join(&new_file_name);
        println!("{:?} -> {:?}", file.path, new_file_path);

        #[cfg(feature = "imdb")]
        {
            if let Ok(result) = imdb::search_for_video(&mut searcher, &file.info) {
                file.update_from_imdb(&result)?;
            }
        }

        if dry_run {
            continue;
        }

        let mut is_copied = false;
        let mut is_metadata_written = false;

        // TODO: Convert mp4 to mkv
        match metadata(&new_file_path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Ok(_) => {
                eprintln!("Skipping {:?} as file already exists", new_file_name);
                is_copied = true;
            }
            _ => todo!(),
        }

        if !is_copied {
            // Use OS builtin API if on same drive as instant
            if same_drive && delete_old {
                std::fs::rename(&file.path, &new_file_path)?;
            } else {
                let mut old_file = OpenOptions::new().read(true).open(&file.path)?;
                let mut new_file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&new_file_path)?;
                if file.file_type == FileType::MKV {
                    file.insert_into_matroska(&mut old_file, &mut new_file)?;
                    is_metadata_written = true;
                } else {
                    std::io::copy(&mut old_file, &mut new_file)?;
                }
                // TODO: Add some kind of copy progress
                if delete_old {
                    std::fs::remove_file(&file.path)?;
                }
            }
        }

        if !is_metadata_written && file.file_type == FileType::MKV {
            // TODO: Write metadata
            eprintln!("Updating metadata");
            let mut old_file = OpenOptions::new().read(true).open(&new_file_path)?;
            let meta_path = new_file_path.with_extension("with_meta");
            let mut new_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&meta_path)?;

            file.insert_into_matroska(&mut old_file, &mut new_file)?;
            let backup_path = new_file_path.with_extension("mkv.bak");
            if !delete_old {
                std::fs::rename(&new_file_path, &backup_path)?;
            }
            std::fs::rename(&meta_path, &new_file_path)?;
        }
    }

    Ok(())
}

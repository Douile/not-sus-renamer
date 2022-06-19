use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::Duration;

use format_num::format_num;
use lazy_static::lazy_static;
use matroska::Matroska;
use regex::{Regex, RegexBuilder};

use crate::magic::FileType;

pub type GenericResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const STANDARD_RESOLUTIONS: [u64; 6] = [480, 720, 1080, 1440, 2160, 4320];

#[derive(Debug)]
pub struct Video {
    pub path: PathBuf,
    pub file_type: FileType,
    pub file_extension: String,
    pub info: VideoData,
}

#[derive(Debug)]
pub enum VideoData {
    Episode(Episode, Metadata),
    Movie(Entity, Metadata),
}

#[derive(Debug)]
pub struct Episode {
    pub episode: u32,
    pub season: u32,
    pub title: String,
    pub show: Entity,
}

#[derive(Debug)]
pub struct Entity {
    pub title: String,
    pub release_year: u32,
    pub imdb_id: String,
}

#[derive(Debug)]
pub struct Metadata {
    pub resolution: (u64, u64),
    pub length: Option<Duration>,
}

impl Metadata {
    pub fn from_matroska<P: AsRef<Path>>(path: P) -> GenericResult<Self> {
        let file = OpenOptions::new().read(true).open(path)?;
        let metadata = Matroska::open(file)?;

        let duration = metadata.info.duration;
        let video_track = metadata.video_tracks().next().ok_or("No video tracks")?;
        let video_track_settings = match &video_track.settings {
            matroska::Settings::Video(v) => Ok(v),
            _ => Err("No video track settings"),
        }?;

        Ok(Self {
            resolution: (
                video_track_settings
                    .display_width
                    .unwrap_or(video_track_settings.pixel_width),
                video_track_settings
                    .display_height
                    .unwrap_or(video_track_settings.pixel_height),
            ),
            length: duration,
        })
    }

    pub fn from_vertical_resolution(vertical_resolution: u64, length: Option<Duration>) -> Self {
        Self {
            resolution: (vertical_resolution / 9 * 16, vertical_resolution),
            length,
        }
    }

    pub fn get_resolution(&self) -> u64 {
        let best_resolution = u64::max(self.resolution.0 / 16 * 9, self.resolution.1);
        for i in 1..STANDARD_RESOLUTIONS.len() {
            let lower = STANDARD_RESOLUTIONS[i - 1];
            let higher = STANDARD_RESOLUTIONS[i];
            if best_resolution >= lower && best_resolution <= higher {
                if best_resolution - lower > higher - best_resolution {
                    return higher;
                } else {
                    return lower;
                }
            }
        }
        best_resolution
    }
}

lazy_static! {
    static ref SEASON: Regex = RegexBuilder::new(r"s(\d+)")
        .case_insensitive(true)
        .build()
        .unwrap();
    static ref EPISODE: Regex = RegexBuilder::new(r"e(\d+)")
        .case_insensitive(true)
        .build()
        .unwrap();
    static ref QUALITY: Regex = RegexBuilder::new(r"(\d{3,})p")
        .case_insensitive(true)
        .build()
        .unwrap();
}

impl Video {
    pub fn from_path(path: PathBuf, file_type: FileType) -> GenericResult<Self> {
        let file_name = path.file_name().ok_or("Not a file")?.to_string_lossy();
        let mut file_name_parts: Vec<&str> = file_name.split(&['.', ' ', '-'][..]).collect();
        let file_extension = file_name_parts
            .remove(file_name_parts.len() - 1)
            .to_string();

        let mut title_end = file_name_parts.len();
        let mut episode_title_end = title_end;
        let mut season = None;
        let mut episode = None;
        let mut quality = None;
        for i in 0..file_name_parts.len() {
            let part = file_name_parts[i];

            if let Some(captures) = SEASON.captures(part) {
                if let Ok(n) = u32::from_str_radix(captures.get(1).unwrap().as_str(), 10) {
                    season = Some(n);
                    title_end = usize::min(i, title_end);
                }
            }

            if let Some(captures) = EPISODE.captures(part) {
                if let Ok(n) = u32::from_str_radix(captures.get(1).unwrap().as_str(), 10) {
                    episode = Some(n);
                    title_end = usize::min(i, title_end);
                }
            }

            if let Some(captures) = QUALITY.captures(part) {
                if let Ok(n) = u64::from_str_radix(captures.get(1).unwrap().as_str(), 10) {
                    quality = Some(n);
                    episode_title_end = usize::min(i, title_end);
                }
            }
        }

        let title = file_name_parts[..title_end].join(" ");
        let episode_title = if episode_title_end - title_end > 1 {
            Some(file_name_parts[title_end + 1..episode_title_end].join(" "))
        } else {
            None
        };

        let metadata = if file_type == FileType::MKV {
            Metadata::from_matroska(&path)?
        } else {
            Metadata::from_vertical_resolution(quality.unwrap_or(0), None)
        };

        let info = if let Some(episode) = episode {
            VideoData::Episode(
                Episode {
                    episode,
                    season: season.unwrap_or(1),
                    title: episode_title.unwrap_or(String::new()),
                    show: Entity {
                        title,
                        release_year: 0,
                        imdb_id: String::new(),
                    },
                },
                metadata,
            )
        } else {
            VideoData::Movie(
                Entity {
                    title,
                    release_year: 0,
                    imdb_id: "".to_string(),
                },
                metadata,
            )
        };

        Ok(Self {
            file_extension,
            file_type,
            path,
            info,
        })
    }

    pub fn generate_file_name(&self) -> String {
        match &self.info {
            VideoData::Episode(episode, meta) => {
                format!(
                    "{}-S{}E{}-{}p.{}",
                    episode.show.title,
                    format_num!("02.0", episode.season),
                    format_num!("02.0", episode.episode),
                    meta.get_resolution(),
                    self.file_extension
                )
            }
            VideoData::Movie(movie, meta) => format!(
                "{}-{}p.{}",
                movie.title,
                meta.get_resolution(),
                self.file_extension
            ),
        }
    }
}

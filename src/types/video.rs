use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;

use format_num::format_num;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use webm_iterable::{
    matroska_spec::{Master, MatroskaSpec},
    WebmIterator, WebmWriter,
};

use crate::magic::FileType;

use super::Entity;
use super::Episode;
use super::GenericResult;
use super::Metadata;

#[derive(Debug, Clone)]
pub struct Video {
    pub path: PathBuf,
    pub file_type: FileType,
    pub file_extension: String,
    pub info: VideoData,
}

#[derive(Debug, Clone)]
pub enum VideoData {
    Episode(Episode, Metadata),
    Movie(Entity, Metadata),
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

const TITLE: &str = "TITLE";
const DATE_RELEASED: &str = "DATE_RELEASED";
const COMMENT: &str = "COMMENT";
const IMDB_ID: &str = "IMDB";
const EPISODE_NUMBER: &str = "EPISODE";
const SEASON_NUMBER: &str = "SEASON";

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
                    title_end = usize::min(i, title_end);
                    episode_title_end = usize::min(i, episode_title_end);
                }
            }
        }

        let title = file_name_parts[..title_end].join(" ");
        let episode_title = if usize::checked_sub(episode_title_end, title_end).unwrap_or(0) > 1 {
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
                    imdb_id: None,
                    series: Entity {
                        title,
                        release_year: 0,
                        imdb_id: None,
                    },
                },
                metadata,
            )
        } else {
            VideoData::Movie(
                Entity {
                    title,
                    release_year: 0,
                    imdb_id: None,
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
                    episode.series.title,
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

    #[cfg(feature = "imdb")]
    pub fn update_from_imdb(&mut self, entity: &crate::imdb::Results) -> GenericResult<()> {
        let mut res = Ok(());
        self.info = match (self.info.clone(), entity) {
            (VideoData::Movie(_, meta), crate::imdb::Results::Movie(entity)) => {
                VideoData::Movie(entity.into(), meta)
            }
            (
                VideoData::Episode(old_entity, meta),
                crate::imdb::Results::Episode(series, episode),
            ) => match (episode, series).try_into() {
                Ok(ep) => VideoData::Episode(ep, meta),
                Err(e) => {
                    res = Err(e.into());
                    VideoData::Episode(old_entity, meta)
                }
            },
            _ => unreachable!(),
        };
        res
    }

    pub fn insert_into_matroska<F: Read, T: Write>(
        &self,
        from: &mut F,
        to: &mut T,
    ) -> GenericResult<()> {
        // FIXME: Make more modular with less code repetition
        let reader = WebmIterator::new(from, &[MatroskaSpec::SimpleTag(Master::Start)]);
        let mut writer = WebmWriter::new(to);

        // Output sections
        let mut info_written = false;
        let mut tags_written = false;

        // Enclosing tag state
        let mut in_info = false;
        let mut in_tags = false;
        let mut in_tag = false;

        // Have to store numbers in upper scope
        let mut release_year: Option<String> = None;
        let mut season_number: Option<String> = None;
        let mut episode_number: Option<String> = None;

        let mut tags: HashMap<&str, &str> = HashMap::new();
        tags.insert(COMMENT, "");
        let title = MatroskaSpec::Title(match &self.info {
            VideoData::Movie(ent, _) => {
                tags.insert(TITLE, &ent.title);
                release_year = Some(ent.release_year.to_string());
                tags.insert(DATE_RELEASED, release_year.as_ref().unwrap());
                if let Some(imdb_id) = ent.imdb_id.as_ref() {
                    tags.insert(IMDB_ID, imdb_id);
                }

                ent.title.clone()
            }
            VideoData::Episode(ep, _) => {
                tags.insert(TITLE, &ep.series.title);
                release_year = Some(ep.series.release_year.to_string());
                tags.insert(DATE_RELEASED, release_year.as_ref().unwrap());
                season_number = Some(ep.season.to_string());
                tags.insert(SEASON_NUMBER, season_number.as_ref().unwrap());
                episode_number = Some(ep.episode.to_string());
                tags.insert(EPISODE_NUMBER, episode_number.as_ref().unwrap());
                if let Some(imdb_id) = ep.imdb_id.as_ref() {
                    tags.insert(IMDB_ID, imdb_id);
                }

                ep.title.clone()
            }
        });

        for tag in reader {
            let tag = tag?;
            if let MatroskaSpec::Info(mode) = &tag {
                in_info = match mode {
                    Master::Start => true,
                    Master::End => {
                        writer.write(&title)?;
                        info_written = true;
                        false
                    }
                    _ => in_info,
                };
                writer.write(&tag)?;
                continue;
            }

            match &tag {
                MatroskaSpec::Tracks(Master::Start)
                | MatroskaSpec::Chapters(Master::Start)
                | MatroskaSpec::Cluster(Master::Start)
                | MatroskaSpec::Cues(Master::Start)
                | MatroskaSpec::Attachments(Master::Start)
                | MatroskaSpec::Tags(Master::Start)
                    if !info_written =>
                {
                    writer.write(&MatroskaSpec::Info(Master::Full(vec![title.clone()])))?;
                    info_written = true;
                }
                _ => {}
            }

            if let MatroskaSpec::Tags(mode) = &tag {
                in_tags = match mode {
                    Master::Start => true,
                    Master::End => {
                        writer.write(&MatroskaSpec::Tag(Master::Start))?;
                        writer.write(&MatroskaSpec::Targets(Master::Full(vec![])))?;
                        for (k, v) in tags.iter() {
                            if v.len() > 0 {
                                writer.write(&MatroskaSpec::SimpleTag(Master::Start))?;
                                writer.write(&MatroskaSpec::TagName(k.to_string()))?;
                                writer.write(&MatroskaSpec::TagString(v.to_string()))?;
                                writer.write(&MatroskaSpec::SimpleTag(Master::End))?;
                            }
                        }
                        writer.write(&MatroskaSpec::Tag(Master::End))?;
                        tags_written = true;
                        false
                    }
                    _ => in_tags,
                };
                writer.write(&tag)?;
                continue;
            }

            if in_info {
                match &tag {
                    MatroskaSpec::Title(_) => {}
                    _ => writer.write(&tag)?,
                }
                continue;
            }

            if in_tags {
                match tag {
                    MatroskaSpec::SimpleTag(Master::Full(tag_data)) if in_tag => {
                        if let (
                            Some(MatroskaSpec::TagName(name)),
                            Some(MatroskaSpec::TagString(_value)),
                        ) = (
                            tag_data.iter().find(|t| match &t {
                                MatroskaSpec::TagName(_) => true,
                                _ => false,
                            }),
                            tag_data.iter().find(|t| match &t {
                                MatroskaSpec::TagString(_) => true,
                                _ => false,
                            }),
                        ) {
                            if !tags.contains_key(name.as_str()) {
                                writer.write(&MatroskaSpec::SimpleTag(Master::Full(tag_data)))?;
                            }
                        }
                        continue;
                    }
                    MatroskaSpec::SimpleTag(_) => unreachable!(),
                    MatroskaSpec::Tag(Master::Start) => in_tag = true,
                    MatroskaSpec::Tag(Master::End) => in_tag = false,
                    _ => {}
                }
                writer.write(&tag)?;
                continue;
            }

            writer.write(&tag)?;
        }

        if !tags_written {
            writer.write(&MatroskaSpec::Tags(Master::Start))?;
            writer.write(&MatroskaSpec::Tag(Master::Start))?;
            writer.write(&MatroskaSpec::Targets(Master::Full(vec![])))?;
            for (k, v) in tags.iter() {
                if v.len() > 0 {
                    writer.write(&MatroskaSpec::SimpleTag(Master::Start))?;
                    writer.write(&MatroskaSpec::TagName(k.to_string()))?;
                    writer.write(&MatroskaSpec::TagString(v.to_string()))?;
                    writer.write(&MatroskaSpec::SimpleTag(Master::End))?;
                }
            }
            writer.write(&MatroskaSpec::Tag(Master::End))?;
            writer.write(&MatroskaSpec::Tags(Master::End))?;
        }

        Ok(())
    }
}

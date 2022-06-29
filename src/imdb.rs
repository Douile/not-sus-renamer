use std::fs::metadata;
use std::path::Path;

pub use imdb_index::Searcher;
use imdb_index::{Index, MediaEntity, Query, Scored, SearchResults, TitleKind};

use crate::types::{GenericResult, VideoData};

pub fn open_if_exists_or_create_index<P1: AsRef<Path>, P2: AsRef<Path>>(
    data_dir: P1,
    index_dir: P2,
) -> GenericResult<Index> {
    match metadata(&index_dir) {
        Ok(meta) if meta.is_dir() => Index::open(&data_dir, &index_dir)
            .map_err(|e| format!("Unable to open index {:?}", e).into()),
        Ok(_) => Err("index_dir must be a directory".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Index::create(&data_dir, &index_dir)
            .map_err(|e| format!("Unable to create index {:?}", e).into()),
        Err(e) => Err(e.into()),
    }
}

pub enum Results {
    Movie(MediaEntity),
    Episode(MediaEntity, MediaEntity),
}

fn score_by_rating(entity: &MediaEntity) -> f64 {
    match entity.rating() {
        Some(rating) => rating.votes.into(),
        None => 0.0,
    }
}

pub fn search_for_video(searcher: &mut Searcher, video: &VideoData) -> imdb_index::Result<Results> {
    match video {
        VideoData::Movie(movie, _) => {
            let query = Query::new()
                .name(&movie.title)
                .kind(TitleKind::Movie)
                .kind(TitleKind::TVMovie)
                .kind(TitleKind::Short)
                .kind(TitleKind::TVShort)
                .votes_ge(0);

            let mut results = searcher.search(&query)?;
            results.rescore(score_by_rating);
            Ok(Results::Movie(
                results.into_vec().swap_remove(0).into_value(),
            ))
        }
        VideoData::Episode(episode, _) => {
            let query = Query::new()
                .name(&episode.series.title)
                .votes_ge(0)
                .kind(TitleKind::TVSeries)
                .kind(TitleKind::TVMiniSeries);

            let mut series_results = searcher.search(&query)?;
            series_results.rescore(|s| s.rating().unwrap().votes.into());
            series_results.trim(1);
            let series = series_results.into_vec().swap_remove(0).into_value();

            let query = Query::new()
                .kind(TitleKind::TVEpisode)
                .tvshow_id(&series.title().id)
                .episode_ge(episode.episode)
                .episode_le(episode.episode)
                .season_ge(episode.season)
                .season_le(episode.season);

            let mut result = searcher.search(&query)?;

            Ok(Results::Episode(
                series,
                result.into_vec().swap_remove(0).into_value(),
            ))
        }
    }
}

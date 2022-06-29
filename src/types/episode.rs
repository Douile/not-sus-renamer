use super::Entity;

#[derive(Debug, Clone)]
pub struct Episode {
    pub episode: u32,
    pub season: u32,
    pub title: String,
    pub imdb_id: Option<String>,
    pub series: Entity,
}

#[cfg(feature = "imdb")]
impl TryFrom<(&imdb_index::MediaEntity, &imdb_index::MediaEntity)> for Episode {
    type Error = &'static str;
    fn try_from(
        entities: (&imdb_index::MediaEntity, &imdb_index::MediaEntity),
    ) -> Result<Self, Self::Error> {
        if let Some(episode) = entities.0.episode() {
            // FIXME: Get episode name
            Ok(Self {
                episode: episode.episode.ok_or(
                    "Cannot create Episode from MediaEntity that does not contain episode.episode",
                )?,
                season: episode.season.ok_or(
                    "Cannot create Episode from MediaEntity that does not contain episode.season",
                )?,
                title: entities.0.title().title.clone(),
                imdb_id: Some(episode.id.clone()),
                series: Entity::from(entities.1),
            })
        } else {
            Err("Cannot create Episode from MediaEntity that does not contain episode data")
        }
    }
}

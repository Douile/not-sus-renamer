#[derive(Debug, Clone)]
pub struct Entity {
    pub title: String,
    pub release_year: u32,
    pub imdb_id: Option<String>,
}

#[cfg(feature = "imdb")]
impl From<&imdb_index::MediaEntity> for Entity {
    fn from(entity: &imdb_index::MediaEntity) -> Self {
        Self {
            title: entity.title().title.clone(),
            release_year: entity.title().start_year.unwrap_or(0),
            imdb_id: Some(entity.title().id.clone()),
        }
    }
}

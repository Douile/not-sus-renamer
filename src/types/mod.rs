pub mod entity;
pub mod episode;
pub mod metadata;
pub mod video;

pub use entity::*;
pub use episode::*;
pub use metadata::*;
pub use video::*;

pub type GenericResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

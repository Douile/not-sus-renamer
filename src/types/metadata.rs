use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use webm_iterable::{matroska_spec::MatroskaSpec, WebmIterator};

use super::GenericResult;

const STANDARD_RESOLUTIONS: [u64; 6] = [480, 720, 1080, 1440, 2160, 4320];

#[derive(Default)]
struct MatroskaData {
    duration: Option<f64>,
    pixel_width: Option<u64>,
    pixel_height: Option<u64>,
    display_width: Option<u64>,
    display_height: Option<u64>,
}

impl MatroskaData {
    fn is_complete(&self) -> bool {
        self.duration.is_some() && self.pixel_height.is_some() && self.pixel_width.is_some()
    }

    fn build(self) -> Option<Metadata> {
        if !self.is_complete() {
            return None;
        }
        let resolution = if let (Some(display_width), Some(display_height)) =
            (self.display_width, self.display_height)
        {
            (display_width, display_height)
        } else {
            (self.pixel_width.unwrap(), self.pixel_height.unwrap())
        };
        Some(Metadata {
            resolution,
            length: Some(Duration::from_secs_f64(self.duration.unwrap())),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub resolution: (u64, u64),
    pub length: Option<Duration>,
}

impl Metadata {
    pub fn from_matroska<P: AsRef<Path>>(path: P) -> GenericResult<Self> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let metadata = WebmIterator::new(&mut file, &[]);

        let mut data = MatroskaData::default();

        for tag in metadata {
            if let Ok(tag) = tag {
                match tag {
                    MatroskaSpec::Duration(duration) => data.duration = Some(duration),
                    MatroskaSpec::PixelWidth(pixel_width) => data.pixel_width = Some(pixel_width),
                    MatroskaSpec::PixelHeight(pixel_height) => {
                        data.pixel_height = Some(pixel_height)
                    }
                    MatroskaSpec::DisplayWidth(display_width) => {
                        data.display_width = Some(display_width)
                    }
                    MatroskaSpec::DisplayHeight(display_height) => {
                        data.display_width = Some(display_height)
                    }
                    _ => {}
                }
                if data.is_complete() {
                    return Ok(data.build().unwrap());
                }
            }
        }

        Err("Unable to extract metadata".into())
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

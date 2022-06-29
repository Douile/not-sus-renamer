use std::fs::OpenOptions;

use webm_iterable::WebmIterator;

mod imdb;
pub mod magic;
pub mod types;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(std::env::args().next_back().unwrap())?;
    let iterator = WebmIterator::new(&mut file, &[]);

    for tag in iterator {
        println!("{:?}", tag?);
    }

    Ok(())
}

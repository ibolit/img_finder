use std::collections::HashMap;

use bytesize::ByteSize;

use crate::library::image;
use crate::library::io::read_from_yaml;

pub fn stats(input: &str) {
    let images: HashMap<String, Vec<image::Image>> =
        read_from_yaml(input).unwrap_or_else(|_| panic!("Failed to read the file"));

    let mut total_size = 0;
    let mut clean_size = 0;
    for (_, images) in images {
        clean_size += images[0].size;
        total_size += images.iter().fold(0, |mut a, b| {
            a += b.size;
            a
        });
    }

    println!(
        "Total size: {}\nClean size: {},\nSpace saved: {}",
        total_size,
        clean_size,
        total_size - clean_size
    );

    let size_in_bytes = ByteSize::b(clean_size);
    println!("Clean size human: {}", size_in_bytes);
    println!("Si: {}", size_in_bytes.display().si());
}

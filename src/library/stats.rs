use std::collections::HashMap;

use bytesize::ByteSize;
use chrono::{DateTime, Utc};

use crate::library::image;
use crate::library::io::{read_from_yaml, write_to_yaml};

pub fn stats(input: &str) {
    let images: HashMap<String, Vec<image::Image>> =
        read_from_yaml(input).unwrap_or_else(|_| panic!("Failed to read the file"));

    let mut total_size = 0;
    let mut clean_size = 0;
    for imgs in images.values() {
        clean_size += imgs[0].size;
        total_size += imgs.iter().fold(0, |mut a, b| {
            a += b.size;
            a
        });
    }

    {
        let total_size = ByteSize::b(total_size);
        let clean_size = ByteSize::b(clean_size);

        println!(
            "Total size: {}\nClean size: {},\nSpace saved: {}",
            total_size.display().si(),
            clean_size.display().si(),
            (total_size - clean_size).display().si()
        );
    }

    let mut flat_images = images
        .values()
        .map(|v| {
            v.iter()
                .filter(|&i| i.date.is_some())
                .min_by_key(|&i| i.date.unwrap())
                .unwrap_or(&v[0])
                .clone()
        })
        .collect::<Vec<image::Image>>();

    flat_images.sort_by(|a, b| a.date.cmp(&b.date));

    let output: Vec<(String, String)> = flat_images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            (
                format!("{}-{:0>6}", date_to_string(img.date.unwrap_or_default()), i),
                img.path.clone(),
            )
        })
        .collect();
    write_to_yaml(&output, "move_plan_2.yaml");
}

fn date_to_string(date: DateTime<Utc>) -> String {
    date.format("%Y-%m-%d").to_string()
}

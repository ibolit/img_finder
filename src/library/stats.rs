use std::collections::HashMap;
use std::fs::create_dir;
use std::os::unix::fs::symlink;
use std::path::Path;

use bytesize::ByteSize;
use chrono::{DateTime, Utc};

use crate::library::image::{self, extension};
use crate::library::io::{read_from_yaml, write_to_yaml};

pub fn stats(input: &str) {
    let images: HashMap<String, Vec<image::Image>> =
        read_from_yaml(input).unwrap_or_else(|_| panic!("Failed to read the file"));

    let mut total_size = 0;
    let mut clean_size = 0;
    for imgs in images.values() {
        clean_size += imgs[0].file_size;
        total_size += imgs.iter().fold(0, |mut a, b| {
            a += b.file_size;
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

    let flat_images = sort_images(images);

    let output: Vec<(String, String)> = flat_images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            (
                format!("{}{:0>6}", date_to_string(img.date.unwrap_or_default()), i),
                img.path.clone(),
            )
        })
        .collect();
    write_to_yaml(&output, "move_plan_2.yaml");
}

fn flatten_images<'a>(
    images: &'a HashMap<String, Vec<image::Image>>,
) -> impl Iterator<Item = image::Image> + 'a {
    images.values().map(|v| {
        v.iter()
            .filter(|&i| i.date.is_some())
            .min_by_key(|&i| i.date.unwrap())
            .unwrap_or(&v[0])
            .clone()
    })
}

fn sort_images(images: HashMap<String, Vec<image::Image>>) -> Vec<image::Image> {
    let mut flat_images = flatten_images(&images).collect::<Vec<image::Image>>();
    flat_images.sort_by(|a, b| a.date.cmp(&b.date));
    flat_images
}

pub fn symlink_non_date(input: &str, output: &str) {
    let images: HashMap<String, Vec<image::Image>> =
        read_from_yaml(input).unwrap_or_else(|_| panic!("Failed to read the file"));
    let sorted_images = sort_images(images);
    create_dir(output).unwrap_or_else(|_| panic!("Failed to create the output dir"));
    for (i, img) in sorted_images
        .iter()
        .take_while(|i| i.date.is_none())
        .enumerate()
    {
        let ext = extension(Path::new(&img.path.clone())).unwrap();
        let new_filename = format!("{output}/IMG_{:0>6}.{}", i, ext);
        let res = symlink(img.path.clone(), new_filename);
        if let Err(e) = res {
            eprintln!("Failed to symlink file {}", e);
        }
    }
}

fn date_to_string(date: DateTime<Utc>) -> String {
    date.format("%Y%m%d").to_string()
}

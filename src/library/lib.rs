use chrono::{DateTime, Utc};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashSet, fs, fs::create_dir_all, path::PathBuf};

use nom_exif::Error;
use nom_exif::{
    EntryValue::NaiveDateTime, EntryValue::Time, ExifIter, ExifTag, MediaParser, MediaSource,
};

pub fn log_time(msg: &str) {
    println!("{}: {}", Utc::now(), msg);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub path: String,
    pub name: String,
    pub sha256: String,
    pub date: Option<DateTime<Utc>>,
}

impl Image {
    pub fn new(path: String, sha256: String, date: Option<DateTime<Utc>>) -> Self {
        let name = match PathBuf::from(&path).file_name() {
            None => "Unknown".to_owned(),
            Some(name) => name
                .to_str()
                .expect("Failed to convert filename to str")
                .to_owned(),
        };
        Self {
            path,
            name,
            sha256,
            date,
        }
    }
}

pub fn get_exif_datetime(path: &str) -> Option<DateTime<Utc>> {
    let mut parser = MediaParser::new();
    let media_source = MediaSource::file_path(path);
    if let Err(_e) = media_source {
        None
    } else {
        let media_source = media_source.unwrap();

        let iter_res: Result<ExifIter, Error> = parser.parse(media_source);

        let mut candidate_time: Option<DateTime<Utc>> = None;

        match iter_res {
            Err(_e) => None,
            Ok(iter) => {
                for a in iter {
                    let tag = a.tag().unwrap_or(ExifTag::Make);
                    if time_tags().contains(&tag) {
                        if let Some(Time(c)) = a.get_value() {
                            let _ = candidate_time.insert(c.to_utc());
                        }
                        if let Some(NaiveDateTime(c)) = a.get_value() {
                            candidate_time.get_or_insert_with(move || {
                                min(candidate_time.unwrap_or(c.and_utc()), c.and_utc())
                            });
                        }
                    }
                }
                candidate_time
            }
        }
    }
}

pub fn move_to_datetime_folder(img: &Image) -> Image {
    let mut parser = MediaParser::new();
    let media_source = MediaSource::file_path(&img.path);
    if let Err(_e) = media_source {
        return img.clone();
    }

    let media_source = media_source.unwrap();

    let iter_res: Result<ExifIter, Error> = parser.parse(media_source);

    let mut candidate_time = Utc::now();

    match iter_res {
        Err(_e) => {
            Image {
                path: format!(
                    "NO_EXIF" // "/Volumes/Hippie/NO_EXIF/{}-{}",
                              // candidate_time.format("%H-%M-%S-%f"),
                              // img.name.clone(),
                ),
                ..img.clone()
            }
            // println!("Img {} has no exif", img.path);
        }
        Ok(iter) => {
            for a in iter {
                let tag = a.tag().unwrap_or(ExifTag::Make);
                if time_tags().contains(&tag) {
                    if let Some(Time(c)) = a.get_value() {
                        candidate_time = min(candidate_time, c.to_utc());
                    }
                    if let Some(NaiveDateTime(c)) = a.get_value() {
                        candidate_time = min(candidate_time, c.and_utc());
                    }
                }
            }
            let path = candidate_time
                .format("/Volumes/Hippie/SORTED/%Y/%m/%d")
                .to_string();
            create_dir_all(&path).unwrap();

            let new_img_path = format!(
                "{}/{}-{}",
                path,
                candidate_time.format("%H-%M-%S"),
                img.name
            );

            fs::rename(&img.path, &new_img_path)
                .unwrap_or_else(|_| panic!("Failed to move it move it: {}", &img.path));

            Image {
                path: new_img_path,
                ..img.clone()
            }
        }
    }
}

fn time_tags() -> &'static HashSet<ExifTag> {
    static HASHSET: OnceLock<HashSet<ExifTag>> = OnceLock::new();
    HASHSET.get_or_init(|| {
        HashSet::from([
            ExifTag::CreateDate,
            ExifTag::ModifyDate,
            ExifTag::DateTimeOriginal,
            ExifTag::GPSDateStamp,
        ])
    })
}

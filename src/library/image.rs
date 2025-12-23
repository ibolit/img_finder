use std::path::Path;

use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashSet, path::PathBuf, sync::OnceLock};

use nom_exif::Error;
use nom_exif::{
    EntryValue::NaiveDateTime, EntryValue::Time, ExifIter, ExifTag, MediaParser, MediaSource,
};

pub struct FileFactory {
    image_formats: Vec<String>,
    known_formats: Vec<String>,
}

impl FileFactory {
    fn new(image_formats: Vec<String>, known_formats: Vec<String>) -> Self {
        FileFactory {
            image_formats,
            known_formats,
        }
    }

    pub fn from_path(&self, path: &Path) -> File {
        if path.is_symlink() {
            return File::SymLink(path.into());
        }
        if path.is_dir() {
            return File::Dir(path.into());
        }
        let ext = extension(path);
        if let Some(ext) = ext {
            if is_image(&ext, &self.image_formats) {
                File::Image(path.into())
            } else if !is_known(&ext, &self.known_formats) {
                File::Unknown(path.into())
            } else {
                File::Known(path.into())
            }
        } else {
            File::Unknown(path.into())
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum File {
    SymLink(Box<Path>),
    Unknown(Box<Path>),
    Known(Box<Path>),
    Image(Box<Path>),
    Dir(Box<Path>),
}

impl File {
    pub fn factory(image_formats: Vec<String>, known_formats: Vec<String>) -> FileFactory {
        FileFactory::new(image_formats, known_formats)
    }
}

pub fn extension(entry: &Path) -> Option<String> {
    Some(entry.extension()?.to_str()?.to_lowercase())
}

fn is_image(ext: &str, img_formats: &[String]) -> bool {
    img_formats.contains(&ext.to_owned())
}

fn is_known(ext: &str, known_formats: &[String]) -> bool {
    known_formats.contains(&ext.to_owned())
}

// pub struct FileIterator {}
// impl Iterator for FileIterator {
//     type Item = File;
//     fn next(&mut self) -> Option<Self::Item> {
//         None
//     }
// }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Dimensions(pub u32, pub u32);

impl From<(u32, u32)> for Dimensions {
    fn from(value: (u32, u32)) -> Self {
        let (w, h) = value;
        Dimensions(w, h)
    }
}

impl Dimensions {
    pub fn reverse(&self) -> Self {
        Dimensions(self.1, self.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub path: String,
    pub name: String,
    pub sha256: String,
    pub date: Option<DateTime<Utc>>,
    pub file_size: u64,
    pub dimensions: Dimensions,
}

impl Image {
    pub fn new(
        path: String,
        sha256: String,
        date: Option<DateTime<Utc>>,
        file_size: u64,
        dimensions: Dimensions,
    ) -> Self {
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
            file_size,
            dimensions,
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

#[cfg(test)]
mod test {
    use super::*;
    use std::env::current_dir;

    #[test]
    fn test_one() {
        let image_formats = vec!["jpeg".to_string(), "jpg".to_string(), "png".to_string()];
        let known_formats = vec!["wav".to_string(), "avi".to_string(), "txt".to_string()];
        let file_factory = File::factory(image_formats, known_formats);

        fn unknown(x: &Path) -> File {
            File::Unknown(x.into())
        }
        fn known(x: &Path) -> File {
            File::Known(x.into())
        }
        fn image(x: &Path) -> File {
            File::Image(x.into())
        }
        fn dir(x: &Path) -> File {
            File::Dir(x.into())
        }

        let current_dir = current_dir().unwrap().into_boxed_path();

        let params: Vec<(&Path, fn(&Path) -> File)> = vec![
            (&current_dir, dir),
            (Path::new("/something/in/the/dir"), unknown),
            (Path::new("/something/in/the/dir.unknown_ext"), unknown),
            (Path::new("/something/in/the/img.jpg"), image),
            (Path::new("/something/in/the/img.jpeg"), image),
            (Path::new("/something/img.txt"), known),
        ];
        for (path, typ) in params {
            let f = file_factory.from_path(path);
            assert!(f == typ(path.into()));
        }
    }
}

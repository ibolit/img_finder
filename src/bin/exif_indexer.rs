use chrono::Utc;
use nom_exif::{
    EntryValue::NaiveDateTime, EntryValue::Time, ExifIter, ExifTag, MediaParser, MediaSource,
};
use std::{cmp::min, collections::HashSet, fs::create_dir_all};

fn main() {
    let mut parser = MediaParser::new();
    let f = "/Volumes/Hippie/SORTED/13-56-53-IMG_4085.JPG";
    let ms = MediaSource::file_path(f).unwrap();

    let time_tags = HashSet::from([
        ExifTag::CreateDate,
        ExifTag::ModifyDate,
        ExifTag::DateTimeOriginal,
        ExifTag::GPSDateStamp,
    ]);

    if ms.has_exif() {
        println!("I have exif");
        let iter: ExifIter = parser.parse(ms).unwrap();

        let mut candidate_time = Utc::now();
        for a in iter {
            let tag = a.tag().unwrap_or(ExifTag::Make);
            // println!("{:?} - {:?}", a.tag(), a.get_value());
            if time_tags.contains(&tag) {
                if let Some(Time(c)) = a.get_value() {
                    candidate_time = min(candidate_time, c.to_utc());
                }
                if let Some(NaiveDateTime(c)) = a.get_value() {
                    candidate_time = min(candidate_time, c.and_utc());
                }
            }
        }
        println!("Candidate time is {}", candidate_time);
        let path = candidate_time.format("./%Y/%m/%d").to_string();
        println!("Folder name: {}", path);
        create_dir_all(path).unwrap();
    }
}

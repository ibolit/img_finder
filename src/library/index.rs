use crate::library::util::log_time;

use image::GenericImageView;
use iter_read::IterRead;
use sha2::{Digest, Sha256};
use std::io;
extern crate image;
use image::Pixel;

use crate::library::image as my;
use crate::library::image::get_exif_datetime;
use crate::library::io::write_to_yaml;

use indicatif::ProgressIterator;
use std::ffi::OsStr;

use std::path::Path;
use std::{
    collections::HashMap,
    fs,
    sync::mpsc::{channel, Sender},
};
use threadpool::ThreadPool;
use walkdir::{DirEntry, WalkDir};

pub fn process_whole_task(
    folder: &str,
    output: &str,
    image_formats: Vec<String>,
    known_formats: Vec<String>,
    skip_dirs: Vec<String>,
    verbose: bool,
) {
    let pool = ThreadPool::new(4);
    let (img_tx, img_rx) = channel();
    let mut unknowns: HashMap<String, Vec<String>> = HashMap::new();

    let mut imgs = 0;

    log_time("Before the dir walk", verbose);
    let file_factory = my::File::factory(image_formats, known_formats);
    for entry in WalkDir::new(folder)
        .follow_root_links(false)
        .follow_links(false)
        .max_depth(4)
        .same_file_system(true)
        .into_iter()
        .filter_entry(|e| should_exclude(e, &skip_dirs))
    {
        if let Err(err) = entry {
            eprintln!("{}", err);
            continue;
        }
        let my_file = file_factory.from_path(&entry.unwrap().into_path());
        process_file(my_file, img_tx.clone(), &pool, &mut imgs, &mut unknowns);
    }

    log_time("After the loop", verbose);

    let imgs_by_hash = img_rx.iter().take(imgs).progress_count(imgs as u64).fold(
        HashMap::<String, Vec<my::Image>>::new(),
        |mut map, img| {
            map.entry(img.sha256.clone()).or_default().push(img);
            map
        },
    );
    log_time("Done", verbose);

    write_to_yaml(&imgs_by_hash, &compute_yaml_name("images", output));
    write_to_yaml(&unknowns, &compute_yaml_name("unexp", output));
}

fn compute_yaml_name(intent: &str, output: &str) -> String {
    format!("{}_{}.yaml", urlencoding::encode(output), intent)
}

fn should_exclude(entry: &DirEntry, skip_dirs: &[String]) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    if entry.file_type().is_dir() {
        return !skip_dirs.contains(&file_name.as_ref().to_string());
    }
    true
}

fn pixel_sha(path: &Path) -> String {
    let img = image::open(path).unwrap();

    let my_iter = img.pixels().flat_map(|(_, _, pixel)| pixel.to_rgb().0);

    let mut reader = IterRead::new(my_iter);
    let mut hasher = Sha256::new();
    io::copy(&mut reader, &mut hasher).expect("Did it actually fail?");
    let result = hasher.finalize();

    hex::encode(result)
}

fn process_img(path: &Path, img_tx: Sender<my::Image>) {
    let sha = pixel_sha(path);

    let metadata = fs::metadata(path).expect("Failed to get the len of a file");
    let size = metadata.len();

    let path = path
        .to_str()
        .unwrap_or_else(|| panic!("Path has no str, {:?}", path))
        .to_owned();
    img_tx
        .send(my::Image::new(
            path.clone(),
            sha,
            get_exif_datetime(&path),
            size,
        ))
        .expect("Chan must not be closed");
}

fn process_unknown(path: &Path, unknowns: &mut HashMap<String, Vec<String>>) {
    let str_path = path
        .to_str()
        .unwrap_or_else(|| panic!("path has no str {:?}", path))
        .to_owned();
    let ext = path
        .extension()
        .unwrap_or(OsStr::new("_"))
        .to_str()
        .unwrap_or("_")
        .to_string();
    unknowns
        .entry(ext)
        .and_modify(|paths| paths.push(str_path.to_owned()))
        .or_insert(vec![str_path]);
}

fn process_file(
    my_file: my::File,
    img_tx: Sender<my::Image>,
    pool: &ThreadPool,
    imgs: &mut usize,
    unknowns: &mut HashMap<String, Vec<String>>,
) {
    match my_file {
        my::File::SymLink(_) | my::File::Dir(_) | my::File::Known(_) => (),
        my::File::Image(p) => {
            *imgs += 1;
            let img_tx = img_tx.clone();
            pool.execute(move || {
                process_img(&p, img_tx.clone());
            });
        }
        my::File::Unknown(p) => {
            process_unknown(&p, unknowns);
        }
    }
}

// pub fn move_to_datetime_folder(img: &Image) -> Image {
//     let mut parser = MediaParser::new();
//     let media_source = MediaSource::file_path(&img.path);
//     if let Err(_e) = media_source {
//         return img.clone();
//     }

//     let media_source = media_source.unwrap();

//     let iter_res: Result<ExifIter, Error> = parser.parse(media_source);

//     let mut candidate_time = Utc::now();

//     match iter_res {
//         Err(_e) => {
//             Image {
//                 path: format!(
//                     "NO_EXIF" // "/Volumes/Hippie/NO_EXIF/{}-{}",
//                               // candidate_time.format("%H-%M-%S-%f"),
//                               // img.name.clone(),
//                 ),
//                 ..img.clone()
//             }
//             // println!("Img {} has no exif", img.path);
//         }
//         Ok(iter) => {
//             for a in iter {
//                 let tag = a.tag().unwrap_or(ExifTag::Make);
//                 if time_tags().contains(&tag) {
//                     if let Some(Time(c)) = a.get_value() {
//                         candidate_time = min(candidate_time, c.to_utc());
//                     }
//                     if let Some(NaiveDateTime(c)) = a.get_value() {
//                         candidate_time = min(candidate_time, c.and_utc());
//                     }
//                 }
//             }
//             let path = candidate_time
//                 .format("/Volumes/Hippie/SORTED/%Y/%m/%d")
//                 .to_string();
//             create_dir_all(&path).unwrap();

//             let new_img_path = format!(
//                 "{}/{}-{}",
//                 path,
//                 candidate_time.format("%H-%M-%S"),
//                 img.name
//             );

//             fs::rename(&img.path, &new_img_path)
//                 .unwrap_or_else(|_| panic!("Failed to move it move it: {}", &img.path));

//             Image {
//                 path: new_img_path,
//                 ..img.clone()
//             }
//         }
//     }
// }

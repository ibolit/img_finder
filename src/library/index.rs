extern crate image;

use chrono::NaiveDateTime;
use little_exif::exif_tag::ExifTag;
use little_exif::metadata::Metadata;
use std::collections::BTreeMap;
use std::path::Path;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs, io,
    sync::mpsc::{channel, Sender},
};
use threadpool::ThreadPool;
use walkdir::{DirEntry, WalkDir};

use image::{GenericImageView, Pixel};
use indicatif::ProgressIterator;
use iter_read::IterRead;
use sha2::{Digest, Sha256};

use crate::library::image::Dimensions;
use crate::library::io::read_from_yaml;
use crate::library::stats::flatten_images;
use crate::library::{
    image::{self as my, get_exif_datetime},
    io::write_to_yaml,
    util::log_time,
};

pub type ImageStore = HashMap<String, Vec<my::Image>>;

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
        ImageStore::new(),
        |mut map, img| {
            map.entry(img.sha256.clone()).or_default().push(img);
            map
        },
    );
    log_time("Done", verbose);

    write_image_store(&imgs_by_hash, &compute_yaml_name("images", output));
    // write_to_yaml(&imgs_by_hash, &compute_yaml_name("images", output));
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

fn pixel_sha(path: &Path) -> Result<(String, Dimensions), String> {
    let img = image::open(path).map_err(|e| format!("{:?}", e))?;
    let dims: Dimensions = img.dimensions().into();

    let my_iter = img.pixels().flat_map(|(_, _, pixel)| pixel.to_rgb().0);

    let mut reader = IterRead::new(my_iter);
    let mut hasher = Sha256::new();
    let copy_result = io::copy(&mut reader, &mut hasher).map_err(|e| format!("{:?}", e));
    match copy_result {
        Ok(_) => Ok((hex::encode(hasher.finalize()), dims)),
        Err(_) => Ok((
            sha256::try_digest(&path)
                .unwrap_or_else(|_| panic!("Failed to calculate sha for file {:?}", &path)),
            dims,
        )),
    }
    // let result = ;
}

fn process_img(path: &Path, img_tx: Sender<my::Image>) {
    let (sha, dims) = pixel_sha(path).unwrap_or_else(|_| {
        (
            sha256::try_digest(&path)
                .unwrap_or_else(|_| panic!("Failed to calculate sha for file {:?}", &path)),
            Dimensions(0, 0),
        )
    });

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
            dims,
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

pub fn get_info(path: &str) {
    let date_format = "%Y:%m:%d %H:%M:%S";
    let exif_date = get_exif_datetime(path);
    match exif_date {
        Some(exif_date) => println!("{}", exif_date.format(date_format)),
        None => println!("None"),
    }
}

pub fn set_datetime(path: &str, date: &str) {
    let date_format = "%Y:%m:%d %H:%M:%S";

    let parsed_date = NaiveDateTime::parse_from_str(date, date_format).expect("Bad date format");

    let exif_date = get_exif_datetime(path);
    if exif_date.is_some() {
        eprintln!("You shalt not ovewrite an existing date!");
        return;
    }

    let image_path = std::path::Path::new(path);
    let mut metadata = Metadata::new_from_path(image_path).unwrap_or(Metadata::new());
    let date_string = parsed_date.format(date_format).to_string();
    metadata.set_tag(ExifTag::DateTimeOriginal(date_string));
    metadata.write_to_file(image_path).unwrap();
    let exif_date = get_exif_datetime(path).unwrap();
    eprintln!("New date is: {exif_date:?}");
}

pub fn rescan_null_dates(input_yaml: &str, output: &str) {
    let mut indexed_files: ImageStore =
        read_from_yaml(&input_yaml).expect("Failed to open the index file");

    let flat_imgs = flatten_images(&indexed_files);

    let updated_imgs: Vec<my::Image> = flat_imgs
        .filter(|i| i.date.is_none())
        .map(|mut i| {
            i.date = get_exif_datetime(&i.path);
            i
        })
        .filter(|i| i.date.is_some())
        .collect();

    for upd_img in updated_imgs {
        let img_vec = indexed_files
            .get_mut(&upd_img.sha256)
            .expect("img_vec is none");
        let found_img = img_vec
            .iter_mut()
            .find(|i| i.path == upd_img.path)
            .expect("Didn't find an image with that path");
        found_img.date = upd_img.date;
    }
    write_image_store(&indexed_files, output);
}

fn write_image_store(images: &ImageStore, to: &str) {
    let mut btree_map: BTreeMap<String, Vec<my::Image>> = BTreeMap::new();
    for (sha, imgs) in images {
        let mut imgs = imgs.clone();
        imgs.sort_by_key(|i| (i.date, i.path.clone()));
        btree_map.insert(sha.clone(), imgs);
    }
    write_to_yaml(&btree_map, to);
}

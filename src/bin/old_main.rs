use img_finder::library::config::Config;
use img_finder::library::image;
use img_finder::library::io::{read_from_yaml, write_to_yaml};
use img_finder::library::lib::{log_time, move_to_datetime_folder, Image};

use indicatif::ProgressIterator;
use sha256;
use std::ffi::OsStr;

use std::path::Path;
use std::sync::mpsc::Sender;
use std::{collections::HashMap, sync::mpsc::channel};
use threadpool::ThreadPool;
use walkdir::{DirEntry, WalkDir};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author = "Timofey",
    version = "2.2.2",
    about = "this is about",
    long_about = "This is about long"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(long_about = "This is about long for this command")]
    Add {
        #[arg(short, long)]
        name: String,
    },
    List,
}

// static EXIFABLE: &[&str] = &[
//     "jpeg", "jpg", "heic", "png", "mov", "mp4", "gif", "aae", "tiff", "wav", "avi", "m4v", "mpg",
//     "mpeg", "tiff", "tif", "raf", "raw", "bmp", "psd", "xmp", "wmv",
// ];

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    folder: String,
}

fn main_2() {
    let args = Args::parse();

    let ht: HashMap<String, Vec<Image>> = read_from_yaml("hippie_images.yaml").unwrap();

    let images = ht
        .values()
        .flatten()
        .map(move_to_datetime_folder)
        .progress_count(ht.len() as u64);

    let mut no_exifs: Vec<Image> = vec![];
    let images = images.filter_map(|img| {
        if img.path.clone() == "NO_EXIF" {
            no_exifs.push(ht[&img.sha256].first().unwrap().clone());
            None
        } else {
            Some(img)
        }
    });
    let new_ht: HashMap<String, Image> = images.map(|img| (img.sha256.clone(), img)).collect();

    write_to_yaml(
        &new_ht,
        &format!("{}_sorted_images.yaml", urlencoding::encode(&args.folder)),
    );

    write_to_yaml(
        &no_exifs,
        &format!("{}_no_exif_images.yaml", urlencoding::encode(&args.folder)),
    );
}

fn is_excluded(entry: &DirEntry, config: &Config) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    if entry.file_type().is_dir() {
        return !config.skip_dirs.contains(&file_name.as_ref().to_string());
    }
    true
}

fn process_img(path: &Path, img_tx: Sender<Image>) {
    let sha =
        sha256::try_digest(&path).expect(&format!("Failed to calculate sha for file {:?}", &path));
    img_tx
        .send(Image::new(
            path.to_str()
                .expect(&format!("Path has no str, {:?}", path))
                .to_owned(),
            sha,
        ))
        .expect("Chan must not be closed");
}

fn process_unknown(path: &Path, unknowns: &mut HashMap<String, Vec<String>>) {
    let str_path = path
        .to_str()
        .expect(&format!("path has no str {:?}", path))
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
    my_file: image::File,
    img_tx: Sender<Image>,
    pool: &ThreadPool,
    imgs: &mut usize,
    unknowns: &mut HashMap<String, Vec<String>>,
) {
    match my_file {
        image::File::SymLink(_) | image::File::Dir(_) | image::File::Known(_) => (),
        image::File::Image(p) => {
            *imgs += 1;
            let img_tx = img_tx.clone();
            pool.execute(move || {
                process_img(&p, img_tx.clone());
            });
        }
        image::File::Unknown(p) => {
            process_unknown(&p, unknowns);
        }
    }
}

fn main() {
    let config = Config::new();

    let pool = ThreadPool::new(4);
    let (img_tx, img_rx) = channel();
    let mut unknowns: HashMap<String, Vec<String>> = HashMap::new();
    let args = Args::parse();
    println!("{:?}", args);

    let mut imgs = 0;

    log_time("Before the dir walk");
    let file_factory = image::File::factory(&config);
    for entry in WalkDir::new(&args.folder)
        .follow_root_links(false)
        .follow_links(false)
        .max_depth(4)
        .same_file_system(true)
        .into_iter()
        .filter_entry(|e| is_excluded(e, &config))
    {
        if let Err(err) = entry {
            println!("{}", err);
            continue;
        }
        let path = entry.unwrap().into_path();
        let my_file = file_factory.from_path(&path);
        process_file(my_file, img_tx.clone(), &pool, &mut imgs, &mut unknowns);
    }

    log_time("After the loop");

    let imgs_by_hash =
        img_rx
            .iter()
            .take(imgs)
            .fold(HashMap::<String, Vec<Image>>::new(), |mut map, img| {
                map.entry(img.sha256.clone()).or_default().push(img);
                map
            });

    write_to_yaml(
        &imgs_by_hash,
        &format!("{}_images.yaml", urlencoding::encode(&args.folder)),
    );
    write_to_yaml(
        &unknowns,
        &format!("{}_unexp.yaml", urlencoding::encode(&args.folder)),
    );
}

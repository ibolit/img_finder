use img_finder::library::io::{read_yaml_file, write_to_yaml};
use img_finder::library::lib::{log_time, move_to_datetime_folder, Image};

use indicatif::ProgressIterator;
use sha256;
use std::io::prelude::*;

use std::{collections::HashMap, fs::File, path::PathBuf, sync::mpsc::channel};
use threadpool::ThreadPool;
use walkdir::WalkDir;

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

static IMGS: &[&str] = &[
    "jpeg", "jpg", "heic", "png", "mov", "mp4", "gif", "aae", "tiff", "wav", "avi", "m4v", "mpg",
    "mpeg", "tiff", "tif", "raf", "raw", "bmp", "psd", "xmp", "wmv",
];

// static EXIFABLE: &[&str] = &[
//     "jpeg", "jpg", "heic", "png", "mov", "mp4", "gif", "aae", "tiff", "wav", "avi", "m4v", "mpg",
//     "mpeg", "tiff", "tif", "raf", "raw", "bmp", "psd", "xmp", "wmv",
// ];

static KNOWN_EXT: &[&str] = &[
    "timestamp",
    "toml",
    "o",
    "rmeta",
    "iml",
    "sample",
    "rs",
    "bin",
];

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    folder: String,
}

fn main() {
    let args = Args::parse();

    let ht: HashMap<String, Vec<Image>> = read_yaml_file("hippie_images.yaml").unwrap();

    let images = ht
        .values()
        .flatten()
        .map(move_to_datetime_folder)
        .progress_count(ht.len() as u64);

    let mut no_exifs: Vec<Image> = vec![];
    let images = images
        .map(|img| {
            if img.path.clone() == "NO_EXIF" {
                no_exifs.push(ht[&img.sha256].first().unwrap().clone());
                None
            } else {
                Some(img)
            }
        })
        .flatten();
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

fn main_2() {
    let pool = ThreadPool::new(4);
    let (img_tx, img_rx) = channel();
    let mut unknowns: HashMap<String, Vec<String>> = HashMap::new();
    let args = Args::parse();
    println!("{:?}", args);

    let mut imgs = 0;

    log_time("Before the dir walk");
    for entry in WalkDir::new(&args.folder)
        .follow_root_links(false)
        .follow_links(false)
        .max_depth(4)
        .same_file_system(true)
    {
        if let Err(err) = entry {
            println!("{}", err);
            continue;
        }
        let path = entry.unwrap().into_path();
        if path.is_symlink() {
            println!("Skipping a symlink at {:?}", path);
            continue;
        }
        let curr_dir = path.file_name().unwrap().to_str().unwrap();

        if curr_dir == "System"
            || curr_dir == "Users"
            || curr_dir == "etc"
            || curr_dir == "private"
            || curr_dir == "usr"
        {
            println!("Got {}", curr_dir);
            continue;
        }

        let ext = extension(&path);
        if let Some(ext) = ext {
            if is_image(&ext) {
                imgs += 1;
                let img_tx = img_tx.clone();
                pool.execute(move || {
                    let sha = sha256::try_digest(&path)
                        .expect(&format!("Failed to calculate sha for file {:?}", &path));
                    img_tx
                        .send((
                            path.to_str()
                                .expect(&format!("Path has no str, {:?}", path))
                                .to_owned(),
                            sha,
                        ))
                        .expect("Chan must not be closed");
                });
            } else if !is_known(&ext) {
                let str_path = path
                    .to_str()
                    .expect(&format!("path has no str {:?}", path))
                    .to_owned();
                unknowns
                    .entry(ext)
                    .and_modify(|paths| paths.push(str_path.to_owned()))
                    .or_insert(vec![str_path]);
            }
        }
    }

    log_time("After the loop");

    let images = img_rx
        .iter()
        .take(imgs)
        .map(|(path, sha)| Image::new(path, sha))
        .collect::<Vec<Image>>();
    log_time("Done calculating shas");
    let mut imgs_by_hash: HashMap<String, Vec<Image>> = HashMap::new();
    for i in images {
        imgs_by_hash
            .entry(i.sha256.clone())
            .or_insert(vec![])
            .push(i);
    }
    log_time("Done making the hashmap");

    let yaml = serde_yaml::to_string(&imgs_by_hash).expect("Da fuck");
    log_time("Done serializing");
    let mut file = File::create(format!("{}_images.yaml", urlencoding::encode(&args.folder)))
        .expect("Failed to open a file for writing image info");
    file.write_all(yaml.as_bytes())
        .expect("Failed to write image info");

    let mut unexp_file = File::create(format!("{}_unexp.yaml", urlencoding::encode(&args.folder)))
        .expect("Failed to create a file for writing unexpected things");
    let unknowns_yaml = serde_yaml::to_string(&unknowns).expect("Convert unknowns to yaml");
    unexp_file
        .write_all(unknowns_yaml.as_bytes())
        .expect("failed to write unknonwn");
}

fn extension(entry: &PathBuf) -> Option<String> {
    Some(entry.extension()?.to_str()?.to_lowercase())
}

fn is_image(ext: &str) -> bool {
    IMGS.contains(&ext)
}

fn is_known(ext: &str) -> bool {
    KNOWN_EXT.contains(&ext)
}

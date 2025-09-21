use chrono::prelude::*;
use clap::Parser;
use indicatif::ProgressIterator;
use serde::{Deserialize, Serialize};
use sha256;
use std::io::prelude::*;
use std::thread::current;
use std::{
    collections::HashMap,
    fs::{self, File},
    path::PathBuf,
    sync::mpsc::channel,
};
use threadpool::ThreadPool;
use walkdir::WalkDir;

static IMGS: &[&str] = &[
    "jpeg", "jpg", "heic", "png", "mov", "mp4", "gif", "aae", "tiff", "wav", "avi", "m4v", "mpg",
    "mpeg", "tiff", "tif", "raf", "raw", "bmp", "psd", "xmp", "wmv",
];
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
    let contents = fs::read_to_string("hippie_images.yaml").unwrap();
    let mut ht: HashMap<String, Vec<Image>> = serde_yaml::from_str(&contents).unwrap();

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
        // .max_depth(4)
        .same_file_system(true)
    {
        if let Err(x) = entry {
            println!("{}", x);
            continue;
        }
        let path = entry.unwrap().into_path();
        if path.is_symlink() {
            println!("Skipping a symlink at {:?}", path);
            continue;
        }
        if path.is_dir() {
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
        .progress_count(imgs as u64)
        .map(|(path, sha)| Image::new(path, sha))
        .collect::<Vec<Image>>();
    log_time("Done calculating shas");
    let mut imgs_by_hash: HashMap<String, Vec<Image>> = HashMap::new();
    let mut imgs_to_remove: Vec<(Image, Vec<Image>)> = vec![];

    for i in images.iter() {
        if ht.contains_key(&i.sha256) {
            imgs_to_remove.push((i.clone(), ht[&i.sha256].clone()));
        } else {
            // ht.insert(i.sha256.clone(), vec![i.clone()]);
            imgs_by_hash
                .entry(i.sha256.clone())
                .or_insert(vec![])
                .push(i.clone());
        }
    }
    log_time("Done making the hashmap");

    let yaml = serde_yaml::to_string(&imgs_by_hash).expect("Da fuck");
    log_time("Done serializing");
    let mut file = File::create(format!("{}_images.yaml", urlencoding::encode(&args.folder)))
        .expect("Failed to open a file for writing image info");
    file.write_all(yaml.as_bytes())
        .expect("Failed to write image info");

    // let yaml_new_hippie_imgs = serde_yaml::to_string(&ht).expect("Da fuck");
    // log_time("Done serializing");
    // let mut new_hippie_yaml = fs::OpenOptions::new()
    //     .write(true)
    //     .truncate(true)
    //     .open("hippie_images.yaml")
    //     .expect("Failed to open a file for writing image info");
    // new_hippie_yaml
    //     .write_all(yaml_new_hippie_imgs.as_bytes())
    //     .expect("Failed to write image info");

    let mut unexp_file = File::create(format!("{}_unexp.yaml", urlencoding::encode(&args.folder)))
        .expect("Failed to create a file for writing unexpected things");
    let unknowns_yaml = serde_yaml::to_string(&unknowns).expect("Convert unknowns to yaml");
    unexp_file
        .write_all(unknowns_yaml.as_bytes())
        .expect("failed to write unknonwn");

    let mut dups_file = File::create(format!("{}_dups.yaml", urlencoding::encode(&args.folder)))
        .expect("Failed to create a file for writing unexpected things");
    let unknowns_yaml = serde_yaml::to_string(&imgs_to_remove).expect("Convert to_remove to yaml");
    dups_file
        .write_all(unknowns_yaml.as_bytes())
        .expect("failed to write unknonwn");
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
        if let Err(x) = entry {
            println!("{}", x);
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Image {
    path: String,
    name: String,
    sha256: String,
}

impl Image {
    fn new(path: String, sha256: String) -> Self {
        let name = match PathBuf::from(&path).file_name() {
            None => "Unknown".to_owned(),
            Some(name) => name
                .to_str()
                .expect("Failed to convert filename to str")
                .to_owned(),
        };
        Self { path, name, sha256 }
    }
}

fn log_time(msg: &str) {
    println!("{}: {}", Utc::now().to_string(), msg);
}

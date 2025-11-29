use chrono::prelude::*;
use chrono::Utc;
// use clap::Parser;
use indicatif::ProgressIterator;
use nom_exif::Error;
use nom_exif::{
    EntryValue::NaiveDateTime, EntryValue::Time, ExifIter, ExifTag, MediaParser, MediaSource,
};
use serde::{Deserialize, Serialize};
use sha256;
use std::io::prelude::*;
use std::sync::OnceLock;
use std::{cmp::min, collections::HashSet, fs::create_dir_all};

use std::{
    collections::HashMap,
    fs::{self, File},
    path::PathBuf,
    sync::mpsc::channel,
};
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
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Add { name }) => {
            println!("This is add {}", name);
        }
        Some(Commands::List) => {
            println!("This is list");
        }
        None => println!("None"),
    };
    let args = Args::parse();

    let contents = fs::read_to_string("hippie_images.yaml").unwrap();
    let ht: HashMap<String, Vec<Image>> = serde_yaml::from_str(&contents).unwrap();

    let images = ht
        .values()
        .flatten()
        .map(move_to_datetime_folder)
        .progress_count(ht.len() as u64);

    let mut no_exifs: Vec<Image> = vec![];
    let images = images
        .map(|i| {
            if i.path.clone() == "NO_EXIF" {
                no_exifs.push(ht[&i.sha256].first().unwrap().clone());
                None
            } else {
                Some(i)
            }
        })
        .flatten();
    let new_ht: HashMap<String, Image> = images.map(|img| (img.sha256.clone(), img)).collect();

    let yaml = serde_yaml::to_string(&new_ht).expect("Da fuck");
    log_time("Done serializing");
    let mut file = File::create(format!(
        "{}_sorted_images.yaml",
        urlencoding::encode(&args.folder)
    ))
    .expect("Failed to open a file for writing image info");
    file.write_all(yaml.as_bytes())
        .expect("Failed to write image info");

    let no_exif_yaml = serde_yaml::to_string(&no_exifs).expect("Failed to serialize no exifs");
    log_time("Done serializing no exifs");
    let mut file = File::create(format!(
        "{}_no_exif_images.yaml",
        urlencoding::encode(&args.folder)
    ))
    .expect("Failed to open a file for writing image info");
    file.write_all(no_exif_yaml.as_bytes())
        .expect("Failed to write image info");
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

fn move_to_datetime_folder(img: &Image) -> Image {
    let mut parser = MediaParser::new();
    let ms = MediaSource::file_path(&img.path);
    if let Err(e) = ms {
        return img.clone();
    }

    let ms = ms.unwrap();

    let iter_res: Result<ExifIter, Error> = parser.parse(ms);

    let mut candidate_time = Utc::now();

    match iter_res {
        Err(e) => {
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
                .expect(&format!("Failed to move it move it: {}", &img.path));

            Image {
                path: new_img_path,
                ..img.clone()
            }
        }
    }
}

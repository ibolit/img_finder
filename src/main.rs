use clap::Parser;
use serde::{Deserialize, Serialize};
use sha256;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    path::PathBuf,
    sync::{atomic::AtomicUsize, mpsc::channel, Arc, Mutex},
};
use threadpool::ThreadPool;
use walkdir::WalkDir;

static IMGS: &[&str] = &["jpeg", "jpg", "heic", "json"];
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
    let pool = ThreadPool::new(4);
    let (img_tx, img_rx) = channel();
    let mut unknowns: HashMap<String, Vec<String>> = HashMap::new();
    let args = Args::parse();
    println!("{:?}", args);

    // let mut imgs = Mutex::new(0);
    let mut imgs = 0;
    for entry in WalkDir::new(&args.folder) {
        let path = entry.unwrap().into_path();
        // let path = entry.path();
        let ext = extension(&path);
        if let Some(ext) = ext {
            if is_image(&ext) {
                imgs += 1;
                let img_tx = img_tx.clone();
                pool.execute(move || {
                    let data: Vec<u8> = fs::read(&path)
                        .ok()
                        .expect(&format!("Failed to read image {:?}", path));
                    let sha = sha256::digest(data);
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

    println!("After the loop");

    let images = img_rx
        .iter()
        .take(imgs)
        .map(|(path, sha)| Image::new(path, sha))
        .collect::<Vec<Image>>();
    let yaml = serde_yaml::to_string(&images).expect("Da fuck");
    println!("yaml is: {}", yaml);

    println!("Unknowns are: {:?}", unknowns);
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

#[derive(Serialize, Deserialize, Debug)]
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

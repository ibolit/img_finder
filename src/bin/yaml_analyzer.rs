use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Write,
    path::Path,
};

use serde::{Deserialize, Serialize};

fn main() {
    // let paths = fs::read_dir("./").unwrap();
    let mut result: HashMap<String, Vec<Image>> = HashMap::new();
    // for path in paths {
    let path = "%2FUsers%2Fdtv%2FDocuments%2Ffrom%20white%20macbook_images.yaml";

    // if !path
    //     .file_name()
    //     .to_str()
    //     .to_owned()
    //     .unwrap()
    //     .ends_with("DIRTY_images.yaml")
    // // .ends_with("DIRTY_dups.yaml")
    // {
    //     println!("Continuing for path {:?}", path);
    //     continue;
    // }

    println!("Processing: {:?}", path);

    let contents = fs::read_to_string(path).unwrap();
    let ht: HashMap<String, Vec<Image>> = serde_yaml::from_str(&contents).unwrap();

    // for (k, mut v) in ht {
    //     if !result.contains_key(&k) {
    //         result.insert(k, v);
    //         continue;
    //     }
    //     result.get_mut(&k).unwrap().append(v.as_mut());
    // }
    //
    //
    // let dups: Vec<(Image, Vec<Image>)> = serde_yaml::from_str(&contents).unwrap();
    // println!("Moving good files");
    for (sha, imgs) in ht {
        fs::rename(
            imgs[0].path.clone(),
            format!(
                "/Users/dtv/UNIQUE_IMGS/xmp/{}",
                imgs[0].name,
                // urlencoding::encode(&imgs[0].path),
            ),
        )
        .expect(&format!(
            "Failed to move it move it: {}",
            imgs[0].path.clone()
        ));
        println!(
            "mv '{}' '/Volumes/Hippie/UNIQUES_1/{}-{}'",
            imgs[0].path,
            imgs[0].name,
            urlencoding::encode(&imgs[0].path),
        );
    }
    // }

    println!("Moving duplicates");
    // let dup_contents = fs::read_to_string("%2FVolumes%2FHippie%2F_dups.yaml").unwrap();

    // let dups: Vec<(Image, Vec<Image>)> = serde_yaml::from_str(&dup_contents).unwrap();
    // for (img, _) in dups {
    //     fs::rename(
    //         img.path.clone(),
    //         format!(
    //             "/Volumes/Hippie/__DUPES__5/{}-{}",
    //             img.name,
    //             urlencoding::encode(&img.path),
    //         ),
    //     )
    //     .expect(&format!("Failed to move it a dup file: {}", img.path));
    // }

    // let mut imgs_by_folder: HashMap<String, Vec<Image>> = HashMap::new();

    // for (_, imgs) in result {
    //     let is_duplicate = imgs.len() > 1;
    //     if !is_duplicate {
    //         continue;
    //     }

    //     let mut imgs_iter = imgs.iter();
    //     let img = imgs_iter.next().unwrap();
    //     if !fs::exists(&img.path).expect("Exists?") {
    //         panic!("Original file does not exist");
    //     }
    //     println!(": # -- {}", img.path);

    //     for img in imgs_iter {
    //         if is_duplicate {
    //             // if !fs::exists(&img.path).expect("Exists 2?") {
    //             //     continue;
    //             // }

    //             // fs::rename(
    //             //     img.path.clone(),
    //             //     format!(
    //             //         "/Volumes/Hippie/__DUPLICATES__/{}-{}",
    //             //         img.name,
    //             //         urlencoding::encode(&img.path),
    //             //     ),
    //             // )
    //             // .expect("Failed to move it move it");
    //             println!(
    //                 "mv '{}' '/Volumes/Hippie/duplicates/{}-{}'",
    //                 img.path,
    //                 img.name,
    //                 urlencoding::encode(&img.path),
    //             );
    //         }

    //         // img.is_duplicate = is_duplicate;
    //         // let p = Path::new(&img.path).parent().unwrap();
    //         // imgs_by_folder
    //         //     .entry(p.as_os_str().to_str().unwrap().to_owned())
    //         //     .or_insert(vec![])
    //         //     .push(*img);
    //     }
    // }

    // let mut dup_folders: Vec<String> = vec![];
    // for (path, imgs) in imgs_by_folder.clone() {
    //     let all_dups = imgs.iter().fold(true, |acc, img| acc & img.is_duplicate);
    //     // dup_folders.insert(path, all_dups);
    //     if all_dups {
    //         // println!("{}", path);
    //         dup_folders.push(path);
    //     }
    // }

    // let mut seen_folders: HashSet<String> = HashSet::new();

    // for dup_folder in dup_folders.iter() {
    //     if seen_folders.contains(dup_folder) {
    //         continue;
    //     }
    // let this_set = shas(&imgs_by_folder[dup_folder].clone());
    // seen_folders.insert(dup_folder.to_owned());

    // for second_folder in dup_folders.iter() {
    //     if seen_folders.contains(second_folder) {
    //         continue;
    //     }
    //     let second_set = shas(&imgs_by_folder[second_folder].clone());
    //     if this_set == second_set {
    //         println!("{dup_folder} == {second_folder}");
    //         seen_folders.insert(second_folder.to_owned());
    //     }
    // }
    // }

    // println!("{:?}", dup_folders);

    // let mut out_file = File::create("merged.yaml").unwrap();
    // let serialized = serde_yaml::to_string(&result).unwrap();
    // out_file.write_all(serialized.as_bytes()).unwrap();
}

fn shas(imgs: &Vec<Image>) -> HashSet<String> {
    imgs.iter().map(|img| img.sha256.clone()).collect()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Image {
    path: String,
    name: String,
    sha256: String,
    #[serde(default)]
    is_duplicate: bool,
}

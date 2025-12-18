use std::path::Path;

pub struct FileFactory {
    image_formats: Vec<String>,
    known_formats: Vec<String>,
}

impl<'a> FileFactory {
    fn new(image_formats: Vec<String>, known_formats: Vec<String>) -> Self {
        FileFactory {
            image_formats: image_formats,
            known_formats: known_formats,
        }
    }

    pub fn from_path(&self, path: &Path) -> File {
        if path.is_symlink() {
            return File::SymLink(path.into());
        }
        if path.is_dir() {
            return File::Dir(path.into());
        }
        let ext = extension(&path);
        if let Some(ext) = ext {
            if is_image(&ext, &self.image_formats) {
                return File::Image(path.into());
            } else if !is_known(&ext, &self.known_formats) {
                return File::Unknown(path.into());
            } else {
                return File::Known(path.into());
            }
        } else {
            return File::Unknown(path.into());
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
    pub fn factory<'a>(image_formats: Vec<String>, known_formats: Vec<String>) -> FileFactory {
        FileFactory::new(image_formats, known_formats)
    }
}

fn extension(entry: &Path) -> Option<String> {
    Some(entry.extension()?.to_str()?.to_lowercase())
}

fn is_image(ext: &str, img_formats: &[String]) -> bool {
    img_formats.contains(&ext.to_owned())
}

fn is_known(ext: &str, known_formats: &[String]) -> bool {
    known_formats.contains(&ext.to_owned())
}

pub struct FileIterator {}
impl Iterator for FileIterator {
    type Item = File;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
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

        fn symlink(x: &Path) -> File {
            File::SymLink(x.into())
        }
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

use std::{
    fs,
    io::Result,
    path::{Path, PathBuf},
};

use crate::platform::FileSystemTrait;

#[derive(Default)]
pub struct FileSystem;

impl FileSystemTrait for FileSystem {
    type ReadDir = ReadDir;

    fn read_file(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path)
    }

    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        fs::read_dir(path).map(ReadDir)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        fs::rename(from, to)
    }

    fn write(&self, path: &Path, contents: &str) -> Result<()> {
        fs::write(path, contents)
    }
    
    fn delete(&self, path: &Path) -> std::io::Result<()> {
        if path.is_file() {
            fs::remove_file(path)
        } else {
            fs::remove_dir_all(path)
        }
    }
}

pub struct ReadDir(fs::ReadDir);

impl Iterator for ReadDir {
    type Item = Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|dir| Ok(dir?.path()))
    }
}
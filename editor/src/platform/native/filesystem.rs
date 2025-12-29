use std::{
    fs,
    io::Result,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::platform::{FileSystemTrait, Project, SearchResult};

#[derive(Default)]
pub struct FileSystem;

impl FileSystem {
    fn get_line_col(contents: &str, offset: usize) -> (usize, usize) {
        let line = contents[..offset].lines().count();
        let col = offset - contents[..offset].rfind('\n').unwrap_or(0);

        (line, col)
    }
}

impl FileSystemTrait for FileSystem {
    type ReadDir = ReadDir;

    fn read_file(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path)
    }

    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        fs::read_dir(path).map(ReadDir)
    }

    fn search_project(&self, project: &Project, pattern: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        for entry in WalkDir::new(&project.path).into_iter().filter_map(|e| e.ok()) {
            if let Ok(contents) = fs::read_to_string(entry.path()) {
                let mut start_index = 0;
                while let Some(pos) = contents[start_index..].find(pattern) {
                    let byte_offset = start_index + pos;
                    let (line, col) = Self::get_line_col(&contents, byte_offset);

                    results.push(SearchResult {
                        path: entry.path().to_path_buf(),
                        line,
                        col,
                    });

                    start_index = byte_offset + pattern.len();
                }
            }
        }
        results
    }

    fn replace(&self, paths: &[&Path], pattern: &str, replace: &str) -> Result<()> {
        for path in paths {
            let contents = fs::read_to_string(path)?.replace(pattern, replace);
            fs::write(path, contents)?;
        }

        Ok(())
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

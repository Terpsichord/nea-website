use super::{BackendHandle, PendingOperations, WebSocketHandle};
use crate::platform::FileSystemTrait;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
    vec::IntoIter,
};
use ws_messages::{Command, ProjectTree};

#[derive(Default, Debug)]
pub struct FileSystem {
    handle: BackendHandle,
    cached_dirs: HashMap<PathBuf, Vec<PathBuf>>,
}

impl FileSystem {
    pub fn new(handle: BackendHandle) -> Self {
        Self { handle, cached_dirs: HashMap::new() }
    }

    pub fn cache(&mut self, tree: ProjectTree) {
        if let ProjectTree::Directory { path, children } = tree {
            let child_paths = children.iter().map(|c| c.path()).cloned().collect();
            self.cached_dirs.insert(path, child_paths);        
            for child in children {
                self.cache(child);
            }
        }
    }

    pub fn get_cached(&self, path: &Path) -> Option<ReadDir> {
        self.cached_dirs
            .get(path)
            .map(|p| ReadDir::new(p.clone().into_iter().map(Ok).collect()))
    }
}

impl FileSystemTrait for FileSystem {
    type ReadDir = ReadDir;

    fn read_file(&self, path: &Path) -> Result<String> {
        self.handle.send(Command::ReadFile { path: path.into() });

        Err(ErrorKind::WouldBlock)?
    }

    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
        // info!("reading dir: {}", path.display());
        // info!("looking in cache: {}", self.cached_dirs.keys().map(|p| p.display()).collect::<Vec<_>>().join(", ")););
        if let Some(read_dir) =  self.get_cached(path) {
            return Ok(read_dir);
        }

        self.handle.send(Command::ReadDir { path: path.into() });

        Err(ErrorKind::WouldBlock)?
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.handle.send(Command::Rename {
            from: from.into(),
            to: to.into(),
        });

        Err(ErrorKind::WouldBlock)?
    }

    fn write(&self, path: &Path, contents: &str) -> Result<()> {
        self.handle.send(Command::WriteFile {
            path: path.into(),
            contents: contents.into(),
        });

        Err(ErrorKind::WouldBlock)?
    }

    fn delete(&self, path: &Path) -> Result<()> {
        self.handle.send(Command::Delete { path: path.into() });

        Err(ErrorKind::WouldBlock)?
    }
}

#[derive(Debug)]
pub struct ReadDir(IntoIter<Result<PathBuf>>);

impl ReadDir {
    pub fn new(paths: Vec<Result<PathBuf>>) -> Self {
        Self(paths.into_iter())
    }
}

impl Iterator for ReadDir {
    type Item = Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

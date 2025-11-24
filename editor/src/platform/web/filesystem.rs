use super::{BackendHandle, PendingOperations, WebSocketHandle};
use crate::platform::FileSystemTrait;
use std::{
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
    vec::IntoIter,
};
use ws_messages::Command;

#[derive(Default)]
pub struct FileSystem {
    handle: BackendHandle,
}

impl FileSystem {
    pub fn new(handle: BackendHandle) -> Self {
        Self { handle }
    }
}

impl FileSystemTrait for FileSystem {
    type ReadDir = ReadDir;

    fn read_file(&self, path: &Path) -> Result<String> {
        self.handle.send(Command::ReadFile { path: path.into() });

        Err(ErrorKind::WouldBlock)?
    }

    fn read_dir(&self, path: &Path) -> Result<ReadDir> {
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

pub struct ReadDir(IntoIter<Result<PathBuf>>);

impl Iterator for ReadDir {
    type Item = Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

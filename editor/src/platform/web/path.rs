use serde::{Serialize, Serializer};
use std::{borrow::Cow, ops::Deref};
use typed_path::{Utf8UnixPath, Utf8UnixPathBuf};

// TODO: I think?? delete this file (or at least stop using typed_path, and use std::path instead)
 
#[derive(Debug)]
#[repr(transparent)]
pub struct Path(Utf8UnixPath);

impl Deref for Path {
    type Target = Utf8UnixPath;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Path {
    pub fn to_str(&self) -> &str {
        &self.0
    }
}


#[derive(Clone, Debug, Default)]
pub struct PathBuf(Utf8UnixPathBuf);

impl PathBuf {
    pub fn is_file(&self) -> bool {
        // FIXME: this is kind of hacky
        self.to_string().map_or_default(|s| s.ends_with('/'))
    }
}

impl Deref for PathBuf {
    type Target = Utf8UnixPathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for PathBuf {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(s)
    }
}

impl PathBuf {
    pub fn file_name(&self) -> Option<ToCow> {
        self.file_name().map(ToCow)
    }

    pub fn extension(&self) -> Option<ToCow> {
        self.extension().map(ToCow)
    }
}

#[derive(Default)]
struct ToCow(String);
impl ToCow {
    pub fn to_str(&self) -> Option<&str> {
        Some(&self.0)
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

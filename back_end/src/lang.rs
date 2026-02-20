use std::{fmt::Display, fs, io, str::FromStr};

use anyhow::bail;
use serde::{Deserialize, Serialize};
use sqlx::{Decode, Postgres, error::BoxDynError, postgres::PgValueRef};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ProjectLang {
    #[serde(rename = "py")]
    Python,
    #[serde(rename = "js")]
    JavaScript,
    #[serde(rename = "ts")]
    TypeScript,
    #[serde(rename = "rs")]
    Rust,
    #[serde(rename = "c")]
    C,
    #[serde(rename = "cpp")]
    CPlusPlus,
    #[serde(rename = "cs")]
    CSharp,
    #[serde(rename = "sh")]
    Bash,
    #[serde(rename = "java")]
    Java,
}

impl Display for ProjectLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::Rust => "rs",
            Self::C => "c",
            Self::CPlusPlus => "cpp",
            Self::CSharp => "cs",
            Self::Bash => "sh",
            Self::Java => "java",
        })
    }
}

impl FromStr for ProjectLang {
    type Err = anyhow::Error;

    fn from_str(lang: &str) -> Result<Self, Self::Err> {
        Ok(match lang {
            "py" => Self::Python,
            "js" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "rs" => Self::Rust,
            "c" => Self::C,
            "cpp" => Self::CPlusPlus,
            "cs" => Self::CSharp,
            "sh" => Self::Bash,
            "java" => Self::Java,
            _ => bail!("Invalid language"),
        })
    }
}

impl<'r> Decode<'r, Postgres> for ProjectLang {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let string = <&str as Decode<Postgres>>::decode(value)?;
        Ok(string.parse()?)
    }
}

impl ProjectLang {
    const LANG_PATH: &'static str = "./back_end/languages";

    pub fn get_project_toml(self) -> io::Result<String> {
        fs::read_to_string(format!("{}/{}/project.toml", Self::LANG_PATH, self))
    }

    pub fn get_initial_file(self) -> io::Result<(&'static str, String)> {
        let name = match self {
            Self::Python => "main.py",
            Self::JavaScript => "main.js",
            Self::TypeScript => "main.ts",
            Self::C => "main.c",
            Self::CPlusPlus => "main.cpp",
            Self::Bash => "main.sh",
            Self::Java => "main.java",
            // these languages have readmes with instructions on how to get started
            Self::Rust | Self::CSharp => "README.md",
        };

        let content = fs::read_to_string(format!("{}/{}/init", Self::LANG_PATH, self))?;

        Ok((name, content))
    }
}

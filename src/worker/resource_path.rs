use std::{fmt::Display, path::PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum ResourcePath {
    Local(PathBuf),
    URL(String),
}

impl Display for ResourcePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ResourcePath::*;
        match self {
            Local(path) => f.write_fmt(format_args!("[Local resource: {}]", path.display())),
            URL(url) => f.write_fmt(format_args!("[URL: {url}]")),
        }
    }
}

const URL_REGEX_SPEC: &str = r"^(http|https)://(.+)$";

impl From<String> for ResourcePath {
    fn from(value: String) -> Self {
        static URL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(URL_REGEX_SPEC).unwrap());

        use ResourcePath::*;

        if URL_REGEX.is_match(&value) {
            URL(value)
        } else {
            Local(value.into())
        }
    }
}

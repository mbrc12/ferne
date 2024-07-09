use std::{fmt::Display, path::PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::worker::{LoadResponse, TemplateRegistry};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum ResourcePath {
    Local(PathBuf),
    URL(String),
    GitHub(String),
}

impl Display for ResourcePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ResourcePath::*;
        match self {
            Local(path) => f.write_fmt(format_args!("[Local resource: {}]", path.display())),
            URL(url) => f.write_fmt(format_args!("[URL: {url}]")),
            GitHub(url) => f.write_fmt(format_args!("[GitHub: {url}]")),
        }
    }
}

// match lines of type "--- name  : hello_world  " and extract "hello_world"
const NAME_REGEX_SPEC: &str = r"^---\s*name\s*:\s*([a-zA-Z0-9_]+)\s*$";

// Split template file into parts
// Each partial starts with a header line that looks like --- name: foobar
pub async fn register_template(data: String, hb_registry: TemplateRegistry) -> LoadResponse {
    static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(NAME_REGEX_SPEC).unwrap());

    let lines = data.lines().collect::<Vec<_>>();

    let mut starts = vec![]; // (name, line_index)

    for (idx, line) in lines.iter().enumerate() {
        let potential_match = NAME_REGEX.captures_at(line, 0);

        if let Some(captures) = potential_match {
            if captures.len() != 1 {
                // there should be exactly one match
                return Err("Failed to parse template".to_string());
            }

            let name = captures.get(0).unwrap(); // has to succeed since regex has 1 group;
            starts.push((name.as_str(), idx))
        }
    }

    // Its time to write to the registry
    let mut registry_write = hb_registry.write().await;

    for idx in 0..starts.len() {
        let name = starts[idx].0;

        let end = if idx == starts.len() {
            lines.len()
        } else {
            starts[idx + 1].1
        };

        let mut buf = String::new();
        for line_idx in idx..end {
            buf.push_str(lines[line_idx]);
        }

        registry_write
            .register_partial(name, buf)
            .map_err(|_| "Failed to register template".to_string())?
    }

    Ok(())
}

const URL_REGEX_SPEC: &str = r"^(http|https)://(.+)$";
const GITHUB_REGEX_SPEC: &str = r"^(?:github\.com|http://github\.com|https://github\.com)/(.+)$";

impl From<String> for ResourcePath {
    fn from(value: String) -> Self {
        static URL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(URL_REGEX_SPEC).unwrap());
        static GITHUB_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(GITHUB_REGEX_SPEC).unwrap());

        use ResourcePath::*;

        if URL_REGEX.is_match(&value) {
            let github_match = GITHUB_REGEX.captures(&value);
            if let Some(captures) = github_match {
                // github url, extract the repository path and use it as the Github URL
                GitHub(captures.get(0).unwrap().as_str().to_string())
            } else {
                URL(value)
            }
        } else {
            Local(value.into())
        }
    }
}

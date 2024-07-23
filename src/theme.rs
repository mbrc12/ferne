use std::sync::Arc;

use anyhow::Context;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::{Mutex, RwLock};

use crate::worker::SubmitQueue;

#[derive(Clone)]
pub struct TemplateRegistry {
    queue: SubmitQueue,
    hb: Arc<RwLock<Handlebars<'static>>>,
    next_tag_idx: Arc<Mutex<u64>>,
}

pub type TemplateTag = String;

// match lines of type "--- name  : hello_world  " and extract "hello_world"
const NAME_REGEX_SPEC: &str = r"^---\s*name\s*:\s*([a-zA-Z0-9_]+)\s*$";

impl TemplateRegistry {
    pub fn new(queue: SubmitQueue) -> Self {
        TemplateRegistry {
            queue,
            hb: Arc::new(RwLock::new(Handlebars::new())),
            next_tag_idx: Arc::new(Mutex::new(0)),
        }
    }

    // Load a template file, split into parts, and register it.
    // Each partial starts with a header line that looks like --- name: foobar
    // Then this loads all the found templates into the registry
    pub async fn load_template(
        self: Self,
        tag: Option<TemplateTag>,
        path: String,
    ) -> anyhow::Result<TemplateTag> {
        static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(NAME_REGEX_SPEC).unwrap());

        // Use provided tag or produce a new tag
        let tag: TemplateTag = {
            if let Some(tag_) = tag {
                tag_
            } else {
                let mut idx_lock = self.next_tag_idx.lock().await;
                *idx_lock += 1;
                format!("theme-{}", idx_lock)
            }
        };

        let data = self.queue.submit(path).await.get().await.clone(); // clone the result string

        let lines = data.lines().collect::<Vec<_>>();

        let mut starts = vec![]; // (name, line_index)

        for (idx, line) in lines.iter().enumerate() {
            let potential_match = NAME_REGEX.captures_at(line, 0);

            if let Some(captures) = potential_match {
                if captures.len() != 1 {
                    // there should be exactly one match
                    anyhow::bail!("Failed to parse template!");
                }

                let name = captures.get(0).unwrap(); // has to succeed since regex has 1 group;
                starts.push((name.as_str(), idx))
            }
        }

        // Parsing finished, its time to write to the registry
        let mut hb_write = self.hb.write().await;

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

            hb_write
                .register_partial(&format!("{}:{}", tag, name), buf)
                .context("Failed to register template to registry!")?
        }

        Ok(tag) // write lock dropped here
    }
}

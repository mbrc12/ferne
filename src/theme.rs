use std::sync::Arc;

use anyhow::{Context, Result};
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::{Mutex, RwLock};

use crate::{walker::RouteConfig, worker::SubmitQueue};

// default slot used for main body of the content,
// sync this with BASE_TEMPLATE_CONTENTS below
pub const CONTENT_SLOT: &str = "__content__";

// template which does nothing except show the raw content
pub const BASE_TEMPLATE_NAME: &str = "__BASE__";
pub const BASE_TEMPLATE_CONTENTS: &str = "{{{__content__}}}";

#[derive(Clone, Debug)]
pub struct TemplateRegistry {
    queue: SubmitQueue,
    hb: Arc<RwLock<Handlebars<'static>>>,
    next_tag_idx: Arc<Mutex<u64>>,
}

// match lines of type "--- name  : hello_world  " and extract "hello_world"
const NAME_REGEX_SPEC: &str = r"^---\s*name\s*:\s*([a-zA-Z0-9_]+)\s*$";

impl TemplateRegistry {
    pub fn new(queue: SubmitQueue) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.register_partial(BASE_TEMPLATE_NAME, BASE_TEMPLATE_CONTENTS)?;

        Ok(TemplateRegistry {
            queue,
            hb: Arc::new(RwLock::new(hb)),
            next_tag_idx: Arc::new(Mutex::new(0)),
        })
    }

    pub async fn has_template(self: &Self, name: &str) -> bool {
        let hb = self.hb.read().await;
        return hb.has_template(name);
    }

    // Load a template file, split into parts, and register it.
    // Each partial starts with a header line that looks like --- name: foobar
    // Then this loads all the found templates into the registry
    pub async fn load_template(self: Self, name: Option<String>, path: String) -> Result<String> {
        static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(NAME_REGEX_SPEC).unwrap());

        // Use provided tag or produce a new tag
        let name: String = {
            if let Some(name_) = name {
                if self.has_template(&name_).await {
                    anyhow::bail!(
                        "Template with name `{}` already present in registry!",
                        name_
                    )
                }
                name_
            } else {
                let mut idx_lock = self.next_tag_idx.lock().await;
                *idx_lock += 1;
                format!("theme-{}", idx_lock)
            }
        };

        let data = self.queue.submit(path.clone()).await?.get().await.clone(); // clone the result string

        let lines = data.lines().collect::<Vec<_>>();

        let mut starts = vec![]; // (name, line_index)

        for (idx, line) in lines.iter().enumerate() {
            let potential_match = NAME_REGEX.captures_at(line, 0);

            if let Some(captures) = potential_match {
                if captures.len() != 1 {
                    // there should be exactly one match
                    anyhow::bail!("Failed to parse template `{}`.", path);
                }

                let name = captures.get(0).unwrap(); // has to succeed since regex has 1 group;
                starts.push((name.as_str(), idx))
            }
        }

        // Parsing finished, its time to write to the registry
        let mut hb_write = self.hb.write().await;

        for idx in 0..starts.len() {
            let id = starts[idx].0;

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
                .register_partial(&format!("{}:{}", name, id), buf)
                .context(format!("Failed to register template {} to registry!", path))?
        }

        Ok(name) // write lock dropped here
    }

    pub async fn render_template(
        self: Self,
        content: &str,
        config: &RouteConfig,
    ) -> Result<String> {
        let TemplateRegistry { hb, .. } = self;
        let name = config.theme.name.to_owned();

        let hb = hb.read().await;

        // render the markdown using the configuration (other than theme)
        let md_as_html = hb.render_template(content, &config.rest)?;

        // then render the theme with the rendered markdown as content
        // first copy the theme config, and insert the content
        // use that as the data to render the template
        let mut config_with_content = config.theme.rest.clone();
        config_with_content.insert(CONTENT_SLOT.to_owned(), toml::Value::String(md_as_html));

        let rendered = hb.render(&name, &config_with_content)?;

        Ok(rendered) // read lock dropped here
    }
}

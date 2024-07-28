use std::sync::Arc;

use anyhow::{Context, Result};
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::{Mutex, RwLock};

use crate::{
    qualified_partial,
    util::{self, theme_names::sanitize_name},
    walker::{RouteConfig, ThemeConfig},
    worker::SubmitQueue,
};

// A theme name is called a "name", a partial name is called a "kind"

// default slot used for main body of the content,
// sync this with BASE_MAIN_CONTENTS below
pub const CONTENT_SLOT: &str = "__content__";

// The default kind used
pub const MAIN_KIND: &str = "main";

// default template name with one kind
pub const BASE_NAME: &str = "__BASE__";

// template which does nothing except show the raw content
pub const BASE_MAIN_CONTENTS: &str = r"{{{__content__}}}";

// match lines of type "--- name  : hello_world  " and extract "hello_world"
const NAME_REGEX_SPEC: &str = r"^---\s*name\s*:\s*([a-zA-Z0-9_]+)\s*$";

#[derive(Clone, Debug)]
pub struct TemplateRegistry {
    queue: SubmitQueue,
    hb: Arc<RwLock<Handlebars<'static>>>,
    next_tag_idx: Arc<Mutex<u64>>,
}

impl TemplateRegistry {
    pub fn new(queue: SubmitQueue) -> Result<Self> {
        let mut hb = Handlebars::new();
        hb.register_partial(
            &qualified_partial!(BASE_NAME, MAIN_KIND),
            BASE_MAIN_CONTENTS,
        )?;

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

        // Use provided name or produce a new name for the theme
        let name: String = sanitize_name({
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
        })?;

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

                // has to succeed since regex has 1 group;
                let partial = sanitize_name(captures.get(0).unwrap().as_str().to_owned())?;
                starts.push((partial, idx))
            }
        }

        // Parsing finished, its time to write to the registry
        let mut hb_write = self.hb.write().await;

        for idx in 0..starts.len() {
            let id = starts[idx].0.clone();

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
                .register_partial(&qualified_partial!(name, id), buf)
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
        let ThemeConfig {
            ref name,
            ref kind,
            rest: ref theme_rest,
        } = config.theme;

        let hb = hb.read().await;

        // render the markdown using the configuration (other than theme)
        let markdown = hb.render_template(content, &config.rest)?;

        // convert markdown to html
        let md_as_html = util::markdown::to_html(&markdown);

        // then render the theme with the rendered markdown as content
        // first copy the theme config, and insert the content
        // use that as the data to render the template
        let mut config_with_content = theme_rest.clone();
        config_with_content.insert(CONTENT_SLOT.to_owned(), toml::Value::String(md_as_html));

        let rendered = hb.render(&qualified_partial!(name, kind), &config_with_content)?;

        Ok(rendered) // read lock dropped here
    }
}

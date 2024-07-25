use anyhow::Result;

use crate::{
    assert_toml_kind,
    theme::{TemplateRegistry, BASE_TEMPLATE_NAME},
    util,
};

const THEME_PATH_KEY: &str = "path";
const THEME_NAME_KEY: &str = "name";
const THEME_TABLE_KEY: &str = "theme";

#[derive(Clone, Debug)]
pub struct Route {
    pub config: RouteConfig,
    pub details: RouteDetails,
}

#[derive(Clone, Debug)]
pub struct RouteConfig {
    theme: ThemeConfig,

    rest: toml::Table,
}

#[derive(Clone, Debug)]
pub struct ThemeConfig {
    name: String, // this is used as the TemplateName

    rest: toml::Table,
}

#[derive(Clone, Debug)]
pub enum RouteDetails {
    Dir(DirectoryRoute),
    File(FileRoute),
}

#[derive(Clone, Debug)]
pub struct DirectoryRoute {
    pub children: Vec<Route>,
}

#[derive(Clone, Debug)]
pub struct FileRoute {
    pub markdown: String,
    pub html: String,

    pub partial_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RouteContext {
    pub registry: TemplateRegistry,
    pub config: RouteConfig,
}

impl ThemeConfig {
    fn new() -> Self {
        ThemeConfig {
            name: BASE_TEMPLATE_NAME.to_string(),
            rest: toml::Table::new(),
        }
    }

    async fn from_toml_with_context(mut table: toml::Table, context: RouteContext) -> Result<Self> {
        // if theme_path is present, load it. if there is a theme_name, use that for the name
        // if there is a conflict, error (which is done in the load_template function)
        // if theme_path is absent, check if theme_name is in the registry, else error

        let theme_name_raw = assert_toml_kind!(String; table, THEME_NAME_KEY)?;
        let theme_path = assert_toml_kind!(String; table, THEME_PATH_KEY)?;

        table.remove(THEME_NAME_KEY);
        table.remove(THEME_PATH_KEY);

        let name = {
            if let Some(path) = theme_path {
                context.registry.load_template(theme_name_raw, path).await?
            } else {
                context.config.theme.name
            }
        };

        let rest = util::toml::merge(context.config.theme.rest, table)?;

        Ok(ThemeConfig { name, rest })
    }
}

impl RouteConfig {
    pub fn new() -> Self {
        RouteConfig {
            theme: ThemeConfig::new(),
            rest: toml::Table::new(),
        }
    }

    async fn from_toml_with_context(mut table: toml::Table, context: RouteContext) -> Result<Self> {
        // Extract out the theme table, and use ThemeConfig to build it
        // For the rest, do a simple merge
        let theme_table =
            assert_toml_kind!(Table; table, THEME_TABLE_KEY)?.unwrap_or(toml::Table::new());

        let theme = ThemeConfig::from_toml_with_context(theme_table, context.clone()).await?;

        table.remove(THEME_TABLE_KEY);

        let rest = util::toml::merge(context.config.rest, table);

        Ok(RouteConfig { theme, rest })
    }
}

impl FileRoute {
    pub async fn from_config_and_content(config: RouteConfig, content: String) -> Self {}
}

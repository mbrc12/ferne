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

// Use Cow on this at some point
#[derive(Clone, Debug)]
pub struct RouteConfig {
    pub theme: ThemeConfig,

    pub rest: toml::Table,
}

#[derive(Clone, Debug)]
pub struct ThemeConfig {
    pub name: String, // this is used as the TemplateName

    pub rest: toml::Table,
}

impl Default for RouteConfig {
    fn default() -> Self {
        RouteConfig {
            theme: ThemeConfig {
                name: BASE_TEMPLATE_NAME.to_string(),
                rest: toml::Table::new(),
            },
            rest: toml::Table::new(),
        }
    }
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
    pub html: String,
}

// Context for building a route
#[derive(Clone, Debug)]
pub struct RouteContext {
    pub registry: TemplateRegistry,
    pub config: RouteConfig,
}

impl RouteContext {
    async fn theme_config_from_toml(self: Self, mut table: toml::Table) -> Result<ThemeConfig> {
        // if theme_path is present, load it. if there is a theme_name, use that for the name
        // if there is a conflict, error (which is done in the load_template function)
        // if theme_path is absent, check if theme_name is in the registry, else error

        let theme_name_raw = assert_toml_kind!(String; table, THEME_NAME_KEY)?;
        let theme_path = assert_toml_kind!(String; table, THEME_PATH_KEY)?;

        table.remove(THEME_NAME_KEY);
        table.remove(THEME_PATH_KEY);

        let name = {
            if let Some(path) = theme_path {
                self.registry.load_template(theme_name_raw, path).await?
            } else {
                self.config.theme.name
            }
        };

        let rest = util::toml::merge(self.config.theme.rest, table)?;

        Ok(ThemeConfig { name, rest })
    }

    pub async fn route_config_from_toml(self: Self, mut table: toml::Table) -> Result<RouteConfig> {
        // Extract out the theme table, and use ThemeConfig to build it
        // For the rest, do a simple merge
        let theme_table =
            assert_toml_kind!(Table; table, THEME_TABLE_KEY)?.unwrap_or(toml::Table::new());

        let theme = self.clone().theme_config_from_toml(theme_table).await?;

        table.remove(THEME_TABLE_KEY);

        let rest = util::toml::merge(self.config.rest, table)?;

        Ok(RouteConfig { theme, rest })
    }

    pub async fn file_route_from_content(self: Self, content: String) -> Result<FileRoute> {
        let RouteContext { registry, config } = self;

        let html = registry.render_template(&content, &config).await?;
        Ok(FileRoute { html })
    }
}

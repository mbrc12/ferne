use anyhow::Result;

use crate::{
    assert_toml_kind,
    theme::{TemplateRegistry, BASE_NAME, MAIN_KIND},
    util,
};

const THEME_PATH_KEY: &str = "path";
const THEME_NAME_KEY: &str = "name";
const THEME_TABLE_KEY: &str = "theme";
const PARTIAL_KEY: &str = "kind";

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
    pub kind: String, // this is used as the partial name

    pub rest: toml::Table,
}

impl Default for RouteConfig {
    fn default() -> Self {
        RouteConfig {
            theme: ThemeConfig {
                name: BASE_NAME.to_string(),
                kind: MAIN_KIND.to_string(),
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
        // inherit old partial name if not present, otherwise use the new partial name
        // if theme_path is present, load it. if there is a theme_name, use that for the name
        // if there is a conflict, error (which is done in the load_template function)
        // if theme_path is absent, check if theme_name is in the registry, else error

        let name_raw = assert_toml_kind!(String; table, THEME_NAME_KEY)?;
        let theme_path = assert_toml_kind!(String; table, THEME_PATH_KEY)?;
        let kind_raw = assert_toml_kind!(String; table, PARTIAL_KEY)?;

        table.remove(THEME_NAME_KEY);
        table.remove(THEME_PATH_KEY);
        table.remove(PARTIAL_KEY);

        let name = {
            if let Some(path) = theme_path {
                self.registry.load_template(name_raw, path).await?
            } else {
                self.config.theme.name
            }
        };

        let kind = {
            if let Some(kind) = kind_raw {
                kind
            } else {
                MAIN_KIND.to_owned()
            }
        };

        let rest = util::toml::merge(self.config.theme.rest, table)?;

        Ok(ThemeConfig { name, kind, rest })
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

    pub async fn merge_toml(self: Self, table: toml::Table) -> Result<Self> {
        Ok(RouteContext {
            registry: self.registry.clone(),
            config: self.route_config_from_toml(table).await?,
        })
    }

    pub async fn file_route_from_content(self: &Self, content: String) -> Result<FileRoute> {
        let RouteContext { registry, config } = self;

        let html = registry.clone().render_template(&content, &config).await?;
        Ok(FileRoute { html })
    }
}

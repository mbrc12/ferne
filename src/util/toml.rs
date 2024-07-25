use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::{fatal, fatal_if_err};

const REPLACE_FLAG: &str = "__replace__";

// assert that a toml key is a string, return error if not
#[macro_export]
macro_rules! assert_toml_kind {
    ($kind: tt; $table: expr, $key: expr) => {{
        if let Some(value) = $table.get($key) {
            if let toml::Value::$kind(value_) = value {
                anyhow::Ok(Some(value_.clone()))
            } else {
                anyhow::bail!("Key `{}` has the wrong type!", $key)
            }
        } else {
            Ok(None)
        }
    }};
}

/// Recursively merge toml tables, but ensure that the overlay table is of higher priority in the
/// merging. Arrays and tables are merged by default, and replaced only if the entire table has the
/// __replace__ key set as __replace__=true
pub fn merge(mut base: toml::Table, overlay: toml::Table) -> Result<toml::Table> {
    let mut replace = false;
    if overlay.contains_key(REPLACE_FLAG) {
        replace = true;
    }

    for (key, value) in overlay {
        if let Some(previous_value) = base.get_mut(&key) {
            use toml::Value::*;

            match previous_value {
                String(_) | Integer(_) | Float(_) | Boolean(_) | Datetime(_) => {
                    // TODO: this ignores the type of value, but should raise an error
                    // if the types are inconsistent
                    *previous_value = value
                }

                Array(array) => {
                    if replace {
                        *previous_value = value;
                    } else {
                        if let Array(mut overlay_array) = value {
                            array.extend(overlay_array.drain(..));
                        } else {
                            anyhow::bail!("Type error during array merge!");
                        }
                    }
                }

                Table(table) => {
                    if replace {
                        if let Table(overlay_table) = value {
                            *previous_value = Table(merge(table.clone(), overlay_table))
                        } else {
                            anyhow::bail!("Type error during table merge!");
                        }
                    } else {
                        *previous_value = value;
                    }
                }
            }
        } else {
            base.insert(key, value);
        }
    }

    Ok(base)
}

pub async fn read(path: &PathBuf) -> Result<toml::Table> {
    // Fails only if toml is provided but fails to parse. Missing file just returns an
    // empty table.

    let contents = tokio::fs::read_to_string(path)
        .await
        .unwrap_or("".to_owned());

    toml::from_str::<toml::Table>(&contents).context(format!(
        "Failed to parse toml in file `{}`.",
        path.display()
    ))
}

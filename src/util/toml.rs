use std::path::PathBuf;

use anyhow::Context;

/// Recursively merge toml tables, but ensure that the overlay table is of higher priority in the
/// merging
pub fn merge(mut base: toml::Table, overlay: toml::Table) -> anyhow::Result<toml::Table> {
    for (key, value) in overlay {
        if let Some(previous_value) = base.get_mut(&key) {
            use toml::Value::*;
            match previous_value {
                String(_) | Integer(_) | Float(_) | Boolean(_) | Datetime(_) | Array(_) => {
                    // TODO: this ignores the type of value, but should raise an error
                    // if the types are inconsistent
                    *previous_value = value
                }

                Table(table) => {
                    if let Table(overlay_table) = value {
                        *previous_value = Table(merge(table.clone(), overlay_table)?)
                    } else {
                        return Err(anyhow::anyhow!("Type error during merge!"));
                    }
                }
            }
        } else {
            base.insert(key, value);
        }
    }

    Ok(base)
}

pub async fn read(path: &PathBuf) -> anyhow::Result<toml::Table> {
    // Fails only if toml is provided but fails to parse. Missing file just returns an
    // empty table.

    let path_str = path.display();

    let contents = tokio::fs::read_to_string(path)
        .await
        .unwrap_or("".to_owned());

    toml::from_str::<toml::Table>(&contents)
        .context(format!("Failed to parse toml in file {}.", path_str))
}

#[macro_export]
macro_rules! qualified_partial {
    ($name: expr, $partial: expr) => {
        format!("{}:{}", $name, $partial)
    };
}

pub fn sanitize_name(str: String) -> anyhow::Result<String> {
    let invalid = str.chars().any(|c| c == ':');
    if invalid {
        anyhow::bail!("Theme/kind name `{}` is invalid!", str)
    } else {
        Ok(str)
    }
}

// use once_cell::sync::Lazy;
// use regex::Regex;
//
// use crate::worker::{LoadResponse, TemplateRegistry};

// match lines of type "--- name  : hello_world  " and extract "hello_world"
// const NAME_REGEX_SPEC: &str = r"^---\s*name\s*:\s*([a-zA-Z0-9_]+)\s*$";
//
// // Split template file into parts
// // Each partial starts with a header line that looks like --- name: foobar
// pub async fn register_template(
//     tag: String,
//     data: String,
//     hb_registry: TemplateRegistry,
// ) -> LoadResponse {
//     static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(NAME_REGEX_SPEC).unwrap());
//
//     let lines = data.lines().collect::<Vec<_>>();
//
//     let mut starts = vec![]; // (name, line_index)
//
//     for (idx, line) in lines.iter().enumerate() {
//         let potential_match = NAME_REGEX.captures_at(line, 0);
//
//         if let Some(captures) = potential_match {
//             if captures.len() != 1 {
//                 // there should be exactly one match
//                 return Err("Failed to parse template".to_string());
//             }
//
//             let name = captures.get(0).unwrap(); // has to succeed since regex has 1 group;
//             starts.push((name.as_str(), idx))
//         }
//     }
//
//     // Its time to write to the registry
//     let mut registry_write = hb_registry.write().await;
//
//     for idx in 0..starts.len() {
//         let name = starts[idx].0;
//
//         let end = if idx == starts.len() {
//             lines.len()
//         } else {
//             starts[idx + 1].1
//         };
//
//         let mut buf = String::new();
//         for line_idx in idx..end {
//             buf.push_str(lines[line_idx]);
//         }
//
//         registry_write
//             .register_partial(&format!("{}:{}", tag, name), buf)
//             .map_err(|_| "Failed to register template".to_string())?
//     }
//
//     Ok(())
// }

//! Rust port of `hydrus/core/HydrusEnvironment.py`.

use std::collections::BTreeMap;

use crate::hydrus_data::safe_print as show_text;

/// `dump_env( env = None )`
///
/// Environment entries whose key contains `PATH`/`DIRS` and whose value looks
/// like a path list are split one entry per line.
pub fn dump_env(env: Option<&BTreeMap<String, String>>) {
    let owned;
    let env = match env {
        Some(env) => env,
        None => {
            owned = std::env::vars().collect::<BTreeMap<_, _>>();
            &owned
        }
    };

    let mut rows: Vec<String> = Vec::new();

    for (key, value) in env {
        if (key.contains("PATH") || key.contains("DIRS"))
            && std::env::split_paths(value).count() > 1
        {
            rows.push(format!("{key}:"));

            for sub_value in std::env::split_paths(value) {
                rows.push(format!("    {}", sub_value.display()));
            }
        } else {
            rows.push(format!("{key}: {value}"));
        }
    }

    show_text(format!("Full environment:\n{}", rows.join("\n")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_env_handles_both_shapes() {
        let mut env = BTreeMap::new();

        env.insert("PLAIN".to_string(), "value".to_string());
        env.insert("MY_PATH".to_string(), "one;two".to_string());

        // Output goes to stdout, but the important thing is that it does not
        // panic on path-list values.
        dump_env(Some(&env));
    }
}

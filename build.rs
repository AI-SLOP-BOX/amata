use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn main() {
    let root = Path::new("locales");
    println!("cargo:rerun-if-changed={}", root.display());

    let mut files: Vec<_> = fs::read_dir(root)
        .expect("locales directory is required")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "at least one locale catalog is required");

    let mut parsed = BTreeMap::new();
    let mut normalized_ids = std::collections::HashSet::new();
    let mut generated = String::from("pub static LOCALE_CATALOGS: &[(&str, &str)] = &[\n");
    for path in files {
        let locale = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("locale filename must be UTF-8");
        assert!(
            locale.split('-').all(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
            }),
            "invalid BCP-47 locale id: {locale}"
        );
        assert!(
            normalized_ids.insert(locale.to_ascii_lowercase()),
            "duplicate locale id (case-insensitive): {locale}"
        );
        println!("cargo:rerun-if-changed={}", path.display());
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read locale catalog {}: {error}", path.display()));
        let value: serde_json::Value = serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("invalid locale JSON {}: {error}", path.display()));
        let object = value
            .as_object()
            .unwrap_or_else(|| panic!("locale catalog {} must be an object", path.display()));
        assert!(
            object
                .get("name")
                .and_then(|value| value.as_str())
                .is_some_and(|s| !s.trim().is_empty()),
            "locale catalog {locale} needs a non-empty native `name`"
        );
        assert!(
            object
                .get("messages")
                .and_then(|value| value.as_object())
                .is_some(),
            "locale catalog {locale} needs a `messages` object"
        );
        let messages = object["messages"].as_object().unwrap();
        assert!(
            messages
                .iter()
                .all(|(key, value)| { !key.trim().is_empty() && value.as_str().is_some() }),
            "locale {locale} message IDs must be non-empty and all values strings"
        );
        let fallback = match object.get("fallback") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(value)) => Some(value.as_str()),
            Some(_) => panic!("locale {locale} `fallback` must be a locale tag or null"),
        };
        parsed.insert(locale.to_string(), (fallback.map(str::to_string), value));
        generated.push_str(&format!(
            "    ({locale:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{path}\"))),\n",
            path = path.display()
        ));
    }
    assert!(
        parsed.contains_key("ja"),
        "the `ja` fallback catalog is required"
    );
    let default_messages = parsed["ja"].1["messages"].as_object().unwrap();
    for (locale, (fallback, _)) in &parsed {
        let messages = parsed[locale].1["messages"].as_object().unwrap();
        let missing: Vec<_> = default_messages
            .keys()
            .filter(|key| !messages.contains_key(*key))
            .collect();
        if !missing.is_empty() {
            println!(
                "cargo:warning=locale {locale} is missing {} messages (they will use fallback)",
                missing.len()
            );
        }
        if let Some(fallback) = fallback {
            assert!(
                parsed.contains_key(fallback),
                "locale {locale} has missing fallback {fallback}"
            );
            let mut seen = std::collections::HashSet::new();
            let mut current = locale.as_str();
            while let Some((Some(next), _)) = parsed.get(current) {
                assert!(
                    seen.insert(current),
                    "locale fallback cycle includes {current}"
                );
                current = next;
            }
            assert!(
                seen.insert(current),
                "locale fallback cycle includes {current}"
            );
        }
    }
    generated.push_str("];\n");

    let out =
        Path::new(&std::env::var_os("OUT_DIR").expect("OUT_DIR is set")).join("locale_catalogs.rs");
    fs::write(out, generated).expect("write generated locale catalog index");
}

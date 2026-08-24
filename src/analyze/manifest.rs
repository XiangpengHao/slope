//! Compare two versions of a Cargo.toml and report dependency events:
//! added, removed, and version-bumped dependencies, per dependency table.

use crate::api::{DepEvent, DepKind};
use std::collections::HashMap;

pub(crate) struct ManifestEvent {
    /// Dependency name as declared (the table key).
    pub(crate) name: String,
    pub(crate) kind: DepKind,
    pub(crate) event: DepEvent,
    /// For removals: the old requirement, for the ghost's caption.
    pub(crate) detail: Option<String>,
}

/// One dependency table flattened to `name -> requirement`.
type DepMap = HashMap<String, String>;

/// A dependency's requirement as a short display string.
fn requirement(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => {
            if let Some(toml::Value::String(v)) = t.get("version") {
                v.clone()
            } else if t.contains_key("path") {
                "path".into()
            } else if t.contains_key("git") {
                "git".into()
            } else if t.contains_key("workspace") {
                "workspace".into()
            } else {
                "*".into()
            }
        }
        _ => "*".into(),
    }
}

fn collect_table(doc: &toml::Table, key: &str, into: &mut DepMap) {
    if let Some(toml::Value::Table(deps)) = doc.get(key) {
        for (name, value) in deps {
            into.insert(name.clone(), requirement(value));
        }
    }
}

/// All dependency tables of one kind, including `[target.'…'.dependencies]`.
fn deps_of_kind(doc: &toml::Table, kind: DepKind) -> DepMap {
    let key = match kind {
        DepKind::Normal => "dependencies",
        DepKind::Dev => "dev-dependencies",
        DepKind::Build => "build-dependencies",
    };
    let mut map = DepMap::new();
    collect_table(doc, key, &mut map);
    if let Some(toml::Value::Table(targets)) = doc.get("target") {
        for target in targets.values() {
            if let toml::Value::Table(target) = target {
                collect_table(target, key, &mut map);
            }
        }
    }
    map
}

impl ManifestEvent {
    pub(crate) fn diff(old: &str, new: &str) -> Vec<Self> {
        let (Ok(old), Ok(new)) = (old.parse::<toml::Table>(), new.parse::<toml::Table>()) else {
            return Vec::new();
        };

        let mut events = Vec::new();
        for kind in [DepKind::Normal, DepKind::Dev, DepKind::Build] {
            let before = deps_of_kind(&old, kind);
            let after = deps_of_kind(&new, kind);

            for (name, req) in &after {
                match before.get(name) {
                    None => events.push(ManifestEvent {
                        name: name.clone(),
                        kind,
                        event: DepEvent::Added,
                        detail: None,
                    }),
                    Some(old_req) if old_req != req => events.push(ManifestEvent {
                        name: name.clone(),
                        kind,
                        event: DepEvent::Bumped(old_req.clone(), req.clone()),
                        detail: None,
                    }),
                    _ => {}
                }
            }
            for (name, old_req) in &before {
                if !after.contains_key(name) {
                    events.push(ManifestEvent {
                        name: name.clone(),
                        kind,
                        event: DepEvent::Removed,
                        detail: Some(old_req.clone()),
                    });
                }
            }
        }
        events
    }
}

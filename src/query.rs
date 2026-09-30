use crate::{
    NoMatch,
    archive::{Index, scan},
    paths::normalize,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::{collections::BTreeSet, io::Write};

#[derive(Default, Clone, Debug)]
pub struct Selector {
    pub path: Option<String>,
    pub guid: Option<String>,
    pub entry: Option<String>,
    pub part: Part,
}
#[derive(Default, Clone, Copy, Debug, clap::ValueEnum)]
pub enum Part {
    #[default]
    Asset,
    Meta,
    Preview,
}
impl Selector {
    pub fn resolve(&self, index: &Index) -> Result<usize> {
        if let Some(entry) = &self.entry {
            let path = normalize(entry)?;
            let matches: Vec<_> = index.entries.iter().filter(|e| e.path == path).collect();
            ensure!(matches.len() <= 1, "ambiguous raw entry: {path}");
            return matches
                .first()
                .map(|e| e.id)
                .ok_or_else(|| NoMatch(format!("entry not found: {path}")).into());
        }
        let path = self.path.as_deref().map(normalize).transpose()?;
        let matches: Vec<_> = index
            .resources
            .iter()
            .filter(|r| {
                path.as_deref() == Some(r.path.as_str())
                    || self
                        .guid
                        .as_ref()
                        .is_some_and(|g| g.eq_ignore_ascii_case(&r.guid))
            })
            .collect();
        ensure!(matches.len() <= 1, "ambiguous resource selection");
        let r = matches
            .first()
            .ok_or_else(|| NoMatch("resource not found".into()))?;
        let id = match self.part {
            Part::Asset => r.asset,
            Part::Meta => r.meta,
            Part::Preview => r.preview,
        };
        id.ok_or_else(|| NoMatch(format!("{:?} missing for {}", self.part, r.path)).into())
    }
}

pub fn copy_entry(index: &Index, id: usize, output: &mut impl Write) -> Result<()> {
    ensure!(
        index.entries[id].kind == "file",
        "entry is not a regular file"
    );
    index.check_unchanged()?;
    scan(&index.source, index.limits, |current, entry| {
        if current == id {
            std::io::copy(entry, output)?;
        }
        Ok(())
    })?;
    index.check_unchanged()
}

#[derive(Debug, Serialize)]
pub struct Item {
    pub kind: String,
    pub path: String,
    pub guid: Option<String>,
    pub size: u64,
    pub has_meta: bool,
}
pub fn items(index: &Index, raw: bool) -> Vec<Item> {
    if raw {
        return index
            .entries
            .iter()
            .filter(|e| !e.path.is_empty())
            .map(|e| Item {
                kind: e.kind.clone(),
                path: e.path.clone(),
                guid: None,
                size: e.size,
                has_meta: false,
            })
            .collect();
    }
    let mut consumed = BTreeSet::new();
    let mut result = Vec::new();
    for r in &index.resources {
        consumed.insert(r.pathname);
        consumed.extend(r.asset);
        consumed.extend(r.meta);
        for &id in &r.entries {
            if index.entries[id].kind == "directory" && index.entries[id].path == r.guid {
                consumed.insert(id);
            }
        }
        result.push(Item {
            kind: if r.folder { "folder" } else { "asset" }.into(),
            path: r.path.clone(),
            guid: Some(r.guid.clone()),
            size: r.asset.map_or(0, |id| index.entries[id].size),
            has_meta: r.meta.is_some(),
        });
    }
    for e in &index.entries {
        if consumed.contains(&e.id) || e.path.is_empty() {
            continue;
        }
        let kind = match e.path.as_str() {
            ".icon.png" => "icon",
            ".cover.png" => "cover",
            "packagemanagermanifest/asset" => "manifest",
            _ => "raw",
        };
        result.push(Item {
            kind: kind.into(),
            path: e.path.clone(),
            guid: None,
            size: e.size,
            has_meta: false,
        });
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}

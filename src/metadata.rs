//! Discovered package metadata and byte-preserving, transactional edits.
use crate::{
    NoMatch,
    archive::{Index, scan},
    extract::write_selected,
    mutate,
    paths::{normalize, reject_symlinks},
    query::{Scope, Selector, copy_entry},
    write::{self, Addition, Data, Edit},
};
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::Path,
};

const JSON_LIMIT: u64 = 16 << 20;
const SET_LIMIT: u64 = 64 << 20;

#[derive(clap::ValueEnum, Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Manifest,
    Icon,
    Cover,
    PackageJson,
    ProjectManifest,
    Settings,
}
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Icon => "icon",
            Self::Cover => "cover",
            Self::PackageJson => "package-json",
            Self::ProjectManifest => "project-manifest",
            Self::Settings => "settings",
        }
    }

    fn fixed_path(self) -> Option<&'static str> {
        match self {
            Self::Manifest => Some("packagemanagermanifest/asset"),
            Self::Icon => Some(".icon.png"),
            Self::Cover => Some(".cover.png"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Item {
    #[serde(skip)]
    pub id: usize,
    pub kind: Kind,
    pub path: String,
    pub entry: String,
    pub guid: Option<String>,
    pub size: u64,
    pub format: String,
}

fn classify(path: &str) -> Option<Kind> {
    match path {
        "packagemanagermanifest/asset" => return Some(Kind::Manifest),
        ".icon.png" => return Some(Kind::Icon),
        ".cover.png" => return Some(Kind::Cover),
        "Packages/manifest.json" => return Some(Kind::ProjectManifest),
        _ => {}
    }
    let parts: Vec<_> = path.split('/').collect();
    if parts.last() == Some(&"package.json") {
        return Some(Kind::PackageJson);
    }
    if !parts[0].eq_ignore_ascii_case("Assets")
        && !path.to_ascii_lowercase().ends_with(".meta")
        && parts[..parts.len() - 1]
            .iter()
            .any(|p| p.to_ascii_lowercase().ends_with("settings"))
    {
        return Some(Kind::Settings);
    }
    None
}

fn format(kind: Kind, path: &str) -> &'static str {
    if matches!(kind, Kind::Icon | Kind::Cover) {
        "png"
    } else if matches!(
        kind,
        Kind::Manifest | Kind::PackageJson | Kind::ProjectManifest
    ) || path.to_ascii_lowercase().ends_with(".json")
    {
        "json"
    } else if path.to_ascii_lowercase().ends_with(".yaml")
        || path.to_ascii_lowercase().ends_with(".yml")
    {
        "yaml"
    } else {
        "opaque"
    }
}

pub fn inventory(index: &Index) -> Vec<Item> {
    let mut result = vec![];
    let mut consumed = BTreeSet::new();
    let mut add = |id: usize, path: &str, guid: Option<&str>| {
        let entry = &index.entries[id];
        if entry.kind == "file"
            && let Some(kind) = classify(path)
        {
            result.push(Item {
                id,
                kind,
                path: path.into(),
                entry: entry.path.clone(),
                guid: guid.map(str::to_owned),
                size: entry.size,
                format: format(kind, path).into(),
            });
        }
    };
    for r in &index.resources {
        consumed.insert(r.pathname);
        consumed.extend(r.asset);
        consumed.extend(r.meta);
        consumed.extend(r.preview);
        if !r.folder
            && let Some(id) = r.asset
        {
            add(id, &r.path, Some(&r.guid));
        }
    }
    for e in &index.entries {
        if !consumed.contains(&e.id) && !e.path.is_empty() {
            add(e.id, &e.path, None);
        }
    }
    result.sort_by(|a, b| (&a.path, &a.entry).cmp(&(&b.path, &b.entry)));
    result
}

pub fn list(index: &Index, kind: Option<Kind>, scope: &Scope) -> Result<Vec<Item>> {
    let items: Vec<_> = inventory(index)
        .into_iter()
        .filter(|i| kind.is_none_or(|k| k == i.kind) && scope.matches(&i.path))
        .collect();
    if items.is_empty() && (kind.is_some() || scope.path().is_some()) {
        return Err(NoMatch("no metadata matches the requested kind/path".into()).into());
    }
    Ok(items)
}

pub fn resolve(index: &Index, kind: Kind, selector: &Selector) -> Result<Item> {
    let path = selector.path.as_deref().map(normalize).transpose()?;
    let entry = selector.entry.as_deref().map(normalize).transpose()?;
    let matches: Vec<_> = inventory(index)
        .into_iter()
        .filter(|item| {
            item.kind == kind
                && path.as_ref().is_none_or(|p| *p == item.path)
                && entry.as_ref().is_none_or(|p| *p == item.entry)
                && selector.guid.as_ref().is_none_or(|g| {
                    item.guid
                        .as_ref()
                        .is_some_and(|id| g.eq_ignore_ascii_case(id))
                })
        })
        .collect();
    ensure!(
        matches.len() <= 1,
        "ambiguous {} metadata; select --path, --guid or --entry: {}",
        kind.name(),
        matches
            .iter()
            .map(|i| format!("{} (--entry {})", i.path, i.entry))
            .collect::<Vec<_>>()
            .join(", ")
    );
    matches
        .into_iter()
        .next()
        .ok_or_else(|| NoMatch(format!("{} metadata not found", kind.name())).into())
}

#[derive(Debug, Serialize)]
pub struct SummaryItem {
    #[serde(flatten)]
    pub item: Item,
    pub details: Value,
    pub warning: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub entries: Vec<SummaryItem>,
    pub counts: BTreeMap<Kind, usize>,
}

pub fn summary(index: &Index, scope: &Scope) -> Result<Summary> {
    let items = list(index, None, scope)?;
    let mut result = Summary {
        entries: vec![],
        counts: BTreeMap::new(),
    };
    let mut positions = BTreeMap::new();
    for item in items {
        *result.counts.entry(item.kind).or_default() += 1;
        positions.insert(item.id, result.entries.len());
        result.entries.push(SummaryItem {
            item,
            details: Value::Null,
            warning: None,
        });
    }
    index.check_unchanged()?;
    scan(&index.source, index.limits, |id, entry| {
        let Some(&position) = positions.get(&id) else {
            return Ok(());
        };
        let record = &mut result.entries[position];
        let outcome = match record.item.format.as_str() {
            "json" if record.item.size > JSON_LIMIT => Err(anyhow::anyhow!(
                "JSON summary exceeds 16 MiB; use get or dump for exact bytes"
            )),
            "json" => {
                let mut bytes = vec![];
                entry.take(JSON_LIMIT + 1).read_to_end(&mut bytes)?;
                json_details(&bytes)
            }
            "png" => {
                let mut bytes = vec![];
                entry.take(33).read_to_end(&mut bytes)?;
                png_details(&bytes)
            }
            _ => Ok(Value::Null),
        };
        match outcome {
            Ok(details) => record.details = details,
            Err(error) => record.warning = Some(format!("{error:#}")),
        }
        Ok(())
    })?;
    index.check_unchanged()?;
    Ok(result)
}

fn json_details(bytes: &[u8]) -> Result<Value> {
    let value: Value = serde_json::from_slice(bytes).context("invalid metadata JSON")?;
    let object = value
        .as_object()
        .context("metadata JSON is not an object")?;
    let mut details = json!({"fields":object.len()});
    for key in ["name", "version", "displayName", "unity"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            details[key] = Value::String(text.chars().take(512).collect());
        }
    }
    if let Some(deps) = value.get("dependencies").and_then(Value::as_object) {
        details["dependencies"] = json!(deps.len());
    }
    Ok(details)
}

fn png_details(bytes: &[u8]) -> Result<Value> {
    ensure!(
        bytes.len() == 33
            && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            && bytes[8..12] == 13_u32.to_be_bytes()
            && &bytes[12..16] == b"IHDR",
        "invalid PNG header"
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into()?);
    ensure!(width != 0 && height != 0, "invalid PNG dimensions");
    Ok(json!({"width":width,"height":height,"validation":"header-only"}))
}

pub fn dump(index: &Index, output: &Path, kind: Option<Kind>, scope: &Scope) -> Result<usize> {
    index.ensure_valid()?;
    let selected: BTreeMap<_, _> = list(index, kind, scope)?
        .into_iter()
        .map(|i| (i.id, i.path))
        .collect();
    if selected.is_empty() {
        return Err(NoMatch("no metadata to dump".into()).into());
    }
    write_selected(index, output, &selected, &BTreeSet::new())
}

pub fn get(index: &Index, kind: Kind, selector: &Selector, out: &mut impl Write) -> Result<()> {
    copy_entry(index, resolve(index, kind, selector)?.id, out)
}

fn validate(item: &Item, bytes: &[u8]) -> Result<()> {
    if matches!(item.kind, Kind::Icon | Kind::Cover) {
        mutate::validate_metadata(item.kind.name(), bytes)
    } else if item.format == "json" {
        let value: Value = serde_json::from_slice(bytes).context("invalid metadata JSON")?;
        ensure!(value.is_object(), "metadata JSON must be an object");
        if matches!(item.kind, Kind::Manifest | Kind::ProjectManifest) {
            mutate::validate_metadata("manifest", bytes)?;
        }
        Ok(())
    } else {
        Ok(())
    }
}

pub fn set(
    index: &Index,
    kind: Kind,
    selector: &Selector,
    source: &Path,
    output: &Path,
    force: bool,
) -> Result<()> {
    reject_symlinks(source)?;
    ensure!(
        source.metadata()?.len() <= SET_LIMIT,
        "metadata file exceeds 64 MiB"
    );
    let mut bytes = vec![];
    std::fs::File::open(source)?
        .take(SET_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= SET_LIMIT,
        "metadata file exceeds 64 MiB"
    );
    let item = match resolve(index, kind, selector) {
        Ok(item) => item,
        Err(error)
            if error.is::<NoMatch>()
                && selector.path.is_none()
                && selector.guid.is_none()
                && selector.entry.is_none() =>
        {
            if let Some(path) = kind.fixed_path() {
                let item = Item {
                    id: 0,
                    kind,
                    path: path.into(),
                    entry: path.into(),
                    guid: None,
                    size: 0,
                    format: format(kind, path).into(),
                };
                validate(&item, &bytes)?;
                return write::rewrite(
                    index,
                    output,
                    force,
                    &BTreeMap::new(),
                    &[Addition {
                        path: path.into(),
                        data: Data::Bytes(bytes),
                    }],
                );
            }
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    validate(&item, &bytes)?;
    write::rewrite(
        index,
        output,
        force,
        &BTreeMap::from([(item.id, Edit::Replace(Data::Bytes(bytes)))]),
        &[],
    )
}

pub fn remove(
    index: &Index,
    kind: Kind,
    selector: &Selector,
    output: &Path,
    force: bool,
) -> Result<()> {
    let item = resolve(index, kind, selector)?;
    let target = if let Some(guid) = item.guid {
        Selector {
            guid: Some(guid),
            ..Default::default()
        }
    } else {
        Selector {
            entry: Some(item.entry),
            ..Default::default()
        }
    };
    mutate::remove(index, &target, false, output, force)
}

pub fn edit(
    index: &Index,
    kind: Kind,
    selector: &Selector,
    sets: &[String],
    deletes: &[String],
    output: &Path,
    force: bool,
) -> Result<()> {
    ensure!(
        !sets.is_empty() || !deletes.is_empty(),
        "edit requires --set or --delete"
    );
    let item = resolve(index, kind, selector)?;
    ensure!(
        item.format == "json",
        "field editing requires JSON; use set --file for YAML/binary settings"
    );
    ensure!(item.size <= JSON_LIMIT, "JSON editing exceeds 16 MiB");
    let mut bytes = vec![];
    copy_entry(index, item.id, &mut bytes)?;
    let mut value: Value = serde_json::from_slice(&bytes).context("invalid metadata JSON")?;
    for assignment in sets {
        let (pointer, literal) = assignment
            .split_once('=')
            .context("--set requires /pointer=JSON_VALUE")?;
        let tokens = pointer_tokens(pointer)?;
        let replacement =
            serde_json::from_str(literal).context("--set value must be valid JSON")?;
        assign(&mut value, &tokens, replacement).with_context(|| format!("set {pointer}"))?;
    }
    for pointer in deletes {
        let tokens = pointer_tokens(pointer)?;
        delete(&mut value, &tokens).with_context(|| format!("delete {pointer}"))?;
    }
    bytes = serde_json::to_vec_pretty(&value)?;
    bytes.push(b'\n');
    validate(&item, &bytes)?;
    write::rewrite(
        index,
        output,
        force,
        &BTreeMap::from([(item.id, Edit::Replace(Data::Bytes(bytes)))]),
        &[],
    )
}

fn pointer_tokens(pointer: &str) -> Result<Vec<String>> {
    if pointer.is_empty() {
        return Ok(vec![]);
    }
    ensure!(
        pointer.starts_with('/'),
        "JSON Pointer must be empty or start with /"
    );
    ensure!(
        pointer[1..].split('/').count() <= 128,
        "JSON Pointer exceeds 128 components"
    );
    pointer[1..]
        .split('/')
        .map(|token| {
            let mut decoded = String::new();
            let mut chars = token.chars();
            while let Some(ch) = chars.next() {
                decoded.push(if ch == '~' {
                    match chars.next() {
                        Some('0') => '~',
                        Some('1') => '/',
                        _ => bail!("invalid JSON Pointer escape; use ~0 or ~1"),
                    }
                } else {
                    ch
                });
            }
            Ok(decoded)
        })
        .collect()
}

fn array_index(token: &str, length: usize) -> Result<usize> {
    ensure!(
        token == "0" || (!token.starts_with('0') && token.bytes().all(|b| b.is_ascii_digit())),
        "invalid JSON array index: {token}"
    );
    let index: usize = token.parse().context("invalid JSON array index")?;
    ensure!(index < length, "JSON array index out of bounds: {token}");
    Ok(index)
}

fn assign(value: &mut Value, tokens: &[String], replacement: Value) -> Result<()> {
    let Some((key, rest)) = tokens.split_first() else {
        *value = replacement;
        return Ok(());
    };
    match value {
        Value::Object(object) if rest.is_empty() => {
            object.insert(key.clone(), replacement);
        }
        Value::Object(object) => {
            assign(
                object.entry(key.clone()).or_insert_with(|| json!({})),
                rest,
                replacement,
            )?;
        }
        Value::Array(array) if key == "-" => {
            ensure!(
                rest.is_empty(),
                "array append '-' must be the final pointer component"
            );
            array.push(replacement);
        }
        Value::Array(array) => {
            let index = array_index(key, array.len())?;
            assign(&mut array[index], rest, replacement)?;
        }
        _ => bail!("JSON Pointer parent is not an object or array"),
    }
    Ok(())
}

fn delete(value: &mut Value, tokens: &[String]) -> Result<()> {
    let (key, rest) = tokens
        .split_first()
        .context("cannot delete the JSON document root")?;
    match value {
        Value::Object(object) if rest.is_empty() => {
            ensure!(object.remove(key).is_some(), "JSON field not found: {key}");
        }
        Value::Object(object) => {
            delete(object.get_mut(key).context("JSON field not found")?, rest)?;
        }
        Value::Array(array) => {
            let index = array_index(key, array.len())?;
            if rest.is_empty() {
                array.remove(index);
            } else {
                delete(&mut array[index], rest)?;
            }
        }
        _ => bail!("JSON Pointer parent is not an object or array"),
    }
    Ok(())
}

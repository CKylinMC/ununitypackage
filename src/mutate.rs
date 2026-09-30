use crate::{
    NoMatch,
    archive::{Index, meta_info},
    pack,
    paths::{normalize, reject_symlinks},
    query::{Part, Selector, copy_entry},
    write::{self, Addition, Data, Edit},
};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, io::Cursor, path::Path};

pub fn add(
    index: &Index,
    source: &Path,
    target: &Selector,
    meta: Option<&Path>,
    generate: bool,
    output: &Path,
    force: bool,
) -> Result<()> {
    reject_symlinks(source)?;
    let additions = if let Some(entry) = &target.entry {
        let path = normalize(entry)?;
        ensure!(
            meta.is_none() && !generate,
            "raw addition does not accept metadata generation"
        );
        vec![Addition {
            path,
            data: if source.is_dir() {
                Data::Directory
            } else {
                Data::File(source.to_owned())
            },
        }]
    } else {
        let target = target
            .path
            .as_deref()
            .context("add needs --path or --entry")?;
        let target = normalize(target)?;
        ensure!(
            !index.resources.iter().any(|r| r.path == target),
            "resource already exists: {target}"
        );
        let sibling = std::path::PathBuf::from(format!("{}.meta", source.display()));
        let meta = meta.or_else(|| sibling.exists().then_some(sibling.as_path()));
        let (guid, additions) = pack::resource(source, &target, meta, generate)?;
        ensure!(
            !index
                .resources
                .iter()
                .any(|r| r.guid.eq_ignore_ascii_case(&guid)),
            "GUID already exists: {guid}"
        );
        additions
    };
    write::rewrite(index, output, force, &BTreeMap::new(), &additions)
}

pub fn replace(
    index: &Index,
    source: &Path,
    selector: &Selector,
    output: &Path,
    force: bool,
) -> Result<()> {
    reject_symlinks(source)?;
    let id = selector.resolve(index)?;
    ensure!(
        index.entries[id].kind == "file",
        "replacement target is not a regular file"
    );
    if selector.entry.is_none() && matches!(selector.part, Part::Meta) {
        ensure!(source.metadata()?.len() <= 4 << 20, "meta too large");
        let new = meta_info(&std::fs::read(source)?)?;
        let r = index
            .resources
            .iter()
            .find(|r| r.meta == Some(id))
            .context("meta resource missing")?;
        ensure!(
            new.guid
                .as_ref()
                .is_some_and(|g| g.eq_ignore_ascii_case(&r.guid))
                && new.folder == r.folder,
            "replacement meta must preserve GUID and folderAsset"
        );
    }
    let edits = BTreeMap::from([(id, Edit::Replace(Data::File(source.to_owned())))]);
    write::rewrite(index, output, force, &edits, &[])
}

pub fn remove(
    index: &Index,
    selector: &Selector,
    recursive: bool,
    output: &Path,
    force: bool,
) -> Result<()> {
    let mut edits = BTreeMap::new();
    if let Some(entry) = &selector.entry {
        let path = normalize(entry)?;
        let id = selector.resolve(index)?;
        let descendants: Vec<_> = index
            .entries
            .iter()
            .filter(|e| e.path.starts_with(&format!("{path}/")))
            .collect();
        ensure!(
            recursive || descendants.is_empty(),
            "raw directory has descendants; use --recursive"
        );
        edits.insert(id, Edit::Delete);
        if recursive {
            for e in descendants {
                edits.insert(e.id, Edit::Delete);
            }
        }
    } else {
        let path = selector.path.as_deref().map(normalize).transpose()?;
        let matches: Vec<_> = index
            .resources
            .iter()
            .filter(|r| {
                path.as_deref() == Some(r.path.as_str())
                    || selector
                        .guid
                        .as_ref()
                        .is_some_and(|g| g.eq_ignore_ascii_case(&r.guid))
            })
            .collect();
        ensure!(matches.len() <= 1, "ambiguous resource selection");
        let r = matches
            .first()
            .ok_or_else(|| NoMatch("resource not found".into()))?;
        let prefix = format!("{}/", r.path);
        let children: Vec<_> = index
            .resources
            .iter()
            .filter(|item| item.path.starts_with(&prefix))
            .collect();
        ensure!(
            recursive || children.is_empty(),
            "directory contains resources; use --recursive"
        );
        for &id in &r.entries {
            edits.insert(id, Edit::Delete);
        }
        if recursive {
            for child in children {
                for &id in &child.entries {
                    edits.insert(id, Edit::Delete);
                }
            }
        }
    }
    write::rewrite(index, output, force, &edits, &[])
}

pub fn validate_metadata(kind: &str, bytes: &[u8]) -> Result<()> {
    if kind == "manifest" {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).context("invalid manifest JSON")?;
        ensure!(value.is_object(), "manifest must be a JSON object");
        if let Some(deps) = value.get("dependencies") {
            ensure!(
                deps.as_object()
                    .is_some_and(|obj| obj.values().all(|v| v.is_string())),
                "manifest dependencies must map names to strings"
            );
        }
    } else {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_limits(png::Limits { bytes: 64 << 20 });
        let mut reader = decoder.read_info().context("invalid PNG")?;
        let size = reader
            .output_buffer_size()
            .context("PNG dimensions too large")?;
        ensure!(size <= 64 << 20, "PNG decoded size exceeds 64 MiB");
        let mut buffer = vec![0; size];
        reader
            .next_frame(&mut buffer)
            .context("invalid PNG image data")?;
        reader.finish().context("invalid PNG end chunks")?;
    }
    Ok(())
}

pub fn metadata_path(kind: &str) -> Result<&'static str> {
    match kind {
        "manifest" => Ok("packagemanagermanifest/asset"),
        "icon" => Ok(".icon.png"),
        "cover" => Ok(".cover.png"),
        _ => anyhow::bail!("unknown metadata kind: {kind}"),
    }
}
pub fn metadata_set(
    index: &Index,
    kind: &str,
    file: &Path,
    output: &Path,
    force: bool,
) -> Result<()> {
    reject_symlinks(file)?;
    ensure!(
        file.metadata()?.len() <= 64 << 20,
        "metadata file exceeds 64 MiB"
    );
    let bytes = std::fs::read(file)?;
    validate_metadata(kind, &bytes)?;
    let path = metadata_path(kind)?;
    let selector = Selector {
        entry: Some(path.into()),
        ..Default::default()
    };
    let mut edits = BTreeMap::new();
    let mut additions = vec![];
    match selector.resolve(index) {
        Ok(id) => {
            ensure!(
                index.entries[id].kind == "file",
                "metadata target is not a file"
            );
            edits.insert(id, Edit::Replace(Data::Bytes(bytes)));
        }
        Err(e) if e.is::<NoMatch>() => additions.push(Addition {
            path: path.into(),
            data: Data::Bytes(bytes),
        }),
        Err(e) => return Err(e),
    }
    write::rewrite(index, output, force, &edits, &additions)
}
pub fn metadata_get(index: &Index, kind: &str, out: &mut impl std::io::Write) -> Result<()> {
    let selector = Selector {
        entry: Some(metadata_path(kind)?.into()),
        ..Default::default()
    };
    copy_entry(index, selector.resolve(index)?, out)
}

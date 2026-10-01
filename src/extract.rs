use crate::{
    NoMatch,
    archive::{Index, scan},
    paths::{check_host_path, fs_key, normalize, reject_symlinks},
    query::Scope,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    path::Path,
};

#[derive(Default)]
pub struct Extraction {
    pub scope: Option<String>,
    pub paths: Vec<String>,
    pub guids: Vec<String>,
    pub globs: Vec<String>,
    pub entries: Vec<String>,
    pub raw: bool,
    pub no_meta: bool,
}

pub fn extract(index: &Index, output: &Path, selection: &Extraction) -> Result<usize> {
    if selection.raw
        || (!selection.entries.is_empty()
            && selection.paths.is_empty()
            && selection.guids.is_empty()
            && selection.globs.is_empty())
    {
        index.ensure_physical_valid()?;
    } else {
        index.ensure_valid()?;
    }
    index.check_unchanged()?;
    let scope = Scope::new(selection.scope.as_deref())?;
    let mut selected = BTreeMap::<usize, String>::new();
    let mut directories = BTreeSet::new();
    let full = selection.paths.is_empty()
        && selection.guids.is_empty()
        && selection.globs.is_empty()
        && selection.entries.is_empty();
    let paths: BTreeSet<_> = selection
        .paths
        .iter()
        .map(|p| normalize(p))
        .collect::<Result<_>>()?;
    let entries: BTreeSet<_> = selection
        .entries
        .iter()
        .map(|p| normalize(p))
        .collect::<Result<_>>()?;
    let mut glob_builder = globset::GlobSetBuilder::new();
    for g in &selection.globs {
        glob_builder.add(globset::Glob::new(g)?);
    }
    let globs = glob_builder.build()?;
    let mut matched_paths = BTreeSet::new();
    let mut matched_entries = BTreeSet::new();
    let mut matched_guids = BTreeSet::new();
    let mut consumed = BTreeSet::new();
    if !selection.raw {
        for r in &index.resources {
            consumed.insert(r.pathname);
            consumed.extend(r.asset);
            consumed.extend(r.meta);
            for &id in &r.entries {
                if index.entries[id].kind == "directory" && index.entries[id].path == r.guid {
                    consumed.insert(id);
                }
            }
            let guid_matches: Vec<_> = selection
                .guids
                .iter()
                .filter(|g| g.eq_ignore_ascii_case(&r.guid))
                .collect();
            if scope.matches(&r.path)
                && (full
                    || paths.contains(&r.path)
                    || !guid_matches.is_empty()
                    || globs.is_match(&r.path))
            {
                matched_paths.insert(r.path.clone());
                matched_guids.extend(guid_matches.into_iter().cloned());
                if r.folder {
                    directories.insert(r.path.clone());
                } else if let Some(id) = r.asset {
                    selected.insert(id, r.path.clone());
                }
                if !selection.no_meta
                    && let Some(id) = r.meta
                {
                    selected.insert(id, format!("{}.meta", r.path));
                }
            }
        }
    }
    for e in &index.entries {
        if e.path.is_empty() || !scope.matches(&e.path) {
            continue;
        }
        let explicit = entries.contains(&e.path);
        let raw_match =
            selection.raw && (full || paths.contains(&e.path) || globs.is_match(&e.path));
        if explicit || raw_match || (full && !consumed.contains(&e.id)) {
            matched_entries.insert(e.path.clone());
            matched_paths.insert(e.path.clone());
            ensure!(
                e.kind != "unsupported",
                "cannot extract link/special entry: {} (inspect with list --raw)",
                e.path
            );
            if e.kind == "directory" {
                directories.insert(e.path.clone());
            } else {
                selected.entry(e.id).or_insert_with(|| e.path.clone());
            }
        }
    }
    for p in &paths {
        if !matched_paths.contains(p) {
            return Err(NoMatch(format!("requested path not found: {p}")).into());
        }
    }
    for p in &entries {
        if !matched_entries.contains(p) {
            return Err(NoMatch(format!("requested entry not found: {p}")).into());
        }
    }
    for g in &selection.guids {
        if !matched_guids.contains(g) {
            return Err(NoMatch(format!("requested GUID not found: {g}")).into());
        }
    }
    if selected.is_empty() && directories.is_empty() {
        return Err(NoMatch("no entries selected".into()).into());
    }
    write_selected(index, output, &selected, &directories)
}

/// Preflight and stream a named entry selection; shared by extraction and metadata dump.
pub(crate) fn write_selected(
    index: &Index,
    output: &Path,
    selected: &BTreeMap<usize, String>,
    directories: &BTreeSet<String>,
) -> Result<usize> {
    index.check_unchanged()?;
    let mut output_names = BTreeSet::new();
    let mut files = BTreeSet::new();
    for path in selected.values() {
        check_host_path(path)?;
        ensure!(
            output_names.insert(fs_key(path)),
            "output path collision: {path}"
        );
        files.insert(fs_key(path));
    }
    for path in selected.values().chain(directories.iter()) {
        check_host_path(path)?;
        for parent in Path::new(path).ancestors().skip(1) {
            if parent.as_os_str().is_empty() {
                continue;
            }
            let parent = parent.to_string_lossy().replace('\\', "/");
            ensure!(
                !files.contains(&fs_key(&parent)),
                "file/directory collision: {parent}"
            );
        }
    }
    for path in directories {
        ensure!(
            !files.contains(&fs_key(path)),
            "file/directory collision: {path}"
        );
    }
    // Preflight every destination before creating any output.
    reject_symlinks(output)?;
    for path in selected.values() {
        let target = output.join(path);
        reject_symlinks(&target)?;
        ensure!(
            !target.try_exists()?,
            "destination exists: {}",
            target.display()
        );
    }
    for path in directories {
        let target = output.join(path);
        reject_symlinks(&target)?;
        ensure!(
            !target.try_exists()? || target.is_dir(),
            "not a directory: {}",
            target.display()
        );
    }
    for path in directories {
        fs::create_dir_all(output.join(path))?;
    }
    scan(&index.source, index.limits, |id, entry| {
        if let Some(path) = selected.get(&id) {
            let target = output.join(path);
            reject_symlinks(&target)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            reject_symlinks(&target)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .with_context(|| format!("create {}", target.display()))?;
            std::io::copy(entry, &mut file)?;
        }
        Ok(())
    })?;
    index.check_unchanged()?;
    Ok(selected.len() + directories.len())
}

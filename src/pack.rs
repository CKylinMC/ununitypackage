use crate::{
    archive::{Limits, is_guid, meta_info},
    paths::{normalize, reject_symlinks},
    write::{self, Addition, Data},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Default)]
pub struct PackOptions {
    pub prefix: String,
    pub generate_meta: bool,
    pub exclude: Vec<String>,
    pub force: bool,
}

pub fn generated_guid(path: &str) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("uup-cli:{path}").as_bytes())
        .simple()
        .to_string()
}
pub fn generated_meta(guid: &str, folder: bool) -> Vec<u8> {
    format!("fileFormatVersion: 2\nguid: {guid}\n{}", if folder {"folderAsset: yes\nDefaultImporter:\n  userData: \n  assetBundleName: \n  assetBundleVariant: \n"} else {""}).into_bytes()
}
fn no_meta_expected(target: &str) -> bool {
    matches!(
        target.split('/').next(),
        Some("ProjectSettings" | "PackageSettings" | "UserSettings")
    ) || target
        .split('/')
        .any(|p| p.ends_with('~') || p.starts_with('.'))
}
pub fn resource(
    source: &Path,
    target: &str,
    meta: Option<&Path>,
    generate: bool,
) -> Result<(String, Vec<Addition>)> {
    reject_symlinks(source)?;
    ensure!(
        source.is_file() || source.is_dir(),
        "source is not a file or directory: {}",
        source.display()
    );
    let target = normalize(target)?;
    let folder = source.is_dir();
    let mut meta_bytes = None;
    let guid;
    if let Some(meta_path) = meta {
        reject_symlinks(meta_path)?;
        ensure!(meta_path.metadata()?.len() <= 4 << 20, "meta exceeds 4 MiB");
        let bytes = std::fs::read(meta_path)?;
        let info = meta_info(&bytes)?;
        ensure!(
            info.folder == folder,
            "folderAsset does not match source type: {}",
            source.display()
        );
        guid = info.guid.context("meta without GUID")?;
        ensure!(is_guid(&guid), "invalid GUID");
        meta_bytes = Some(bytes);
    } else {
        ensure!(
            generate || no_meta_expected(&target),
            "missing .meta for {target}; supply --meta or --generate-meta"
        );
        guid = generated_guid(&target);
        if generate && !no_meta_expected(&target) {
            meta_bytes = Some(generated_meta(&guid, folder));
        }
    }
    let mut additions = vec![Addition {
        path: format!("{guid}/pathname"),
        data: Data::Bytes(target.into_bytes()),
    }];
    if !folder {
        additions.push(Addition {
            path: format!("{guid}/asset"),
            data: Data::File(source.to_owned()),
        });
    }
    if let Some(bytes) = meta_bytes {
        additions.push(Addition {
            path: format!("{guid}/asset.meta"),
            data: Data::Bytes(bytes),
        });
    }
    Ok((guid, additions))
}

pub fn pack(source: &Path, output: &Path, options: &PackOptions, limits: Limits) -> Result<usize> {
    reject_symlinks(source)?;
    ensure!(source.is_dir(), "source is not a directory");
    let prefix = normalize(&options.prefix)?;
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in &options.exclude {
        builder.add(globset::Glob::new(pattern)?);
    }
    let excludes = builder.build()?;
    let excluded = |relative: &str| {
        let base = relative.strip_suffix(".meta").unwrap_or(relative);
        base.split('/').any(|p| {
            matches!(
                p,
                ".git"
                    | ".svn"
                    | ".hg"
                    | "node_modules"
                    | "target"
                    | "Library"
                    | "Temp"
                    | "Logs"
                    | "obj"
                    | "bin"
                    | ".DS_Store"
            )
        }) || excludes.is_match(base)
    };
    let output_absolute = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir()?.join(output)
    };
    // An output inside the source can be accidentally included on the next run.
    let source_absolute = source.canonicalize()?;
    let output_parent = output_absolute.parent().context("missing output parent")?;
    if output_parent.exists() {
        let resolved = output_parent
            .canonicalize()?
            .join(output_absolute.file_name().context("missing filename")?);
        ensure!(
            !resolved.starts_with(&source_absolute),
            "output package must be outside the source directory"
        );
    } else {
        ensure!(
            !output_absolute.starts_with(&source_absolute),
            "output package must be outside source directory"
        );
    }
    let mut additions = Vec::new();
    let mut guids = BTreeSet::new();
    let mut count = 0;
    let walker = WalkDir::new(source)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let relative = entry
                .path()
                .strip_prefix(source)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            !excluded(&relative)
        });
    for entry in walker {
        let entry = entry?;
        if entry.depth() == 0 {
            continue;
        }
        ensure!(
            !entry.file_type().is_symlink(),
            "source contains symlink: {}",
            entry.path().display()
        );
        let relative = entry
            .path()
            .strip_prefix(source)?
            .to_str()
            .context("source path is not UTF-8")?
            .replace('\\', "/");
        if relative.ends_with(".meta") {
            let companion = entry.path().with_file_name(
                entry
                    .file_name()
                    .to_str()
                    .context("meta path is not UTF-8")?
                    .strip_suffix(".meta")
                    .unwrap(),
            );
            ensure!(
                companion.exists(),
                "orphan meta: {}",
                entry.path().display()
            );
            continue;
        }
        ensure!(
            entry.file_type().is_file() || entry.file_type().is_dir(),
            "unsupported source file type"
        );
        let target = format!("{prefix}/{relative}");
        let meta_path = std::path::PathBuf::from(format!("{}.meta", entry.path().display()));
        let meta = meta_path.exists().then_some(meta_path.as_path());
        let (guid, records) = resource(entry.path(), &target, meta, options.generate_meta)?;
        ensure!(guids.insert(guid.clone()), "duplicate GUID: {guid}");
        additions.extend(records);
        count += 1;
    }
    // A sibling meta describes the selected root folder (when it exists).
    let root_meta = std::path::PathBuf::from(format!("{}.meta", source.display()));
    if root_meta.exists() || (options.generate_meta && prefix != "Assets") {
        let (guid, records) = resource(
            source,
            &prefix,
            root_meta.exists().then_some(root_meta.as_path()),
            options.generate_meta,
        )?;
        ensure!(guids.insert(guid.clone()), "duplicate root GUID: {guid}");
        additions.extend(records);
        count += 1;
    }
    ensure!(count > 0, "no package resources found");
    write::create(output, options.force, &additions, limits)?;
    Ok(count)
}

pub fn upm_manifest(source: &Path) -> Result<(Value, Vec<String>)> {
    let manifest_path = source.join("package.json");
    reject_symlinks(&manifest_path)?;
    ensure!(
        manifest_path.metadata()?.len() <= 16 << 20,
        "package.json exceeds 16 MiB"
    );
    let manifest: Value = serde_json::from_slice(&std::fs::read(manifest_path)?)?;
    let name = manifest["name"]
        .as_str()
        .context("package.json needs name")?;
    ensure!(
        regex::Regex::new(r"^[a-z0-9]+(?:[._-][a-z0-9]+)+$")?.is_match(name),
        "invalid UPM package name: {name}"
    );
    semver::Version::parse(
        manifest["version"]
            .as_str()
            .context("package.json needs version")?,
    )?;
    let mut warnings = Vec::new();
    if manifest.get("scopedRegistries").is_some() {
        warnings.push("scopedRegistries must be configured in the destination project's Packages/manifest.json.".into());
    }
    if let Some(deps) = manifest.get("dependencies") {
        let deps = deps.as_object().context("dependencies must be an object")?;
        for (name, version) in deps {
            let version = version
                .as_str()
                .context("dependency version must be a string")?;
            if semver::Version::parse(version).is_err() {
                warnings.push(format!(
                    "dependency {name}={version} requires external resolution (Git/file/range)"
                ));
            } else if !name.starts_with("com.unity.") {
                warnings.push(format!(
                    "dependency {name} may need a custom registry in the destination project"
                ));
            }
        }
    }
    warnings.push("Dependencies are preserved, not bundled; path-sensitive code and importer behavior require Unity validation.".into());
    if !WalkDir::new(source)
        .into_iter()
        .filter_map(Result::ok)
        .any(|e| e.path().extension().is_some_and(|x| x == "asmdef"))
    {
        warnings.push(
            "No .asmdef found; confirm that package scripts have the intended assemblies in Unity."
                .into(),
        );
    }
    Ok((manifest, warnings))
}
pub fn upm_report(manifest: &Value, prefix: &str, count: usize, warnings: &[String]) -> Value {
    json!({"name":manifest["name"], "version":manifest["version"], "target":prefix,
        "resources":count, "dependencies":manifest.get("dependencies").cloned().unwrap_or(json!({})),
        "warnings":warnings, "unity_import":"not_verified"})
}

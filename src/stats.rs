//! Filename-based resource statistics, separate from physical archive overhead.
use crate::{
    archive::Index,
    query::{Scope, scoped_items},
};
use anyhow::Result;
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

const CATEGORIES: &[&str] = &[
    "animations",
    "controllers",
    "models",
    "audio",
    "video",
    "images",
    "textures",
    "scripts",
    "assembly_definitions",
    "plugin_binaries",
    "scenes",
    "prefabs",
    "materials",
    "shaders",
    "ui",
    "fonts",
    "physics",
    "terrain",
    "text",
    "assets",
    "other",
];

#[derive(Default, Clone, Debug, Serialize)]
pub struct Count {
    pub count: usize,
    pub bytes: u64,
}
impl Count {
    fn add(&mut self, bytes: u64) {
        self.count += 1;
        self.bytes += bytes;
    }
}

#[derive(Debug, Serialize)]
pub struct Statistics {
    pub scope: Option<String>,
    pub files: Count,
    pub folders: usize,
    pub categories: BTreeMap<String, Count>,
    pub extensions: BTreeMap<String, Count>,
    pub auxiliary: BTreeMap<String, Count>,
}

pub fn collect(index: &Index, scope: &Scope) -> Result<Statistics> {
    scoped_items(index, false, scope)?;
    let mut result = Statistics {
        scope: scope.path().map(str::to_owned),
        files: Count::default(),
        folders: 0,
        categories: CATEGORIES
            .iter()
            .map(|&c| (c.into(), Count::default()))
            .collect(),
        extensions: BTreeMap::new(),
        auxiliary: BTreeMap::new(),
    };
    let mut classified = BTreeMap::new();
    for r in &index.resources {
        let included = scope.matches(&r.path);
        // Unknown resource siblings follow the logical resource scope.
        for &id in &r.entries {
            classified.insert(id, ("unknown", included));
        }
        classified.insert(r.pathname, ("pathname", included));
        if let Some(id) = r.meta {
            classified.insert(id, ("meta", included));
        }
        if let Some(id) = r.preview {
            classified.insert(id, ("preview", included));
        }
        if r.folder && included {
            result.folders += 1;
        }
        if let Some(id) = r.asset {
            let entry = &index.entries[id];
            classified.insert(
                id,
                (
                    if r.folder {
                        "folder_payload"
                    } else {
                        "resource_payload"
                    },
                    included,
                ),
            );
            if included && !r.folder && entry.kind == "file" {
                let extension = extension(&r.path);
                result.files.add(entry.size);
                result
                    .categories
                    .get_mut(category(&extension))
                    .unwrap()
                    .add(entry.size);
                result
                    .extensions
                    .entry(extension)
                    .or_default()
                    .add(entry.size);
            }
        }
    }
    for entry in &index.entries {
        let (kind, included) = classified.get(&entry.id).copied().unwrap_or_else(|| {
            let kind = match entry.path.as_str() {
                ".icon.png" | ".cover.png" | "packagemanagermanifest/asset" => "special",
                _ => "unknown",
            };
            (kind, scope.matches(&entry.path))
        });
        if !included || kind == "resource_payload" {
            continue;
        }
        let kind = if entry.kind == "directory" {
            "tar_directories"
        } else if entry.kind != "file" {
            "unsupported"
        } else {
            kind
        };
        result
            .auxiliary
            .entry(kind.into())
            .or_default()
            .add(entry.size);
    }
    Ok(result)
}

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .filter(|ext| !ext.is_empty())
        .map_or_else(
            || "<none>".into(),
            |ext| format!(".{}", ext.to_string_lossy().to_lowercase()),
        )
}

fn category(extension: &str) -> &'static str {
    match extension {
        ".anim" => "animations",
        ".controller" | ".overridecontroller" => "controllers",
        ".fbx" | ".obj" | ".blend" | ".dae" | ".3ds" | ".dxf" | ".max" | ".ma" | ".mb"
        | ".gltf" | ".glb" => "models",
        ".wav" | ".mp3" | ".ogg" | ".aif" | ".aiff" | ".flac" | ".aac" | ".m4a" | ".mod"
        | ".it" | ".s3m" | ".xm" => "audio",
        ".mp4" | ".mov" | ".webm" | ".ogv" | ".avi" | ".m4v" | ".wmv" | ".mpeg" | ".mpg" => "video",
        ".png" | ".jpg" | ".jpeg" | ".tga" | ".tif" | ".tiff" | ".psd" | ".psb" | ".bmp"
        | ".gif" | ".exr" | ".hdr" | ".svg" | ".iff" | ".pict" => "images",
        ".rendertexture" | ".cubemap" | ".dds" | ".ktx" | ".ktx2" => "textures",
        ".cs" | ".js" | ".boo" | ".lua" | ".py" => "scripts",
        ".asmdef" | ".asmref" => "assembly_definitions",
        ".dll" | ".so" | ".dylib" | ".bundle" | ".a" | ".lib" | ".bc" | ".aar" | ".jar"
        | ".wasm" => "plugin_binaries",
        ".unity" => "scenes",
        ".prefab" => "prefabs",
        ".mat" => "materials",
        ".shader" | ".compute" | ".shadergraph" | ".shadersubgraph" | ".shadervariants"
        | ".hlsl" | ".cginc" | ".glsl" | ".cg" => "shaders",
        ".uxml" | ".uss" | ".guiskin" => "ui",
        ".ttf" | ".otf" | ".fontsettings" => "fonts",
        ".physicmaterial" | ".physicsmaterial2d" => "physics",
        ".terrainlayer" => "terrain",
        ".txt" | ".md" | ".markdown" | ".json" | ".yaml" | ".yml" | ".xml" | ".csv" | ".tsv"
        | ".ini" | ".cfg" | ".toml" | ".html" | ".css" | ".rst" => "text",
        ".asset" => "assets",
        _ => "other",
    }
}

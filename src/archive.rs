use crate::paths::normalize;
use anyhow::{Context, Result, ensure};
use flate2::read::MultiGzDecoder;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    time::SystemTime,
};
use tar::{Archive, Entry, Header};
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub entries: usize,
    pub entry_bytes: u64,
    pub expanded_bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 1_000_000,
            entry_bytes: 8 << 30,
            expanded_bytes: 64 << 30,
        }
    }
}
pub struct BoundedReader<R> {
    inner: R,
    count: u64,
    limit: u64,
}
impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let available = self.limit.saturating_sub(self.count).saturating_add(1);
        let length = buf.len().min(available.min(usize::MAX as u64) as usize);
        let n = self.inner.read(&mut buf[..length])?;
        self.count += n as u64;
        if self.count > self.limit {
            return Err(io::Error::other("expanded archive size limit exceeded"));
        }
        Ok(n)
    }
}
pub type PackageReader = BoundedReader<MultiGzDecoder<BufReader<File>>>;

pub fn scan<F>(path: &Path, limits: Limits, mut visitor: F) -> Result<()>
where
    F: FnMut(usize, &mut Entry<'_, PackageReader>) -> Result<()>,
{
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BoundedReader {
        inner: MultiGzDecoder::new(BufReader::new(file)),
        count: 0,
        limit: limits.expanded_bytes,
    };
    let mut archive = Archive::new(reader);
    for (id, entry) in archive.entries().context("read tar entries")?.enumerate() {
        ensure!(id < limits.entries, "archive entry count limit exceeded");
        let mut entry = entry.context("invalid tar entry")?;
        ensure!(
            entry.size() <= limits.entry_bytes,
            "entry size limit exceeded"
        );
        visitor(id, &mut entry)?;
        io::copy(&mut entry, &mut io::sink()).context("read entry payload")?;
    }
    // Tar stops at its end marker. Drain the compression stream to verify CRC,
    // truncation, concatenated members and the expanded-size limit as well.
    let mut decoder = archive.into_inner();
    io::copy(&mut decoder, &mut io::sink()).context("invalid gzip trailer or truncated package")?;
    Ok(())
}

pub fn small_read(reader: &mut impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "metadata exceeds {limit} bytes"
    );
    Ok(bytes)
}

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub severity: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct PhysicalEntry {
    pub id: usize,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub kind: String,
    #[serde(skip)]
    pub header: Header,
}
#[derive(Clone, Debug, Serialize)]
pub struct Resource {
    pub guid: String,
    pub path: String,
    pub folder: bool,
    pub asset: Option<usize>,
    pub meta: Option<usize>,
    pub preview: Option<usize>,
    pub pathname: usize,
    pub entries: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct Index {
    pub source: PathBuf,
    pub entries: Vec<PhysicalEntry>,
    pub resources: Vec<Resource>,
    pub diagnostics: Vec<Diagnostic>,
    pub limits: Limits,
    file_size: u64,
    modified: Option<SystemTime>,
}

#[derive(Default, Debug)]
pub struct MetaInfo {
    pub guid: Option<String>,
    pub folder: bool,
}
pub fn meta_info(bytes: &[u8]) -> Result<MetaInfo> {
    let text = std::str::from_utf8(bytes)
        .context("meta is not UTF-8")?
        .trim_start_matches('\u{feff}');
    let docs = YamlLoader::load_from_str(text).context("invalid meta YAML")?;
    ensure!(
        docs.len() == 1 && docs[0].as_hash().is_some(),
        "meta must contain one YAML mapping"
    );
    let doc = &docs[0];
    let guid = match &doc["guid"] {
        Yaml::String(s) | Yaml::Real(s) => Some(s.to_lowercase()),
        Yaml::Integer(_) => {
            // Unity GUIDs are unquoted strings, even when they contain only digits.
            // YAML scalar inference must not erase leading zeros.
            let pattern =
                regex::Regex::new(r"(?m)^guid:[ \t]*([0-9A-Fa-f]{32})[ \t]*(?:#.*)?\r?$")?;
            Some(
                pattern
                    .captures(text)
                    .context("invalid numeric meta GUID")?[1]
                    .to_lowercase(),
            )
        }
        Yaml::BadValue | Yaml::Null => None,
        _ => anyhow::bail!("meta guid must be a scalar string"),
    };
    if let Some(guid) = &guid {
        ensure!(is_guid(guid), "invalid meta GUID: {guid}");
    }
    let folder = matches!(&doc["folderAsset"], Yaml::Boolean(true))
        || doc["folderAsset"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case("yes") || s.eq_ignore_ascii_case("true"));
    Ok(MetaInfo { guid, folder })
}
pub fn is_guid(guid: &str) -> bool {
    guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Index {
    pub fn open(path: impl AsRef<Path>, limits: Limits) -> Result<Self> {
        let path = path.as_ref();
        let stat = path.metadata()?;
        let mut index = Self {
            source: path.to_owned(),
            entries: vec![],
            resources: vec![],
            diagnostics: vec![],
            limits,
            file_size: stat.len(),
            modified: stat.modified().ok(),
        };
        let mut names = BTreeMap::new();
        let mut pathnames = BTreeMap::<String, (usize, String)>::new();
        let mut metas = BTreeMap::<String, MetaInfo>::new();
        scan(path, limits, |id, entry| {
            let name = std::str::from_utf8(&entry.path_bytes())
                .context("tar path is not UTF-8")?
                .to_owned();
            if matches!(name.as_str(), "." | "./" | "") && entry.header().entry_type().is_dir() {
                // Keep the record ID stable across scans, but root isn't a resource.
                index.entries.push(PhysicalEntry {
                    id,
                    name,
                    path: String::new(),
                    size: entry.size(),
                    kind: "directory".into(),
                    header: entry.header().clone(),
                });
                return Ok(());
            }
            let normalized =
                normalize(&name).with_context(|| format!("unsafe tar path {name:?}"))?;
            if names.insert(normalized.clone(), id).is_some() {
                index.error(format!("duplicate normalized tar path: {normalized}"));
            }
            let typ = entry.header().entry_type();
            let kind = if typ.is_dir() {
                "directory"
            } else if typ.is_file() {
                "file"
            } else {
                "unsupported"
            };
            index.entries.push(PhysicalEntry {
                id,
                name,
                path: normalized.clone(),
                size: entry.size(),
                kind: kind.into(),
                header: entry.header().clone(),
            });
            if !typ.is_file() {
                return Ok(());
            }
            if let Some((group, part)) = normalized.split_once('/')
                && !part.contains('/')
                && group != "packagemanagermanifest"
            {
                if part == "pathname" {
                    let bytes = small_read(entry, 64 << 10)?;
                    let text = std::str::from_utf8(&bytes)
                        .context("pathname is not UTF-8")?
                        .trim_start_matches('\u{feff}');
                    let first = text
                        .split('\n')
                        .next()
                        .unwrap_or("")
                        .trim_end_matches(['\r', '\0']);
                    match normalize(first) {
                        Ok(p) => {
                            pathnames.insert(group.into(), (id, p));
                        }
                        Err(e) => index.error(format!("invalid pathname in {group}: {e}")),
                    }
                } else if part == "asset.meta" {
                    match meta_info(&small_read(entry, 4 << 20)?) {
                        Ok(meta) => {
                            metas.insert(group.into(), meta);
                        }
                        Err(e) => index.error(format!("{normalized}: {e:#}")),
                    }
                }
            }
            Ok(())
        })?;
        let mut paths = BTreeSet::new();
        for (guid, (pathname, path)) in pathnames {
            if !paths.insert(path.clone()) {
                index.error(format!("duplicate resource pathname: {path}"));
            }
            let lookup = |part: &str| names.get(&format!("{guid}/{part}")).copied();
            let asset = lookup("asset");
            let meta = lookup("asset.meta");
            let preview = lookup("preview.png");
            let info = metas.get(&guid);
            if let Some(info) = info {
                if let Some(meta_guid) = &info.guid {
                    if !meta_guid.eq_ignore_ascii_case(&guid) {
                        index.error(format!("GUID mismatch: {guid} vs meta {meta_guid}"));
                    }
                } else {
                    index.error(format!("meta without GUID: {path}"));
                }
                if asset.is_none() && !info.folder {
                    index.error(format!("missing asset payload: {path}"));
                }
            }
            let folder = info.is_some_and(|m| m.folder) || (asset.is_none() && meta.is_none());
            if folder && asset.is_some_and(|id| index.entries[id].size > 0) {
                index.error(format!("folder has nonempty asset payload: {path}"));
            }
            let entries = index
                .entries
                .iter()
                .filter(|e| e.path == guid || e.path.starts_with(&format!("{guid}/")))
                .map(|e| e.id)
                .collect();
            index.resources.push(Resource {
                guid,
                path,
                folder,
                asset,
                meta,
                preview,
                pathname,
                entries,
            });
        }
        let resource_groups: BTreeSet<_> =
            index.resources.iter().map(|r| r.guid.as_str()).collect();
        let warnings: Vec<_> = index
            .entries
            .iter()
            .filter_map(|e| {
                let (group, part) = e.path.split_once('/')?;
                (is_guid(group)
                    && matches!(part, "asset" | "asset.meta")
                    && !resource_groups.contains(group))
                .then(|| format!("orphan resource entry (no pathname): {}", e.path))
            })
            .collect();
        for message in warnings {
            index.diagnostics.push(Diagnostic {
                severity: "warning".into(),
                message,
            });
        }
        index.resources.sort_by(|a, b| a.path.cmp(&b.path));
        index.check_unchanged()?;
        Ok(index)
    }
    fn error(&mut self, message: String) {
        self.diagnostics.push(Diagnostic {
            severity: "error".into(),
            message,
        });
    }
    pub fn ensure_valid(&self) -> Result<()> {
        let errors: Vec<_> = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == "error")
            .map(|d| d.message.as_str())
            .collect();
        ensure!(errors.is_empty(), "invalid package:\n{}", errors.join("\n"));
        Ok(())
    }
    pub fn check_unchanged(&self) -> Result<()> {
        let stat = self.source.metadata()?;
        ensure!(
            stat.len() == self.file_size && stat.modified().ok() == self.modified,
            "input package changed during operation"
        );
        Ok(())
    }
}

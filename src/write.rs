use crate::{
    archive::{Index, Limits, scan},
    paths::{normalize, reject_symlinks},
};
use anyhow::{Context, Result, ensure};
use flate2::{Compression, GzBuilder, write::GzEncoder};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
};
use tar::{Builder, Header};
use tempfile::NamedTempFile;

#[derive(Clone, Debug)]
pub enum Data {
    File(PathBuf),
    Bytes(Vec<u8>),
    Directory,
}
impl Data {
    pub fn size(&self) -> Result<u64> {
        match self {
            Self::File(p) => {
                reject_symlinks(p)?;
                ensure!(p.is_file(), "not a regular file: {}", p.display());
                Ok(p.metadata()?.len())
            }
            Self::Bytes(b) => Ok(b.len() as u64),
            Self::Directory => Ok(0),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Addition {
    pub path: String,
    pub data: Data,
}
#[derive(Clone, Debug)]
pub enum Edit {
    Delete,
    Replace(Data),
}
type OutputTar<'a> = Builder<GzEncoder<&'a mut File>>;

fn long_extension(builder: &mut OutputTar<'_>, typ: u8, value: &[u8]) -> Result<()> {
    let mut header = Header::new_gnu();
    header.set_path("././@LongLink")?;
    header.set_mode(0o644);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_entry_type(tar::EntryType::new(typ));
    header.set_size(value.len() as u64 + 1);
    header.set_cksum();
    builder.append(&header, Cursor::new(value).chain(io::repeat(0).take(1)))?;
    Ok(())
}

fn append_exact(
    builder: &mut OutputTar<'_>,
    header: &mut Header,
    name: &str,
    data: impl io::Read,
) -> Result<()> {
    if header.path_bytes().is_empty() && header.set_path(name).is_err() {
        header.set_path("uup-long-path")?;
    }
    if header.path_bytes().as_ref() != name.as_bytes() {
        long_extension(builder, b'L', name.as_bytes())?;
    }
    header.set_cksum();
    builder.append(header, data)?;
    Ok(())
}

fn append(
    builder: &mut OutputTar<'_>,
    name: &str,
    data: &Data,
    original: Option<&Header>,
) -> Result<()> {
    let mut header = original.cloned().unwrap_or_else(Header::new_gnu);
    if original.is_none() {
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
    }
    header.set_size(data.size()?);
    match data {
        Data::Directory => {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_mode(0o755);
            header.set_size(0);
            append_exact(builder, &mut header, name, io::empty())?;
        }
        Data::Bytes(b) => {
            header.set_entry_type(tar::EntryType::Regular);
            append_exact(builder, &mut header, name, Cursor::new(b))?;
        }
        Data::File(path) => {
            header.set_entry_type(tar::EntryType::Regular);
            append_exact(builder, &mut header, name, File::open(path)?)?;
        }
    }
    Ok(())
}

fn atomic_package<F>(
    output: &Path,
    force: bool,
    source: Option<&Path>,
    limits: Limits,
    build: F,
) -> Result<()>
where
    F: FnOnce(&mut OutputTar<'_>) -> Result<()>,
{
    reject_symlinks(output)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    if let Some(source) = source {
        let resolved = if output.exists() {
            output.canonicalize()?
        } else {
            parent
                .canonicalize()?
                .join(output.file_name().context("output filename missing")?)
        };
        ensure!(
            resolved != source.canonicalize()?,
            "output must differ from input package"
        );
    }
    ensure!(
        force || !output.try_exists()?,
        "output exists (use --force): {}",
        output.display()
    );
    ensure!(!output.exists() || output.is_file(), "output is not a file");
    let mut temp = NamedTempFile::new_in(parent)?;
    {
        let encoder = GzBuilder::new()
            .mtime(0)
            .operating_system(255)
            .write(temp.as_file_mut(), Compression::default());
        let mut builder = Builder::new(encoder);
        build(&mut builder)?;
        builder.finish()?;
        builder.into_inner()?.finish()?.flush()?;
    }
    temp.as_file().sync_all()?;
    Index::open(temp.path(), limits)?
        .ensure_valid()
        .context("validate generated package")?;
    reject_symlinks(output)?;
    if force {
        temp.persist(output).map_err(|e| e.error)?;
    } else {
        temp.persist_noclobber(output).map_err(|e| e.error)?;
    }
    Ok(())
}

pub fn create(output: &Path, force: bool, additions: &[Addition], limits: Limits) -> Result<()> {
    for addition in additions {
        normalize(&addition.path)?;
        addition.data.size()?;
    }
    atomic_package(output, force, None, limits, |builder| {
        for addition in additions {
            append(builder, &addition.path, &addition.data, None)?;
        }
        Ok(())
    })
}

pub fn rewrite(
    index: &Index,
    output: &Path,
    force: bool,
    edits: &BTreeMap<usize, Edit>,
    additions: &[Addition],
) -> Result<()> {
    index.check_unchanged()?;
    // Allow repairing a malformed resource via a raw edit. Generated output must validate.
    for addition in additions {
        normalize(&addition.path)?;
        addition.data.size()?;
    }
    atomic_package(
        output,
        force,
        Some(&index.source),
        index.limits,
        |builder| {
            scan(&index.source, index.limits, |id, entry| {
                if matches!(edits.get(&id), Some(Edit::Delete)) {
                    return Ok(());
                }
                let expected = &index.entries[id];
                ensure!(
                    entry.path_bytes().as_ref() == expected.name.as_bytes()
                        && entry.size() == expected.size,
                    "input package changed during rewrite"
                );
                let typ = entry.header().entry_type();
                ensure!(
                    !typ.is_gnu_sparse(),
                    "sparse tar entries cannot be safely rewritten"
                );
                // PAX extended attributes must survive, including attributes not understood here.
                let mut pax = Vec::new();
                if let Some(attrs) = entry.pax_extensions()? {
                    for attr in attrs {
                        let attr = attr?;
                        pax.push((attr.key()?.to_owned(), attr.value_bytes().to_vec()));
                    }
                }
                if !pax.is_empty() {
                    if matches!(edits.get(&id), Some(Edit::Replace(_))) {
                        // Recomputed values come from the new header; avoid stale PAX sizes.
                        pax.retain(|(key, _)| !matches!(key.as_str(), "size"));
                    }
                    builder.append_pax_extensions(
                        pax.iter()
                            .map(|(key, value)| (key.as_str(), value.as_slice())),
                    )?;
                }
                if let Some(Edit::Replace(data)) = edits.get(&id) {
                    append(builder, &expected.name, data, Some(entry.header()))?;
                } else {
                    let mut header = entry.header().clone();
                    header.set_size(expected.size);
                    if typ.is_symlink() || typ.is_hard_link() {
                        ensure!(
                            expected.size == 0,
                            "link entries with payloads are unsupported"
                        );
                        let link = entry.link_name_bytes().context("link target missing")?;
                        if header.link_name_bytes().as_deref() != Some(link.as_ref()) {
                            long_extension(builder, b'K', &link)?;
                        }
                    }
                    append_exact(builder, &mut header, &expected.name, entry)?;
                }
                Ok(())
            })?;
            index.check_unchanged()?;
            for addition in additions {
                append(builder, &addition.path, &addition.data, None)?;
            }
            Ok(())
        },
    )
}

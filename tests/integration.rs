use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tar::{Archive, Builder, Header};
use tempfile::TempDir;
use uup_cli::{
    archive::{Index, Limits},
    extract::{self, Extraction},
    mutate,
    pack::{self, PackOptions},
    query::Selector,
    write,
};

const A: &str = "11111111111111111111111111111111";
const B: &str = "22222222222222222222222222222222";
fn meta(guid: &str, folder: bool) -> Vec<u8> {
    pack::generated_meta(guid, folder)
}
fn fixture(root: &Path, entries: Vec<(String, Vec<u8>)>) -> PathBuf {
    let path = root.join("source.unitypackage");
    let mut tar = Builder::new(GzEncoder::new(
        File::create(&path).unwrap(),
        Compression::default(),
    ));
    for (name, bytes) in entries {
        let mut header = Header::new_ustar();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(123);
        tar.append_data(&mut header, name, Cursor::new(bytes))
            .unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
    path
}
fn record(guid: &str, path: &str, data: Option<&[u8]>, folder: bool) -> Vec<(String, Vec<u8>)> {
    let mut out = vec![
        (format!("{guid}/pathname"), path.as_bytes().to_vec()),
        (format!("{guid}/asset.meta"), meta(guid, folder)),
    ];
    if let Some(data) = data {
        out.push((format!("{guid}/asset"), data.to_vec()));
    }
    out
}
fn contents(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut archive = Archive::new(GzDecoder::new(File::open(path).unwrap()));
    archive
        .entries()
        .unwrap()
        .map(|e| {
            let mut e = e.unwrap();
            let name = e.path().unwrap().to_string_lossy().into_owned();
            let mut bytes = vec![];
            e.read_to_end(&mut bytes).unwrap();
            (name, bytes)
        })
        .collect()
}
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_uup-cli"))
        .args(args)
        .output()
        .unwrap()
}
fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}
fn ok(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn index(p: &Path) -> Index {
    Index::open(p, Limits::default()).unwrap()
}
fn select(p: &str) -> Selector {
    Selector {
        path: Some(p.into()),
        ..Default::default()
    }
}
fn png() -> Vec<u8> {
    let mut data = vec![];
    {
        let mut encoder = png::Encoder::new(&mut data, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255, 0, 0, 255]).unwrap();
    }
    data
}

#[test]
fn full_extract_preserves_non_assets_metadata_unknown_and_empty_folders() {
    let dir = TempDir::new().unwrap();
    let mut entries = record(A, "Assets/空目录", None, true);
    entries.extend(record(B, "PackageSettings/value.json", Some(b"{}"), false));
    entries.extend([
        (".icon.png".into(), png()),
        (
            "packagemanagermanifest/asset".into(),
            b"{\"dependencies\":{}}".to_vec(),
        ),
        ("other/data.bin".into(), vec![0, 255, 17]),
        (format!("{B}/preview.png"), png()),
    ]);
    let package = fixture(dir.path(), entries);
    let out = dir.path().join("out");
    extract::extract(&index(&package), &out, &Extraction::default()).unwrap();
    assert!(out.join("Assets/空目录").is_dir());
    assert!(out.join("Assets/空目录.meta").is_file());
    assert_eq!(
        fs::read(out.join("PackageSettings/value.json")).unwrap(),
        b"{}"
    );
    assert_eq!(fs::read(out.join("other/data.bin")).unwrap(), [0, 255, 17]);
    assert!(out.join(".icon.png").is_file());
    assert!(out.join("packagemanagermanifest/asset").is_file());
    assert!(out.join(format!("{B}/preview.png")).is_file());
}
#[test]
fn unordered_bom_crlf_and_dot_prefix_records_work() {
    let dir = TempDir::new().unwrap();
    let package = fixture(
        dir.path(),
        vec![
            (format!("./{A}/asset"), b"payload".to_vec()),
            (format!("./{A}/asset.meta"), meta(A, false)),
            (
                format!("./{A}/pathname"),
                "\u{feff}Assets/中文.txt\r\n00".as_bytes().to_vec(),
            ),
        ],
    );
    let idx = index(&package);
    idx.ensure_valid().unwrap();
    assert_eq!(idx.resources[0].path, "Assets/中文.txt");
    let output = ok(run(&["cat", path(&package), "--path", "Assets/中文.txt"]));
    assert_eq!(output.stdout, b"payload");
}
#[test]
fn project_settings_without_meta_and_zero_length_files() {
    let dir = TempDir::new().unwrap();
    let package = fixture(
        dir.path(),
        vec![
            (
                format!("{A}/pathname"),
                b"ProjectSettings/ProjectSettings.asset".to_vec(),
            ),
            (format!("{A}/asset"), vec![]),
        ],
    );
    let idx = index(&package);
    assert!(!idx.resources[0].folder);
    idx.ensure_valid().unwrap();
    let out = dir.path().join("out");
    extract::extract(&idx, &out, &Extraction::default()).unwrap();
    assert!(out.join("ProjectSettings/ProjectSettings.asset").is_file());
}
#[test]
fn suffix_lookalikes_stay_raw_and_do_not_create_resources() {
    let dir = TempDir::new().unwrap();
    let package = fixture(
        dir.path(),
        vec![
            ("extras/myasset".into(), b"raw".to_vec()),
            ("extras/notpathname".into(), b"unknown".to_vec()),
        ],
    );
    let idx = index(&package);
    assert!(idx.resources.is_empty());
    assert_eq!(
        ok(run(&["cat", path(&package), "--entry", "extras/myasset"])).stdout,
        b"raw"
    );
}
#[test]
fn selected_extract_includes_meta_and_does_not_extract_unselected_content() {
    let dir = TempDir::new().unwrap();
    let mut entries = record(A, "Assets/a.txt", Some(b"A"), false);
    entries.extend(record(B, "Assets/b.txt", Some(b"B"), false));
    let package = fixture(dir.path(), entries);
    let out = dir.path().join("out");
    ok(run(&[
        "extract",
        path(&package),
        "-o",
        path(&out),
        "--glob",
        "**/a.txt",
    ]));
    assert_eq!(fs::read(out.join("Assets/a.txt")).unwrap(), b"A");
    assert!(out.join("Assets/a.txt.meta").is_file());
    assert!(!out.join("Assets/b.txt").exists());
    assert_eq!(
        run(&[
            "extract",
            path(&package),
            "-o",
            path(&out),
            "--path",
            "Assets/a.txt"
        ])
        .status
        .code(),
        Some(1)
    );
}
#[test]
fn read_only_commands_do_not_require_temp_directory() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/a.txt", Some(b"A"), false));
    let invalid = dir.path().join("not-there");
    for args in [
        vec!["info", "--json"],
        vec!["list", "--json"],
        vec!["find", "a.txt", "--json"],
        vec!["cat", "--path", "Assets/a.txt"],
        vec!["show", "--path", "Assets/a.txt", "--json"],
        vec!["verify", "--json"],
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_uup-cli"));
        command.arg(args[0]).arg(&package).args(&args[1..]);
        command
            .env("TMPDIR", &invalid)
            .env("TMP", &invalid)
            .env("TEMP", &invalid);
        ok(command.output().unwrap());
    }
    assert!(!invalid.exists());
    assert_eq!(
        run(&["cat", path(&package), "--path", "Assets/missing"])
            .status
            .code(),
        Some(3)
    );
    assert_eq!(
        run(&["find", path(&package), "missing"]).status.code(),
        Some(3)
    );
    assert_eq!(
        run(&["cat", path(&package), "--path", "Assets/a.txt", "--guid", A])
            .status
            .code(),
        Some(2)
    );
}
#[test]
fn invalid_crc_and_truncated_payloads_fail() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/a.txt", Some(b"A"), false));
    let original = fs::read(&package).unwrap();
    let mut bad = original.clone();
    let n = bad.len();
    bad[n - 8] ^= 1;
    fs::write(&package, bad).unwrap();
    assert!(Index::open(&package, Limits::default()).is_err());
    assert_eq!(run(&["verify", path(&package)]).status.code(), Some(1));
    fs::write(&package, &original[..original.len() - 6]).unwrap();
    assert!(Index::open(&package, Limits::default()).is_err());
}
#[test]
fn traversal_and_duplicate_normalized_paths_are_rejected() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "../escape", Some(b"x"), false));
    let out = dir.path().join("out");
    assert!(extract::extract(&index(&package), &out, &Extraction::default()).is_err());
    assert!(!out.exists());
    let package = fixture(
        dir.path(),
        vec![
            ("same".into(), b"a".to_vec()),
            ("./same".into(), b"b".to_vec()),
        ],
    );
    assert!(index(&package).ensure_valid().is_err());
}
#[test]
fn logical_raw_collision_fails_before_writing() {
    let dir = TempDir::new().unwrap();
    let mut entries = record(A, "Assets/a.txt", Some(b"A"), false);
    entries.push(("Assets/a.txt".into(), b"unknown raw".to_vec()));
    let package = fixture(dir.path(), entries);
    let out = dir.path().join("out");
    assert!(extract::extract(&index(&package), &out, &Extraction::default()).is_err());
    assert!(!out.exists());
}
#[cfg(unix)]
#[test]
fn extraction_refuses_existing_symlink_parents() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/a.txt", Some(b"A"), false));
    let out = dir.path().join("out");
    fs::create_dir(&out).unwrap();
    let external = dir.path().join("external");
    fs::create_dir(&external).unwrap();
    std::os::unix::fs::symlink(&external, out.join("Assets")).unwrap();
    assert!(extract::extract(&index(&package), &out, &Extraction::default()).is_err());
    assert!(!external.join("a.txt").exists());
}
#[test]
fn repack_and_replace_preserve_every_other_payload_and_source() {
    let dir = TempDir::new().unwrap();
    let mut entries = record(A, "Assets/a.bin", Some(&[0, 255]), false);
    entries.extend([
        (format!("{A}/preview.png"), png()),
        (format!("{A}/unknown-extension"), b"opaque".to_vec()),
        (".icon.png".into(), png()),
        (
            "packagemanagermanifest/asset".into(),
            b"{\"dependencies\":{},\"extra\":true}".to_vec(),
        ),
    ]);
    let package = fixture(dir.path(), entries);
    let original = fs::read(&package).unwrap();
    let expected = contents(&package);
    let repacked = dir.path().join("repacked.unitypackage");
    write::rewrite(&index(&package), &repacked, false, &BTreeMap::new(), &[]).unwrap();
    assert_eq!(contents(&repacked), expected);
    let replacement = dir.path().join("replacement");
    fs::write(&replacement, b"updated").unwrap();
    let edited = dir.path().join("edited.unitypackage");
    mutate::replace(
        &index(&package),
        &replacement,
        &select("Assets/a.bin"),
        &edited,
        false,
    )
    .unwrap();
    let mut new = expected;
    new.insert(format!("{A}/asset"), b"updated".to_vec());
    assert_eq!(contents(&edited), new);
    assert_eq!(fs::read(&package).unwrap(), original);
}
#[test]
fn failed_write_preserves_source_and_existing_output() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/a", Some(b"a"), false));
    let original = fs::read(&package).unwrap();
    assert!(write::rewrite(&index(&package), &package, true, &BTreeMap::new(), &[]).is_err());
    assert_eq!(fs::read(&package).unwrap(), original);
    let dest = dir.path().join("dest");
    fs::write(&dest, b"keep me").unwrap();
    let additions = vec![write::Addition {
        path: format!("{B}/pathname"),
        data: write::Data::Bytes(b"Assets/a".to_vec()),
    }];
    assert!(write::rewrite(&index(&package), &dest, true, &BTreeMap::new(), &additions).is_err());
    assert_eq!(fs::read(&dest).unwrap(), b"keep me");
}
#[test]
fn add_non_assets_and_remove_recursive_resources_and_unknown_siblings() {
    let dir = TempDir::new().unwrap();
    let mut entries = record(A, "Assets/Folder", None, true);
    entries.extend(record(B, "Assets/Folder/child.txt", Some(b"child"), false));
    entries.push((format!("{B}/custom"), b"extra".to_vec()));
    let package = fixture(dir.path(), entries);
    let dest = dir.path().join("removed");
    assert!(
        mutate::remove(
            &index(&package),
            &select("Assets/Folder"),
            false,
            &dest,
            false
        )
        .is_err()
    );
    mutate::remove(
        &index(&package),
        &select("Assets/Folder"),
        true,
        &dest,
        false,
    )
    .unwrap();
    assert!(contents(&dest).is_empty());
    let data = dir.path().join("data");
    fs::write(&data, b"settings").unwrap();
    let added = dir.path().join("added");
    mutate::add(
        &index(&package),
        &data,
        &select("PackageSettings/new.json"),
        None,
        false,
        &added,
        false,
    )
    .unwrap();
    assert!(
        index(&added)
            .resources
            .iter()
            .any(|r| r.path == "PackageSettings/new.json")
    );
}
#[test]
fn metadata_set_get_delete_and_invalid_png_are_transactional() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/a", Some(b"a"), false));
    let icon = dir.path().join("icon.png");
    fs::write(&icon, png()).unwrap();
    let out = dir.path().join("icon-package");
    ok(run(&[
        "metadata",
        path(&package),
        "set",
        "icon",
        "--file",
        path(&icon),
        "-o",
        path(&out),
    ]));
    assert_eq!(
        ok(run(&["metadata", path(&out), "get", "icon"])).stdout,
        fs::read(&icon).unwrap()
    );
    let manifest = dir.path().join("manifest.json");
    let data = b"{\"dependencies\":{},\"custom\":{\"keep\":true}}\n";
    fs::write(&manifest, data).unwrap();
    let newer = dir.path().join("manifest-package");
    ok(run(&[
        "metadata",
        path(&out),
        "set",
        "manifest",
        "--file",
        path(&manifest),
        "-o",
        path(&newer),
    ]));
    assert_eq!(
        ok(run(&["metadata", path(&newer), "get", "manifest"])).stdout,
        data
    );
    let removed = dir.path().join("no-icon");
    ok(run(&[
        "metadata",
        path(&newer),
        "remove",
        "icon",
        "-o",
        path(&removed),
    ]));
    assert!(!contents(&removed).contains_key(".icon.png"));
    fs::write(&icon, b"fake PNG").unwrap();
    let failed = dir.path().join("failed");
    assert_eq!(
        run(&[
            "metadata",
            path(&newer),
            "set",
            "icon",
            "--file",
            path(&icon),
            "-o",
            path(&failed)
        ])
        .status
        .code(),
        Some(1)
    );
    assert!(!failed.exists());
}
#[test]
fn pack_preserves_metas_directory_names_and_deterministic_guids() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source");
    let folder = source.join("with.meta.in-name");
    fs::create_dir_all(&folder).unwrap();
    fs::write(source.join("with.meta.in-name.meta"), meta(A, true)).unwrap();
    fs::write(folder.join("file"), b"bytes").unwrap();
    fs::write(folder.join("file.meta"), meta(B, false)).unwrap();
    let output = dir.path().join("packed");
    pack::pack(
        &source,
        &output,
        &PackOptions {
            prefix: "Assets".into(),
            ..Default::default()
        },
        Limits::default(),
    )
    .unwrap();
    let idx = index(&output);
    assert!(
        idx.resources
            .iter()
            .any(|r| r.path == "Assets/with.meta.in-name/file" && r.guid == B)
    );
    assert_eq!(
        contents(&output)[&format!("{B}/asset.meta")],
        meta(B, false)
    );
    let orphan = source.join("orphan.meta");
    fs::write(&orphan, meta(B, false)).unwrap();
    assert!(
        pack::pack(
            &source,
            &dir.path().join("failed"),
            &PackOptions {
                prefix: "Assets".into(),
                ..Default::default()
            },
            Limits::default()
        )
        .is_err()
    );
}
#[test]
fn upm_both_layouts_include_ignored_folders_preserve_guids_and_report_dependencies() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("upm");
    fs::create_dir_all(source.join("Runtime")).unwrap();
    fs::create_dir_all(source.join("Samples~/Demo")).unwrap();
    fs::create_dir_all(source.join("Documentation~")).unwrap();
    fs::create_dir_all(source.join(".git")).unwrap();
    fs::write(source.join(".git/config"), b"exclude").unwrap();
    fs::write(source.join("package.json"),b"{\"name\":\"com.test.demo\",\"version\":\"1.2.3\",\"dependencies\":{\"com.other.lib\":\"1.0.0\"}}").unwrap();
    fs::write(source.join("Runtime.meta"), meta(A, true)).unwrap();
    fs::write(source.join("Runtime/Foo.cs"), b"class Foo {}").unwrap();
    fs::write(source.join("Runtime/Foo.cs.meta"), meta(B, false)).unwrap();
    fs::write(source.join("Samples~/Demo/data.txt"), b"sample").unwrap();
    fs::write(source.join("Documentation~/index.md"), b"doc").unwrap();
    let before = fs::read(source.join("Runtime/Foo.cs.meta")).unwrap();
    for layout in ["packages", "assets"] {
        let out = dir.path().join(layout);
        let response = ok(run(&[
            "from-upm",
            path(&source),
            path(&out),
            "--layout",
            layout,
            "--generate-meta",
            "--json",
        ]));
        let report: serde_json::Value = serde_json::from_slice(&response.stdout).unwrap();
        assert!(!report["warnings"].as_array().unwrap().is_empty());
        let idx = index(&out);
        let prefix = if layout == "packages" {
            "Packages"
        } else {
            "Assets"
        };
        assert!(
            idx.resources
                .iter()
                .any(|r| r.path == format!("{prefix}/com.test.demo/Runtime/Foo.cs") && r.guid == B)
        );
        assert!(
            idx.resources
                .iter()
                .any(|r| r.path.ends_with("Samples~/Demo/data.txt") && r.meta.is_none())
        );
        assert!(!idx.resources.iter().any(|r| r.path.contains(".git")));
    }
    assert_eq!(
        fs::read(source.join("Runtime/Foo.cs.meta")).unwrap(),
        before
    );
    assert!(!source.join("package.json.meta").exists());
}
#[test]
fn limits_apply_to_payload_and_expanded_stream() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/a", Some(&[0; 2048]), false));
    assert!(
        Index::open(
            &package,
            Limits {
                entry_bytes: 1024,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Index::open(
            &package,
            Limits {
                expanded_bytes: 1024,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Index::open(
            &package,
            Limits {
                entries: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn long_raw_names_and_pax_attributes_survive_repack() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("pax");
    let name = format!("extras/{}/data", "long".repeat(70));
    let mut builder = Builder::new(GzEncoder::new(
        File::create(&path).unwrap(),
        Compression::default(),
    ));
    builder
        .append_pax_extensions([("vendor.property", b"opaque".as_slice())])
        .unwrap();
    let mut header = Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o600);
    header.set_mtime(777);
    header.set_uid(17);
    header.set_gid(23);
    builder
        .append_data(&mut header, &name, Cursor::new(b"raw"))
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap();
    let out = dir.path().join("repacked");
    write::rewrite(&index(&path), &out, false, &BTreeMap::new(), &[]).unwrap();
    assert_eq!(contents(&out)[&name], b"raw");
    let mut archive = Archive::new(GzDecoder::new(File::open(out).unwrap()));
    let mut entries = archive.entries().unwrap();
    let mut entry = entries.next().unwrap().unwrap();
    assert_eq!(entry.header().uid().unwrap(), 17);
    assert_eq!(entry.header().mode().unwrap(), 0o600);
    let attrs: Vec<_> = entry
        .pax_extensions()
        .unwrap()
        .unwrap()
        .map(|a| {
            let a = a.unwrap();
            (a.key().unwrap().to_owned(), a.value_bytes().to_vec())
        })
        .collect();
    assert!(attrs.contains(&("vendor.property".into(), b"opaque".to_vec())));
}
#[test]
fn generated_metadata_is_stable_across_repeated_builds_and_content_edits() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("file.txt"), b"one").unwrap();
    let options = PackOptions {
        prefix: "Assets/Demo".into(),
        generate_meta: true,
        ..Default::default()
    };
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    pack::pack(&source, &first, &options, Limits::default()).unwrap();
    fs::write(source.join("file.txt"), b"two").unwrap();
    pack::pack(&source, &second, &options, Limits::default()).unwrap();
    assert_eq!(
        index(&first).resources[0].guid,
        index(&second).resources[0].guid
    );
    let guid = &index(&first).resources[0].guid;
    assert_eq!(
        contents(&first)[&format!("{guid}/asset.meta")],
        contents(&second)[&format!("{guid}/asset.meta")]
    );
    assert!(!source.join("file.txt.meta").exists());
}

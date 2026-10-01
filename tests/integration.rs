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
    Command::new(env!("CARGO_BIN_EXE_uup"))
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
fn scoped_queries_alias_virtual_directories_and_legacy_find() {
    let dir = TempDir::new().unwrap();
    let mut entries = record(A, "Assets/中文/run.ANIM", Some(b"clip"), false);
    entries.extend(record(B, "Assets/中文Extra/code.cs", Some(b"code"), false));
    let package = fixture(dir.path(), entries);
    let listed = ok(run(&["list", path(&package), "Assets/中文", "--json"]));
    assert_eq!(
        listed.stdout,
        ok(run(&[
            "ls",
            path(&package),
            "--path",
            "./Assets\\中文/",
            "--json"
        ]))
        .stdout
    );
    let items: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(items.as_array().unwrap().len(), 1);
    assert_eq!(items[0]["path"], "Assets/中文/run.ANIM");
    for args in [
        vec![
            "find",
            path(&package),
            "**/*.ANIM",
            "Assets/中文",
            "--glob",
            "--json",
        ],
        vec![
            "find",
            path(&package),
            "run",
            "--path",
            "Assets/中文",
            "--json",
        ],
        vec!["list", path(&package), "Assets/中文/run.ANIM", "--json"],
    ] {
        assert_eq!(ok(run(&args)).stdout, listed.stdout);
    }
    assert!(
        ok(run(&["find", path(&package), "code"]))
            .stdout
            .ends_with("Assets/中文Extra/code.cs\n".as_bytes())
    );
    let raw: serde_json::Value =
        serde_json::from_slice(&ok(run(&["ls", path(&package), A, "--raw", "--json"])).stdout)
            .unwrap();
    assert_eq!(raw.as_array().unwrap().len(), 3);
    for command in ["list", "info"] {
        let output = run(&[command, path(&package), "Assets/missing", "--json"]);
        assert_eq!(output.status.code(), Some(3));
        assert!(output.stdout.is_empty());
    }
    assert_eq!(
        run(&["find", path(&package), "code", "Assets/中文"])
            .status
            .code(),
        Some(3)
    );
    assert_eq!(
        run(&["ls", path(&package), "Assets", "--path", "Assets"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        run(&["info", path(&package), "../Assets"]).status.code(),
        Some(1)
    );
    assert_eq!(
        ok(run(&["ls", path(&package), ".", "--json"])).stdout,
        ok(run(&["list", path(&package), "--json"])).stdout
    );
}

#[test]
fn scoped_extraction_includes_metas_and_intersects_selectors() {
    let dir = TempDir::new().unwrap();
    let mut entries = record(A, "Packages/com.demo/Runtime/code.cs", Some(b"code"), false);
    entries.extend(record(
        B,
        "Packages/com.demoExtra/Runtime/code.cs",
        Some(b"other"),
        false,
    ));
    entries.push(("unknown/value.bin".into(), vec![1, 2, 3]));
    let package = fixture(dir.path(), entries);
    let out = dir.path().join("out");
    ok(run(&[
        "extract",
        path(&package),
        "Packages/com.demo",
        "-o",
        path(&out),
    ]));
    assert_eq!(
        fs::read(out.join("Packages/com.demo/Runtime/code.cs")).unwrap(),
        b"code"
    );
    assert!(out.join("Packages/com.demo/Runtime/code.cs.meta").exists());
    assert!(!out.join("Packages/com.demoExtra").exists());
    assert!(!out.join("unknown").exists());
    let file = dir.path().join("file");
    ok(run(&[
        "extract",
        path(&package),
        "Packages/com.demo/Runtime/code.cs",
        "-o",
        path(&file),
        "--no-meta",
    ]));
    assert!(!file.join("Packages/com.demo/Runtime/code.cs.meta").exists());
    let miss = dir.path().join("miss");
    assert_eq!(
        run(&[
            "extract",
            path(&package),
            "Packages/com.demo",
            "--guid",
            B,
            "-o",
            path(&miss)
        ])
        .status
        .code(),
        Some(3)
    );
    assert!(!miss.exists());
    // Exact --path keeps its 2.0.0 semantics; positional scopes allow virtual directories.
    assert_eq!(
        run(&[
            "extract",
            path(&package),
            "--path",
            "Packages/com.demo",
            "-o",
            path(&miss)
        ])
        .status
        .code(),
        Some(3)
    );
    let raw = dir.path().join("raw");
    ok(run(&[
        "extract",
        path(&package),
        A,
        "--raw",
        "-o",
        path(&raw),
    ]));
    assert_eq!(fs::read(raw.join(format!("{A}/asset"))).unwrap(), b"code");
    assert!(!raw.join(B).exists());
    let unknown = dir.path().join("unknown-out");
    ok(run(&[
        "extract",
        path(&package),
        "unknown",
        "-o",
        path(&unknown),
    ]));
    assert_eq!(
        fs::read(unknown.join("unknown/value.bin")).unwrap(),
        [1, 2, 3]
    );
}

#[test]
fn info_counts_unity_extensions_disjoint_categories_and_scopes() {
    let dir = TempDir::new().unwrap();
    let cases = [
        ("run.anim", "animations"),
        ("JUMP.ANIM", "animations"),
        ("walk.controller", "controllers"),
        ("walk.overrideController", "controllers"),
        ("character.fbx", "models"),
        ("song.ogg", "audio"),
        ("movie.webm", "video"),
        ("image.png", "images"),
        ("map.renderTexture", "textures"),
        ("code.cs", "scripts"),
        ("demo.asmdef", "assembly_definitions"),
        ("plugin.dll", "plugin_binaries"),
        ("native.so", "plugin_binaries"),
        ("level.unity", "scenes"),
        ("unit.prefab", "prefabs"),
        ("surface.mat", "materials"),
        ("node.shadergraph", "shaders"),
        ("window.uxml", "ui"),
        ("face.ttf", "fonts"),
        ("surface.physicMaterial", "physics"),
        ("soil.terrainlayer", "terrain"),
        ("readme.md", "text"),
        ("value.asset", "assets"),
        ("README", "other"),
        ("unknown.weird", "other"),
    ];
    let mut entries = vec![];
    for (i, (name, _)) in cases.iter().enumerate() {
        entries.extend(record(
            &format!("{:032x}", i + 1),
            &format!("Packages/com.demo/{name}"),
            Some(b"abc"),
            false,
        ));
    }
    entries.extend(record(A, "Assets/empty", None, true));
    entries.extend(record(B, "PackageSettings/empty.json", Some(b""), false));
    entries.extend([
        (format!("{B}/preview.png"), png()),
        (".icon.png".into(), png()),
        ("opaque.anim".into(), b"not a resource".to_vec()),
    ]);
    let package = fixture(dir.path(), entries);
    let report: serde_json::Value =
        serde_json::from_slice(&ok(run(&["info", path(&package), "--json"])).stdout).unwrap();
    let stats = &report["statistics"];
    assert_eq!(stats["files"]["count"], cases.len() + 1);
    assert_eq!(stats["files"]["bytes"], cases.len() * 3);
    assert_eq!(stats["folders"], 1);
    assert_eq!(stats["extensions"][".anim"]["count"], 2);
    assert_eq!(stats["extensions"]["<none>"]["count"], 1);
    assert_eq!(stats["categories"]["animations"]["count"], 2);
    assert_eq!(stats["auxiliary"]["preview"]["count"], 1);
    assert_eq!(stats["auxiliary"]["special"]["count"], 1);
    assert_eq!(stats["auxiliary"]["unknown"]["count"], 1);
    for field in ["extensions", "categories"] {
        let values: Vec<_> = stats[field].as_object().unwrap().values().collect();
        assert_eq!(
            values
                .iter()
                .map(|v| v["count"].as_u64().unwrap())
                .sum::<u64>(),
            stats["files"]["count"].as_u64().unwrap()
        );
        assert_eq!(
            values
                .iter()
                .map(|v| v["bytes"].as_u64().unwrap())
                .sum::<u64>(),
            stats["files"]["bytes"].as_u64().unwrap()
        );
    }
    for (_, category) in cases {
        assert!(stats["categories"][category]["count"].as_u64().unwrap() > 0);
    }
    let scoped = ok(run(&["info", path(&package), "PackageSettings", "--json"]));
    assert_eq!(
        scoped.stdout,
        ok(run(&[
            "info",
            path(&package),
            "--path",
            "PackageSettings",
            "--json"
        ]))
        .stdout
    );
    let scoped: serde_json::Value = serde_json::from_slice(&scoped.stdout).unwrap();
    assert_eq!(scoped["statistics"]["files"]["count"], 1);
    assert_eq!(scoped["statistics"]["auxiliary"]["preview"]["count"], 1);
    let text =
        String::from_utf8(ok(run(&["info", path(&package), "Packages/com.demo/run.anim"])).stdout)
            .unwrap();
    assert!(text.contains("1 resource files, 3 bytes"));
    assert!(text.contains(".anim\t1\t3"));
    assert!(text.contains("animations\t1\t3"));
}

fn metadata_fixture(root: &Path) -> PathBuf {
    let mut entries = record(
        A,
        "Packages/com.demo/package.json",
        Some(
            br#"{
  "name":"com.demo", "version":"1.0.0", "description":"keep unless deleted",
  "dependencies":{"com.old":"1.2.3"}, "custom":{"enabled":true},
  "a/b":{"~key":1}, "items":["first","second"]
}
"#,
        ),
        false,
    );
    entries.extend(record(
        B,
        "PackageSettings/editor.json",
        Some(br#"{"enabled":true,"unknown":[1,2]}"#),
        false,
    ));
    entries.extend(record(
        "33333333333333333333333333333333",
        "ProjectSettings/ProjectSettings.asset",
        Some(b"%YAML 1.1\n---\nPlayerSettings:\n  value: 1\n"),
        false,
    ));
    entries.extend(record(
        "44444444444444444444444444444444",
        "Assets/Settings/local.json",
        Some(b"{}"),
        false,
    ));
    entries.extend(record(
        "55555555555555555555555555555555",
        "Packages/manifest.json",
        Some(br#"{"dependencies":{"com.demo":"file:demo"},"custom":42}"#),
        false,
    ));
    entries.extend([
        (
            "extras/package.json".into(),
            br#"{"name":"com.other","version":"1.0.0"}"#.to_vec(),
        ),
        (
            "UserSettings/user.json".into(),
            b"{\"color\":\"blue\"}".to_vec(),
        ),
        (
            "Packages/com.other/Runtime/PluginSettings/config.bin".into(),
            vec![0, 255, 1],
        ),
        (
            "packagemanagermanifest/asset".into(),
            br#"{"dependencies":{"com.demo":"1.0.0"}}"#.to_vec(),
        ),
        (".icon.png".into(), png()),
        (".cover.png".into(), png()),
        (format!("{A}/preview.png"), png()),
        (format!("{A}/unknown"), b"opaque".to_vec()),
    ]);
    fixture(root, entries)
}

#[test]
fn metadata_discovers_summarizes_and_selects_without_temp_files() {
    let dir = TempDir::new().unwrap();
    let package = metadata_fixture(dir.path());
    let expected = contents(&package);
    let report = ok(run(&["metadata", path(&package), "--json"]));
    assert_eq!(
        report.stdout,
        ok(run(&["metadata", path(&package), "summary", "--json"])).stdout
    );
    let report: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(report["entries"].as_array().unwrap().len(), 10);
    assert_eq!(report["counts"]["package-json"], 2);
    assert_eq!(report["counts"]["settings"], 4);
    let entries = report["entries"].as_array().unwrap();
    assert!(
        !entries
            .iter()
            .any(|i| i["path"] == "Assets/Settings/local.json")
    );
    let icon = entries.iter().find(|i| i["kind"] == "icon").unwrap();
    assert_eq!(icon["details"]["width"], 1);
    assert_eq!(icon["details"]["validation"], "header-only");
    let upm = entries.iter().find(|i| i["guid"] == A).unwrap();
    assert_eq!(upm["details"]["name"], "com.demo");
    assert_eq!(upm["details"]["dependencies"], 1);
    assert_eq!(upm["entry"], format!("{A}/asset"));
    let ambiguous = run(&["metadata", path(&package), "get", "package-json"]);
    assert_eq!(ambiguous.status.code(), Some(1));
    let message = String::from_utf8(ambiguous.stderr).unwrap();
    assert!(
        message.contains("extras/package.json")
            && message.contains("Packages/com.demo/package.json")
    );
    assert_eq!(
        run(&["metadata", path(&package), "get", "icon", "--guid", A])
            .status
            .code(),
        Some(3)
    );
    assert_eq!(
        run(&[
            "metadata",
            path(&package),
            "get",
            "package-json",
            "--guid",
            A,
            "--path",
            "Packages/com.demo/package.json"
        ])
        .status
        .code(),
        Some(2)
    );
    let invalid = dir.path().join("missing-temp");
    for args in [
        vec!["metadata", path(&package), "--json"],
        vec![
            "metadata",
            path(&package),
            "summary",
            "--path",
            "Packages/com.demo",
            "--json",
        ],
        vec!["metadata", path(&package), "list", "settings", "--json"],
        vec![
            "metadata",
            path(&package),
            "get",
            "package-json",
            "--guid",
            A,
            "--json",
        ],
        vec!["info", path(&package), "Packages/com.demo", "--json"],
        vec!["ls", path(&package), "PackageSettings", "--json"],
    ] {
        let output = ok(Command::new(env!("CARGO_BIN_EXE_uup"))
            .args(&args)
            .env("TMPDIR", &invalid)
            .env("TMP", &invalid)
            .env("TEMP", &invalid)
            .output()
            .unwrap());
        if args.contains(&"get") {
            assert_eq!(output.stdout, expected[&format!("{A}/asset")]);
        }
    }
    assert!(!invalid.exists());
    let raw = ok(run(&[
        "metadata",
        path(&package),
        "get",
        "settings",
        "--entry",
        "UserSettings/user.json",
    ]));
    assert_eq!(raw.stdout, expected["UserSettings/user.json"]);
    let project = ok(run(&[
        "metadata",
        path(&package),
        "get",
        "project-manifest",
    ]));
    assert_eq!(
        project.stdout,
        expected["55555555555555555555555555555555/asset"]
    );
}

#[test]
fn metadata_dump_preserves_bytes_and_preflights_all_destinations() {
    let dir = TempDir::new().unwrap();
    let package = metadata_fixture(dir.path());
    let expected = contents(&package);
    let items: serde_json::Value =
        serde_json::from_slice(&ok(run(&["metadata", path(&package), "list", "--json"])).stdout)
            .unwrap();
    let out = dir.path().join("dump");
    let result = ok(run(&[
        "metadata",
        path(&package),
        "dump",
        "-o",
        path(&out),
        "--json",
    ]));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["dumped"],
        10
    );
    for item in items.as_array().unwrap() {
        assert_eq!(
            fs::read(out.join(item["path"].as_str().unwrap())).unwrap(),
            expected[item["entry"].as_str().unwrap()]
        );
    }
    assert!(!out.join(format!("{A}/preview.png")).exists());
    assert!(!out.join("Packages/com.demo/package.json.meta").exists());
    let conflict = dir.path().join("conflict");
    fs::create_dir_all(conflict.join("extras")).unwrap();
    fs::write(conflict.join("extras/package.json"), b"keep").unwrap();
    assert_eq!(
        run(&["metadata", path(&package), "dump", "-o", path(&conflict)])
            .status
            .code(),
        Some(1)
    );
    assert!(!conflict.join(".icon.png").exists());
    assert!(!conflict.join("Packages").exists());
    assert_eq!(
        fs::read(conflict.join("extras/package.json")).unwrap(),
        b"keep"
    );
    let filtered = dir.path().join("filtered");
    ok(run(&[
        "metadata",
        path(&package),
        "dump",
        "--kind",
        "settings",
        "--path",
        "PackageSettings",
        "-o",
        path(&filtered),
    ]));
    assert_eq!(
        fs::read(filtered.join("PackageSettings/editor.json")).unwrap(),
        expected[&format!("{B}/asset")]
    );
    assert!(!filtered.join("ProjectSettings").exists());
    let missing = dir.path().join("missing");
    assert_eq!(
        run(&[
            "metadata",
            path(&package),
            "dump",
            "--path",
            "Nowhere",
            "-o",
            path(&missing)
        ])
        .status
        .code(),
        Some(3)
    );
    assert!(!missing.exists());
}

#[test]
fn metadata_dump_rejects_logical_raw_and_file_directory_collisions() {
    let dir = TempDir::new().unwrap();
    for raw_path in [
        "Packages/com.demo/package.json",
        "Packages/com.demo/package.json/nested/package.json",
    ] {
        let mut entries = record(A, "Packages/com.demo/package.json", Some(b"{}"), false);
        entries.push((raw_path.into(), b"{}".to_vec()));
        let package = fixture(dir.path(), entries);
        let out = dir.path().join("out");
        assert_eq!(
            run(&["metadata", path(&package), "dump", "-o", path(&out)])
                .status
                .code(),
            Some(1)
        );
        assert!(!out.exists());
    }
}

#[cfg(unix)]
#[test]
fn metadata_dump_refuses_symlink_parents() {
    let dir = TempDir::new().unwrap();
    let package = metadata_fixture(dir.path());
    let output = dir.path().join("output");
    let elsewhere = dir.path().join("elsewhere");
    fs::create_dir(&output).unwrap();
    fs::create_dir(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, output.join("Packages")).unwrap();
    assert_eq!(
        run(&["metadata", path(&package), "dump", "-o", path(&output)])
            .status
            .code(),
        Some(1)
    );
    assert!(!output.join(".icon.png").exists());
    assert!(!elsewhere.join("com.demo").exists());
}

#[test]
fn metadata_json_pointer_edits_preserve_identity_unknown_fields_and_entries() {
    let dir = TempDir::new().unwrap();
    let package = metadata_fixture(dir.path());
    let original = fs::read(&package).unwrap();
    let expected = contents(&package);
    let out = dir.path().join("edited.unitypackage");
    ok(run(&[
        "metadata",
        path(&package),
        "edit",
        "package-json",
        "--guid",
        A,
        "--set",
        "/version=\"2.3.4\"",
        "--set",
        "/dependencies/com.sample=\"2.0.0\"",
        "--set",
        "/new/nested=true",
        "--set",
        "/a~1b/~0key=2",
        "--set",
        "/items/-={\"x\":1}",
        "--set",
        "/items/0=\"changed\"",
        "--delete",
        "/description",
        "--delete",
        "/items/1",
        "-o",
        path(&out),
    ]));
    index(&out).ensure_valid().unwrap();
    let mut actual = contents(&out);
    let payload = actual.remove(&format!("{A}/asset")).unwrap();
    assert!(payload.ends_with(b"\n"));
    let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(value["version"], "2.3.4");
    assert_eq!(value["name"], "com.demo");
    assert_eq!(value["custom"]["enabled"], true);
    assert_eq!(value["dependencies"]["com.old"], "1.2.3");
    assert_eq!(value["dependencies"]["com.sample"], "2.0.0");
    assert_eq!(value["new"]["nested"], true);
    assert_eq!(value["a/b"]["~key"], 2);
    assert!(value.get("description").is_none());
    assert_eq!(value["items"], serde_json::json!(["changed", {"x":1}]));
    let mut untouched = expected;
    untouched.remove(&format!("{A}/asset"));
    assert_eq!(actual, untouched);
    assert_eq!(fs::read(&package).unwrap(), original);
    let idx = index(&out);
    let resource = idx.resources.iter().find(|r| r.guid == A).unwrap();
    assert_eq!(resource.path, "Packages/com.demo/package.json");
    assert!(resource.meta.is_some() && resource.preview.is_some());
}

#[test]
fn invalid_metadata_edits_leave_source_and_existing_destination_untouched() {
    let dir = TempDir::new().unwrap();
    let package = metadata_fixture(dir.path());
    let original = fs::read(&package).unwrap();
    let out = dir.path().join("keep.unitypackage");
    fs::write(&out, b"keep me").unwrap();
    for (operation, expression) in [
        ("--set", "version=1"),
        ("--set", "/bad~2key=1"),
        ("--set", "/items/99=1"),
        ("--set", "/items/01=1"),
        ("--set", "/items/-/child=1"),
        ("--set", "/version/child=1"),
        ("--set", "/new=not-json"),
        ("--set", "=[]"),
        ("--delete", "/missing"),
        ("--delete", ""),
    ] {
        let output = run(&[
            "metadata",
            path(&package),
            "edit",
            "package-json",
            "--guid",
            A,
            operation,
            expression,
            "-o",
            path(&out),
            "--force",
        ]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{expression}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&out).unwrap(), b"keep me");
        assert_eq!(fs::read(&package).unwrap(), original);
    }
    assert_eq!(
        run(&[
            "metadata",
            path(&package),
            "edit",
            "package-json",
            "--guid",
            A,
            "-o",
            path(&out)
        ])
        .status
        .code(),
        Some(2)
    );
    assert_eq!(
        run(&[
            "metadata",
            path(&package),
            "edit",
            "settings",
            "--path",
            "ProjectSettings/ProjectSettings.asset",
            "--set",
            "/x=1",
            "-o",
            path(&out),
            "--force"
        ])
        .status
        .code(),
        Some(1)
    );
    assert_eq!(
        run(&[
            "metadata",
            path(&package),
            "edit",
            "project-manifest",
            "--set",
            "/dependencies/com.bad=42",
            "-o",
            path(&out),
            "--force"
        ])
        .status
        .code(),
        Some(1)
    );
    assert_eq!(fs::read(&out).unwrap(), b"keep me");
}

#[test]
fn metadata_set_and_remove_handle_resource_and_raw_settings() {
    let dir = TempDir::new().unwrap();
    let package = metadata_fixture(dir.path());
    let expected = contents(&package);
    let file = dir.path().join("replacement");
    let updated = b"%YAML 1.1\n---\nPlayerSettings:\n  value: 2\n";
    fs::write(&file, updated).unwrap();
    let out = dir.path().join("yaml.unitypackage");
    ok(run(&[
        "metadata",
        path(&package),
        "set",
        "settings",
        "--path",
        "ProjectSettings/ProjectSettings.asset",
        "--file",
        path(&file),
        "-o",
        path(&out),
    ]));
    let mut new = expected.clone();
    new.insert(
        "33333333333333333333333333333333/asset".into(),
        updated.to_vec(),
    );
    assert_eq!(contents(&out), new);
    let raw = dir.path().join("raw.unitypackage");
    fs::write(&file, [255, 0, 8]).unwrap();
    ok(run(&[
        "metadata",
        path(&package),
        "set",
        "settings",
        "--entry",
        "Packages/com.other/Runtime/PluginSettings/config.bin",
        "--file",
        path(&file),
        "-o",
        path(&raw),
    ]));
    let mut new = expected.clone();
    new.insert(
        "Packages/com.other/Runtime/PluginSettings/config.bin".into(),
        vec![255, 0, 8],
    );
    assert_eq!(contents(&raw), new);
    let removed = dir.path().join("removed.unitypackage");
    ok(run(&[
        "metadata",
        path(&package),
        "remove",
        "package-json",
        "--guid",
        A,
        "-o",
        path(&removed),
    ]));
    let mut new = expected.clone();
    new.retain(|name, _| !name.starts_with(&format!("{A}/")));
    assert_eq!(contents(&removed), new);
    index(&removed).ensure_valid().unwrap();
    let removed_raw = dir.path().join("removed-raw.unitypackage");
    ok(run(&[
        "metadata",
        path(&package),
        "remove",
        "settings",
        "--entry",
        "UserSettings/user.json",
        "-o",
        path(&removed_raw),
    ]));
    let mut new = expected;
    new.remove("UserSettings/user.json");
    assert_eq!(contents(&removed_raw), new);
    let absent = dir.path().join("absent.unitypackage");
    assert_eq!(
        run(&[
            "metadata",
            path(&package),
            "set",
            "settings",
            "--path",
            "PackageSettings/missing.bin",
            "--file",
            path(&file),
            "-o",
            path(&absent)
        ])
        .status
        .code(),
        Some(3)
    );
    assert!(!absent.exists());
}

#[test]
fn malformed_and_oversized_metadata_remain_discoverable_and_dumpable() {
    let dir = TempDir::new().unwrap();
    let large = vec![b' '; (16 << 20) + 1];
    let package = fixture(
        dir.path(),
        vec![
            ("package.json".into(), b"{bad".to_vec()),
            (".icon.png".into(), b"bad png".to_vec()),
            ("UserSettings/large.json".into(), large.clone()),
        ],
    );
    let summary: serde_json::Value =
        serde_json::from_slice(&ok(run(&["metadata", path(&package), "--json"])).stdout).unwrap();
    let entries = summary["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|i| i["warning"].is_string()));
    assert_eq!(
        ok(run(&["metadata", path(&package), "get", "package-json"])).stdout,
        b"{bad"
    );
    let out = dir.path().join("dump");
    ok(run(&["metadata", path(&package), "dump", "-o", path(&out)]));
    assert_eq!(
        fs::read(out.join("UserSettings/large.json")).unwrap(),
        large
    );
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
        let mut command = Command::new(env!("CARGO_BIN_EXE_uup"));
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
    let a = index(&first);
    let b = index(&second);
    let guid = &a
        .resources
        .iter()
        .find(|r| r.path == "Assets/Demo/file.txt")
        .unwrap()
        .guid;
    assert_eq!(
        guid,
        &b.resources
            .iter()
            .find(|r| r.path == "Assets/Demo/file.txt")
            .unwrap()
            .guid
    );
    assert_eq!(
        contents(&first)[&format!("{guid}/asset.meta")],
        contents(&second)[&format!("{guid}/asset.meta")]
    );
    assert!(!source.join("file.txt.meta").exists());
}

#[test]
fn numeric_and_scientific_looking_guids_keep_all_32_digits() {
    for guid in [
        "00000000000000000000000000000000",
        "00000000000000000000000000000001",
        "000000000000000000000000000000e1",
    ] {
        let dir = TempDir::new().unwrap();
        let package = fixture(dir.path(), record(guid, "Assets/file", Some(b"x"), false));
        index(&package).ensure_valid().unwrap();
        assert_eq!(index(&package).resources[0].guid, guid);
    }
}
#[test]
fn raw_extraction_can_recover_semantically_invalid_resources() {
    let dir = TempDir::new().unwrap();
    let package = fixture(
        dir.path(),
        vec![
            (format!("{A}/pathname"), b"../unsafe-logical-path".to_vec()),
            (format!("{A}/asset"), b"recover me".to_vec()),
        ],
    );
    let out = dir.path().join("raw");
    ok(run(&["extract", path(&package), "--raw", "-o", path(&out)]));
    assert_eq!(
        fs::read(out.join(format!("{A}/asset"))).unwrap(),
        b"recover me"
    );
    assert!(!dir.path().join("unsafe-logical-path").exists());
}
#[test]
fn nonzero_trailing_data_and_oversized_extension_are_rejected_before_allocation() {
    use std::io::Write;
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/file", Some(b"x"), false));
    let mut tar = vec![];
    GzDecoder::new(File::open(&package).unwrap())
        .read_to_end(&mut tar)
        .unwrap();
    tar.extend_from_slice(b"hidden content");
    let mut gzip = GzEncoder::new(File::create(&package).unwrap(), Compression::default());
    gzip.write_all(&tar).unwrap();
    gzip.finish().unwrap();
    assert!(Index::open(&package, Limits::default()).is_err());
    let mut header = Header::new_gnu();
    header.set_path("long").unwrap();
    header.set_entry_type(tar::EntryType::new(b'L'));
    header.set_size(1 << 30);
    header.set_cksum();
    let mut gzip = GzEncoder::new(File::create(&package).unwrap(), Compression::default());
    gzip.write_all(header.as_bytes()).unwrap();
    gzip.finish().unwrap();
    let error = Index::open(&package, Limits::default()).unwrap_err();
    assert!(error.to_string().contains("metadata exceeds"));
}
#[test]
fn directory_and_long_link_records_preserve_names_and_targets() {
    let dir = TempDir::new().unwrap();
    let package = dir.path().join("links");
    let target = "path/".repeat(60);
    let mut builder = Builder::new(GzEncoder::new(
        File::create(&package).unwrap(),
        Compression::default(),
    ));
    let mut header = Header::new_gnu();
    header.set_size(0);
    header.set_mode(0o755);
    header.set_entry_type(tar::EntryType::Directory);
    builder
        .append_data(&mut header, "extras/empty", Cursor::new([]))
        .unwrap();
    header.set_entry_type(tar::EntryType::Symlink);
    builder
        .append_link(&mut header, "extras/link", &target)
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap();
    let out = dir.path().join("repacked");
    write::rewrite(&index(&package), &out, false, &BTreeMap::new(), &[]).unwrap();
    let mut archive = Archive::new(GzDecoder::new(File::open(out).unwrap()));
    let links: Vec<_> = archive
        .entries()
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.path_bytes().into_owned(),
                e.link_name_bytes().map(|l| l.into_owned()),
            )
        })
        .collect();
    assert!(links.contains(&(b"extras/link".to_vec(), Some(target.into_bytes()))));
    let extracted = dir.path().join("extract");
    assert!(extract::extract(&index(&package), &extracted, &Extraction::default()).is_err());
    assert!(!extracted.exists());
}
#[test]
fn unknown_empty_directory_inside_resource_group_is_not_discarded() {
    let dir = TempDir::new().unwrap();
    let package = dir.path().join("directories");
    let mut builder = Builder::new(GzEncoder::new(
        File::create(&package).unwrap(),
        Compression::default(),
    ));
    for (name, bytes) in record(A, "Assets/file", Some(b"x"), false) {
        let mut h = Header::new_gnu();
        h.set_size(bytes.len() as u64);
        h.set_mode(0o644);
        builder
            .append_data(&mut h, name, Cursor::new(bytes))
            .unwrap();
    }
    let mut h = Header::new_gnu();
    h.set_size(0);
    h.set_mode(0o755);
    h.set_entry_type(tar::EntryType::Directory);
    builder
        .append_data(&mut h, format!("{A}/custom-empty"), Cursor::new([]))
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap();
    let out = dir.path().join("out");
    extract::extract(&index(&package), &out, &Extraction::default()).unwrap();
    assert!(out.join(format!("{A}/custom-empty")).is_dir());
}
#[test]
fn pax_size_override_and_replacement_keep_valid_alignment() {
    let dir = TempDir::new().unwrap();
    let package = dir.path().join("pax-size");
    let mut builder = Builder::new(GzEncoder::new(
        File::create(&package).unwrap(),
        Compression::default(),
    ));
    builder
        .append_pax_extensions([
            ("size", b"3".as_slice()),
            ("vendor.key", b"keep".as_slice()),
        ])
        .unwrap();
    let mut header = Header::new_ustar();
    header.set_path("extras/file").unwrap();
    header.set_mode(0o644);
    header.set_size(0);
    header.set_cksum();
    builder.append(&header, Cursor::new(b"abc")).unwrap();
    builder.into_inner().unwrap().finish().unwrap();
    let repacked = dir.path().join("repacked");
    write::rewrite(&index(&package), &repacked, false, &BTreeMap::new(), &[]).unwrap();
    assert_eq!(contents(&repacked)["extras/file"], b"abc");
    let new = dir.path().join("new");
    fs::write(&new, b"longer replacement").unwrap();
    let replaced = dir.path().join("replaced");
    mutate::replace(
        &index(&package),
        &new,
        &Selector {
            entry: Some("extras/file".into()),
            ..Default::default()
        },
        &replaced,
        false,
    )
    .unwrap();
    assert_eq!(contents(&replaced)["extras/file"], b"longer replacement");
}
#[test]
fn raw_add_replace_remove_do_not_touch_resources() {
    let dir = TempDir::new().unwrap();
    let package = fixture(dir.path(), record(A, "Assets/a", Some(b"a"), false));
    let file = dir.path().join("file");
    fs::write(&file, b"one").unwrap();
    let added = dir.path().join("added");
    ok(run(&[
        "add",
        path(&package),
        path(&file),
        "--entry",
        "extra/data",
        "-o",
        path(&added),
    ]));
    fs::write(&file, b"two").unwrap();
    let replaced = dir.path().join("replaced");
    ok(run(&[
        "replace",
        path(&added),
        path(&file),
        "--entry",
        "extra/data",
        "-o",
        path(&replaced),
    ]));
    assert_eq!(contents(&replaced)["extra/data"], b"two");
    let removed = dir.path().join("removed");
    ok(run(&[
        "remove",
        path(&replaced),
        "--entry",
        "extra/data",
        "-o",
        path(&removed),
    ]));
    assert_eq!(contents(&removed), contents(&package));
}
#[test]
fn malformed_png_end_and_invalid_manifest_leave_no_output() {
    let mut data = png();
    data.truncate(data.len() - 10);
    assert!(mutate::validate_metadata("icon", &data).is_err());
    assert!(mutate::validate_metadata("manifest", b"{\"dependencies\":{\"com.bad\":42}}").is_err());
}
#[test]
fn case_aliased_guids_and_file_directory_conflicts_are_invalid() {
    let dir = TempDir::new().unwrap();
    let lower = "abcdefabcdefabcdefabcdefabcdefab";
    let mut entries = record(lower, "Assets/a", Some(b"one"), false);
    entries.extend(record(
        &lower.to_uppercase(),
        "Assets/b",
        Some(b"two"),
        false,
    ));
    let package = fixture(dir.path(), entries);
    assert!(index(&package).ensure_valid().is_err());
    let mut entries = record(A, "Assets/file", Some(b"one"), false);
    entries.extend(record(B, "Assets/file/child", Some(b"two"), false));
    let package = fixture(dir.path(), entries);
    assert!(index(&package).ensure_valid().is_err());
}
#[test]
fn find_regex_part_selection_json_reports_and_build_alias_work() {
    let dir = TempDir::new().unwrap();
    let package = fixture(
        dir.path(),
        record(A, "Assets/a.cs", Some(b"class A {}"), false),
    );
    let result = ok(run(&[
        "find",
        path(&package),
        r"\.cs$",
        "--regex",
        "--json",
    ]));
    let items: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(items.as_array().unwrap().len(), 1);
    assert_eq!(
        ok(run(&["cat", path(&package), "--guid", A, "--part", "meta"])).stdout,
        meta(A, false)
    );
    let source = dir.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("a.cs"), b"class A {}").unwrap();
    fs::write(source.join("a.cs.meta"), meta(A, false)).unwrap();
    let built = dir.path().join("built");
    ok(run(&["build", path(&source), path(&built)]));
    let out = dir.path().join("repacked");
    let report = ok(run(&["repack", path(&built), "-o", path(&out), "--json"]));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&report.stdout).unwrap()["success"],
        true
    );
}

#[test]
fn posix_filenames_are_inspectable_on_every_platform() {
    let dir = TempDir::new().unwrap();
    let package = fixture(
        dir.path(),
        record(A, "Assets/name:with?chars.txt", Some(b"inspect me"), false),
    );
    ok(run(&["list", path(&package), "--json"]));
    assert_eq!(
        ok(run(&[
            "cat",
            path(&package),
            "--path",
            "Assets/name:with?chars.txt"
        ]))
        .stdout,
        b"inspect me"
    );
    let out = dir.path().join("out");
    let result = extract::extract(&index(&package), &out, &Extraction::default());
    if cfg!(windows) {
        assert!(result.is_err());
        assert!(!out.exists());
    } else {
        result.unwrap();
    }
}

#[test]
fn drive_and_unc_paths_cannot_hide_behind_dot_prefixes() {
    for path in [
        "C:/outside",
        "./C:/outside",
        "././c:relative",
        "\\\\host\\share",
        "/absolute",
        "Assets/../outside",
    ] {
        assert!(
            uup_cli::paths::normalize(path).is_err(),
            "accepted unsafe path: {path}"
        );
    }
}

#[test]
fn unknown_asset_meta_is_not_interpreted_without_a_pathname() {
    let dir = TempDir::new().unwrap();
    let package = fixture(
        dir.path(),
        vec![
            ("extras/asset.meta".into(), vec![0, 255, 23]),
            ("large-extra/asset.meta".into(), vec![0; 5 << 20]),
        ],
    );
    let idx = index(&package);
    idx.ensure_valid().unwrap();
    assert!(idx.resources.is_empty());
    let out = dir.path().join("repacked");
    write::rewrite(&idx, &out, false, &BTreeMap::new(), &[]).unwrap();
    assert_eq!(contents(&out), contents(&package));
}

#[cfg(target_os = "macos")]
#[test]
fn macos_system_directory_aliases_are_allowed() {
    for system in ["/var", "/tmp", "/etc"] {
        uup_cli::paths::reject_symlinks(Path::new(system)).unwrap();
    }
}

# uup-cli specification

Project/Cargo package name: uup-cli (ununitypackage-cli). Executable/command:
uup on macOS/Linux, uup.exe on Windows. The Rust library remains uup_cli.

## Content model

Resources have a pathname and optional asset, asset.meta, preview.png and unknown
siblings. Targets may be Assets, Packages, ProjectSettings, PackageSettings or
any safe relative path. Empty folders are real resources. Meta is optional on
read. Special entries include packagemanagermanifest/asset, .icon.png and .cover.png.
Everything else stays accessible as a raw tar entry and survives rewrites.
Match complete components, never filename suffixes. Normalize separators and
leading ./ for lookup; preserve original names and untouched payloads when writing.
Accept entry order variations, BOM, CRLF and legacy pathname newline + 00 tails.

## Commands

| Command | Contract |
| --- | --- |
| info PACKAGE [INSIDE_PATH] | Package summary, resource file categories/extensions, sizes and diagnostics; --path is an alternative scope |
| list PACKAGE [INSIDE_PATH] | Resources and other content; ls is an alias; --path is an alternative scope; --raw shows physical entries |
| find PACKAGE PATTERN [INSIDE_PATH] | Substring matching; --glob or --regex selects syntax; --path is an alternative scope |
| cat PACKAGE SELECTOR | Exact payload bytes to stdout |
| show PACKAGE SELECTOR | Metadata and bounded text / --hex preview |
| extract PACKAGE [INSIDE_PATH] -o DIR | Full/scoped extraction or --path/--guid/--glob/--entry selection |
| pack DIR OUTPUT | Pack a directory under --prefix (default Assets); build is an alias |
| repack PACKAGE -o OUTPUT | Recompress without dropping unknown content |
| add PACKAGE FILE --path TARGET -o OUTPUT | Add a resource; --entry adds raw content |
| replace PACKAGE FILE SELECTOR -o OUTPUT | Preserve resource GUID/meta by default |
| remove PACKAGE SELECTOR -o OUTPUT | Remove resource group; --recursive for descendants |
| metadata PACKAGE [ACTION] | Default summary; summary/list/dump/get/set/edit/remove; kinds include manifest/icon/cover/package-json/project-manifest/settings |
| from-upm DIR OUTPUT | Packages/name by default; --layout assets selects Assets/name |
| verify PACKAGE | Validate compression, tar structure, resource paths and conflicts |

Selectors --path, --guid and --entry are mutually exclusive for single-item
operations. --part asset/meta/preview selects a resource component; cat/show
default to asset. Extraction allows repeated selectors and includes associated
meta by default; --no-meta disables this. --raw extraction exports physical tar
paths. Logical full extraction includes special/unknown content at original
safe paths. Conflicts between logical and raw output are errors.

info/list/find/show/verify support --json (also a global option). Status messages
and diagnostics go to stderr; cat never mixes messages with payload bytes.
Exit codes: 0 success, 1 operational/validation error, 2 command usage, 3 no match.
All read-only commands stream the input without temporary files. show buffers
only the requested preview. Indexed pathname/meta values have size limits.
Archive scans include the gzip trailer/CRC, not just tar end markers.

## Writes and safety

Always require a distinct output package. Write to a sibling temporary file,
validate it, sync and persist. Existing outputs require --force. Never modify the
source or truncate an output before successful validation. Rewrites preserve
untouched payload, pathname, meta, preview and unknown entry bytes; compressed
bytes need not be identical. Preserve supported tar headers and PAX attributes;
unsupported archive constructs (global PAX and sparse tar) must fail rather than
silently lose content. Link targets survive repack; extraction does not create links.

Adding resources preserves supplied --meta and its GUID; otherwise use explicit
--generate-meta. Generated GUIDs depend on target path and a namespace, not file
contents. Detect duplicate GUIDs. Existing metas are not reserialized. Directory
packing requires meta on Unity-tracked resources unless --generate-meta is set;
ProjectSettings and ignored UPM directories are exceptions. Orphan metas fail.
Replacing asset bytes leaves meta and preview unchanged. --part meta validates
that the replacement GUID matches. Deleting a resource removes all siblings.

Reject absolute/drive/UNC paths, traversal, invalid components, normalized
duplicates, GUID conflicts and output collisions. Extraction never follows
symlinks/reparse points. Windows reserved names and case/Unicode collisions are
checked for actual filesystem output; inspection preserves names. Do not create
archive links during extraction. Apply configurable entry/count/expanded-size
limits. Inspection reports malformed resource records; extraction and mutation
fail when requested output would be ambiguous or unsafe. --raw / raw-only entry
extraction can recover payloads despite resource semantic errors, while still
checking physical paths and output safety. Concurrent filesystem modification
is unsupported; extraction failures may leave partial output.

## Metadata and UPM

manifest means packagemanagermanifest/asset, with a JSON object and string-valued
dependencies. package-json selects files named package.json; project-manifest
selects Packages/manifest.json. These stay ordinary resources or raw entries.
Metadata set replaces with user-supplied JSON, keeping its unknown keys and byte
formatting; unchanged JSON stays exact.
icon is .icon.png, cover is .cover.png; validate actual PNG decoding on set.

from-upm requires a valid root package.json with name and semantic version.
Preserve existing GUIDs/metas and include Runtime, Editor, Tests, Samples~,
Documentation~ and other package content. Exclude VCS/build/cache artifacts,
support additional --exclude globs. Tilde directories need no .meta. The input
tree is never changed, including when generating metadata. Packages/name is an
embedded package; Assets/name is an explicit mapping that may change semantics.
Keep dependency declarations, do not download dependencies or modify a project's
manifest. Emit warnings/report for custom registries, Git/file dependencies,
missing assemblies and path-sensitive code. No promise of automatic reference
repair or exact Unity-generated importer settings for synthesized metas.

## v2.1.0 path scopes

INSIDE_PATH is optional on info/list/find/extract. It normalizes separators and
leading ./, matches an exact file or a directory and its descendants using full
components (Assets/A never matches Assets/AB), and works without a folder record.
Omission or . selects the entire logical view. list/find/info also accept --path
instead of the positional scope, never both. find's existing PATTERN remains
required. --raw scopes refer to physical tar names. extract's existing --path
remains an exact selector; positional scope intersects its selector union. Metas
follow selected resources even when the scope is the exact resource file.
An explicit scope with no matches returns exit 3 before filesystem writes.

## v2.1.0 info statistics

Keep the existing package/entries/resources/payload_bytes/diagnostics JSON keys
as whole-archive information. Add scope and scoped file statistics: total count
and bytes, folders, categories and extensions. Each non-folder regular resource
payload is counted once; non-Assets paths participate equally. Extensions are
lowercased, no extension is represented by <none>, and categories are disjoint.
Include animation/controller/model/audio/video/image/texture/script/assembly
definition/plugin binary/scene/prefab/material/shader/UI/font/physics/terrain/
text/generic asset/other. Meta, pathname, preview, special and unknown raw entries
have separate count/byte summaries and are not resource payloads. File counts do
not infer embedded FBX clips, YAML object types or managed/native DLL identity.
Text and JSON reports expose the same counts, including zero-count categories.

## v2.1.0 metadata discovery and operations

Default metadata invocation and summary report an inventory plus bounded parsed
details (JSON name/version/dependencies and image dimensions where available).
list [KIND] reports paths, physical entries, GUIDs, sizes and formats. Discover
fixed manifest/icon/cover, any package.json, exact Packages/manifest.json, and
settings outside Assets in directory components named Settings or ending in
Settings (including ProjectSettings/PackageSettings/UserSettings). Classification
is a documented filename convention, not inference of every vendor format.

get/set/edit/remove accept one optional --path/--guid/--entry selector. With no
selector, exactly one candidate is required; multiple candidates fail and list
their paths. Fixed manifest/icon/cover set can add an absent special entry;
discovered kinds operate on existing files, with add available separately.
get emits exact bytes even with --json. set --file validates PNG or JSON where
applicable and preserves supplied bytes; YAML/binary settings remain opaque.
Resource removal deletes its whole record; raw removal deletes its entry.

edit uses repeatable --set '/pointer=JSON_VALUE' and --delete '/pointer'. JSON
Pointer escaping follows RFC 6901; missing object parents can be created, array
indexes must be valid and '-' appends on set. Apply sets in order, then deletes
in order. Invalid pointers/deletions/types fail before output. JSON documents
must remain objects; serialize edited JSON with indentation and a final newline,
retaining all untouched fields. Editing YAML/binary settings is not supported;
use set for whole-file replacement.

dump -o DIR exports discovered files at logical resource paths or physical raw
paths, retaining exact bytes. It excludes arbitrary asset metas/previews and
uses extraction preflight rules for existing files, links and path collisions.
summary/list/get never create temporary files or extract to disk. Summary parsing
is bounded and reports malformed/oversized metadata without dropping inventory.

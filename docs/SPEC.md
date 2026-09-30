# uup-cli specification

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
| info | Package summary and diagnostics |
| list | Resources and other content; --raw shows physical entries |
| find PACKAGE PATTERN | Substring matching; --glob or --regex selects that syntax |
| cat PACKAGE SELECTOR | Exact payload bytes to stdout |
| show PACKAGE SELECTOR | Metadata and bounded text / --hex preview |
| extract PACKAGE -o DIR | Full extraction or --path/--guid/--glob/--entry selection |
| pack DIR OUTPUT | Pack a directory under --prefix (default Assets); build is an alias |
| repack PACKAGE -o OUTPUT | Recompress without dropping unknown content |
| add PACKAGE FILE --path TARGET -o OUTPUT | Add a resource; --entry adds raw content |
| replace PACKAGE FILE SELECTOR -o OUTPUT | Preserve resource GUID/meta by default |
| remove PACKAGE SELECTOR -o OUTPUT | Remove resource group; --recursive for descendants |
| metadata PACKAGE get/set/remove KIND | KIND = manifest, icon or cover; set uses --file |
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
dependencies. package.json and Packages/manifest.json remain ordinary resources
edited through their explicit pathname. Metadata set replaces with user-supplied
JSON, keeping its unknown keys and byte formatting; unchanged JSON stays exact.
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

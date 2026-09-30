# Format evidence and compatibility

Unity packages are gzip-compressed tar archives. Typical resource layout:

```
<guid>/pathname
<guid>/asset        # absent for directories
<guid>/asset.meta   # optional (e.g. ProjectSettings)
<guid>/preview.png  # optional
packagemanagermanifest/asset
.icon.png
```

This is an observed layout, not a claim that every Unity version has the same
private schema. Unknown entries must be preserved and made available to users.
Non-GUID record identifiers can be inspected; meta GUID mismatches are diagnostics.
An explicit folderAsset: yes in meta or a path-only resource identifies a folder;
a zero-length asset is still a file unless folder metadata says otherwise.

## Evidence

- [Unity asset packages](https://docs.unity3d.com/6000.0/Documentation/Manual/AssetPackages.html)
- [Unity Asset Store exporter source](https://github.com/Unity-Technologies/UnityPlayground/blob/fabf7c8361d636db6010173feda5ca217ff3ad19/Packages/com.unity.asset-store-tools/Editor/Exporter/DefaultPackageExporter.cs):
  writes pathname, optional meta, file-only asset and package manager manifest;
  uses tar/7z and permits ProjectSettings assets without meta.
- [UPM manifest](https://docs.unity3d.com/6000.0/Documentation/Manual/upm-manifestPkg.html)
- [UPM layout](https://docs.unity3d.com/6000.0/Documentation/Manual/cus-layout.html):
  folders ending in ~ are ignored by the AssetDatabase and usually lack metas.
- [Project manifest](https://docs.unity3d.com/6000.0/Documentation/Manual/upm-manifestPrj.html)
- [Hybrid packages](https://github.com/needle-tools/hybrid-packages): third-party
  evidence for Packages paths, not a substitute for testing Unity imports.

## Legacy baseline (2026-09-30)

Eight synthetic archives were checked against the preexisting .NET binary:
canonical assets, ./ names and ProjectSettings without meta succeeded; ordinary
PackageSettings paths succeeded. Root icon, dependency manifest and unknown raw
entries were omitted. An empty folder produced its meta but not the directory.
These cases become Rust regressions. The README's duplicate-resource report has
not been reproduced in Unity; its root cause remains unknown.

Reader support: gzip, USTAR, GNU long names and PAX. Record original tar names and
effective headers. Safe lookup normalization must not silently rewrite unchanged
records. Long physical entry names require extension records when writing.
Gzip filename/comment/extra headers are container metadata and may be regenerated;
the package's tar contents are the preservation contract.

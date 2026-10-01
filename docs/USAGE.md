# uup-cli 使用文档与旧版迁移

> 本文包含 v2.1.0 新增的路径范围、文件统计和 metadata 发现/编辑。
> 这些功能已在 next 实现；版本同步和正式发布按 PROGRESS.md 记录推进。

适用于项目 `uup-cli 2.0.0`，全称 `ununitypackage-cli`。Rust 版本线从 2.0.0 承接原 .NET 的 1.0.0。
安装后的命令为 `uup`，可执行文件为 `uup`（macOS/Linux）或 `uup.exe`（Windows）。
工具直接处理 `.unitypackage` 的 gzip/tar 内容，可查看、解包、修改和打包，运行时无需 Unity Editor、.NET 或系统 `tar`。

## 1. 安装与快速开始

使用 Rust 1.88 或更新版本安装：

```sh
cargo install --git https://github.com/CKylinMC/ununitypackage --tag v2.0.0 --locked
uup --version
uup --help
```

在仓库内构建：

```sh
cargo build --release --locked
./target/release/uup --help
```

Windows PowerShell 对应可执行文件为 `./target/release/uup.exe`。
也可以下载 [GitHub Releases](https://github.com/CKylinMC/ununitypackage/releases) 中的平台压缩包，解压后将可执行文件所在目录加入 `PATH`。
Release 压缩包根目录直接包含 `uup` 或 `uup.exe`，附带 README、SKILL.md、LICENSE 和使用/发布文档；同名 `.sha256` 可用于校验下载内容。
现有原生 CI 产物为 Linux x64、macOS ARM64 和 Windows x64；Intel macOS 可从源码构建，本次没有对应原生 runner 验证。
macOS/Linux 手动下载后，进入可执行文件所在目录，如文件没有执行权限，可运行 `chmod +x ./uup`。

先查看和校验，再解包到独立目录：

```sh
uup info demo.unitypackage
uup list demo.unitypackage
uup verify demo.unitypackage
uup extract demo.unitypackage -o extracted
```

后续示例中的 `demo.unitypackage`、资源路径和本地文件需要换成实际值。
带空格的路径要加引号；glob 和正则也要加引号，避免 shell 提前展开。
包内路径统一使用 `/`；本地路径按操作系统习惯填写即可。

## 2. 命令速查

| 命令 | 用途 |
| --- | --- |
| `info PACKAGE [INSIDE_PATH]` | 文件类别/扩展名数量和大小、归档概况及诊断 |
| `list PACKAGE [INSIDE_PATH]` / `ls` | 列出资源、图标、manifest 和未知内容 |
| `find PACKAGE PATTERN [INSIDE_PATH]` | 按路径子串、glob 或正则查找 |
| `cat PACKAGE SELECTOR` | 输出指定内容的原始字节 |
| `show PACKAGE SELECTOR` | 条目详情和有长度限制的文本/十六进制预览 |
| `verify PACKAGE` | 校验压缩、归档、资源结构和冲突 |
| `extract PACKAGE [INSIDE_PATH] -o DIR` | 完整、目录范围或选择性解包 |
| `pack DIR OUTPUT` | 从目录创建包；`build` 为兼容别名 |
| `repack PACKAGE -o OUTPUT` | 保留内容并重新压缩 |
| `add PACKAGE FILE ... -o OUTPUT` | 新增资源或原始条目 |
| `replace PACKAGE FILE SELECTOR -o OUTPUT` | 替换内容 |
| `remove PACKAGE SELECTOR -o OUTPUT` | 删除资源或原始条目 |
| `metadata PACKAGE [ACTION]` | 自动汇总、列出、导出、读取、替换、字段编辑和删除元数据/设置 |
| `from-upm DIR OUTPUT` | 将 `package.json` 所在目录转换为 unitypackage |

每个命令都支持 `--help`，例如 `uup replace --help`。
`--json` 为全局选项，可以放在命令前或命令后。

## 3. 如何选中包内内容

包内的 Unity 资源路径与 tar 物理路径是两套路径：

| 选择器 | 指向 | 示例 |
| --- | --- | --- |
| `--path` | `pathname` 定义的资源位置 | `Assets/Demo/readme.txt` |
| `--guid` | 资源记录的 GUID | 从 `list --json` 复制 `guid` |
| `--entry` | tar 内原始条目 | `.icon.png`、`packagemanagermanifest/asset`、`GUID/asset.meta` |

```sh
uup list demo.unitypackage --json
uup list demo.unitypackage --raw
uup show demo.unitypackage --path Assets/Demo/readme.txt
uup show demo.unitypackage --guid 0123456789abcdef0123456789abcdef
uup show demo.unitypackage --entry .icon.png --hex --limit 64
```

示例 GUID 只是占位值，使用时换成包中实际的 GUID。
`cat`、`show`、`replace` 和 `remove` 一次只能使用一种选择器。
选中资源后，`cat`、`show`、`replace` 可用 `--part asset|meta|preview` 指定组成部分，默认是 `asset`。
`--entry` 已经定位到具体原始条目，直接访问该条目，无需 `--part`。
空文件夹没有 asset；查看其 meta 时使用 `--part meta`。

`--path` 支持 `Assets`、`Packages`、`ProjectSettings`、`PackageSettings` 和其他合法相对路径。
如果内容没有资源 `pathname`，使用 `--entry` 操作。

## 4. 查看与查找：不解包落盘

```sh
uup info demo.unitypackage --json
uup list demo.unitypackage
uup list demo.unitypackage --raw --json
uup ls demo.unitypackage Assets/Demo
uup list demo.unitypackage --path Packages/com.example.tool --json
uup info demo.unitypackage Assets/Demo --json

uup find demo.unitypackage readme
uup find demo.unitypackage '**/*.cs' --glob
uup find demo.unitypackage 'Runtime/.*\.cs$' --regex
uup find demo.unitypackage 'extras/**' --glob --raw
uup find demo.unitypackage '*.anim' Assets/Demo --glob
uup find demo.unitypackage readme --path Assets/Demo

uup cat demo.unitypackage --path Assets/Demo/readme.txt
uup cat demo.unitypackage --path Assets/Demo/readme.txt --part meta
uup show demo.unitypackage --path Assets/Demo/readme.txt --limit 2048
uup show demo.unitypackage --path Assets/Demo/image.png --part preview --hex --limit 64
```

`find` 默认对完整路径做区分大小写的子串匹配，文件名也是路径的一部分。
`--glob` 和 `--regex` 互斥；默认搜索逻辑资源和其他内容，`--raw` 改为搜索原始 tar 路径。
`list` 文本输出为类型、大小和路径；需要 GUID、meta 状态等字段时使用 `--json`。

`ls` 是 `list` 的同义别名。`info/list/find` 可以用位置参数或 `--path` 限定范围，
两者不能同时使用；省略或 `.` 表示整个包。文件精确匹配，目录包含全部后代，
即使包内没有对应的文件夹记录也能查询。`Assets/A` 不会匹配 `Assets/AB`。
`find` 的搜索 PATTERN 仍为必填参数；范围内再执行原有子串/glob/正则搜索。
`--raw` 下范围指向 tar 物理路径。明确指定的范围或搜索结果不存在时退出码为 3。

### info：按 Unity 文件类型统计

```sh
uup info demo.unitypackage
uup info demo.unitypackage --path Packages/com.example.tool --json
```

JSON 保留原有 `entries/resources/payload_bytes/diagnostics` 全包字段，新增 `scope`
和 `statistics`。`statistics.files` 给出范围内资源文件的 `count/bytes`，
`folders` 是资源目录数量；`categories` 和 `extensions` 的每个值也包含 `count/bytes`。
例如动画数量为 `statistics.extensions[".anim"].count`。

| categories 键 | 代表扩展名 |
| --- | --- |
| `animations` / `controllers` | `.anim` / `.controller`、`.overrideController` |
| `models` | `.fbx`、`.obj`、`.blend`、`.dae`、`.gltf`、`.glb` 等 |
| `audio` / `video` | `.wav`、`.mp3`、`.ogg`、`.flac` 等 / `.mp4`、`.mov`、`.webm` 等 |
| `images` / `textures` | `.png`、`.jpg`、`.psd`、`.exr` 等 / `.renderTexture`、`.cubemap`、`.dds`、`.ktx` 等 |
| `scripts` | `.cs`、`.js`、`.boo`、`.lua`、`.py` |
| `assembly_definitions` / `plugin_binaries` | `.asmdef`、`.asmref` / `.dll`、`.so`、`.dylib`、`.bundle`、`.a` 等 |
| `scenes` / `prefabs` / `materials` | `.unity` / `.prefab` / `.mat` |
| `shaders` | `.shader`、`.compute`、`.shadergraph`、`.shadersubgraph`、`.hlsl`、`.cginc` 等 |
| `ui` / `fonts` | `.uxml`、`.uss`、`.guiskin` / `.ttf`、`.otf`、`.fontsettings` |
| `physics` / `terrain` | `.physicMaterial`、`.physicsMaterial2D` / `.terrainlayer` |
| `text` | `.txt`、`.md`、`.json`、`.yaml`、`.xml`、`.csv`、`.ini`、`.toml` 等 |
| `assets` / `other` | `.asset` / 未分类或无扩展名 |

扩展名统一转小写，无扩展名的键为 `<none>`。每个有常规 asset payload 的资源文件
只计入一个类别；类别和扩展名各自的合计都等于 `files`。零数量类别也会列出。
非 Assets 资源一样计数；空文件计为一个文件，目录不计入文件数。

`auxiliary` 单独汇总 `pathname/meta/preview/special/unknown/tar_directories` 等归档内容。
没有 pathname 的原始 `.anim` 不加入资源动画数量，可用 `list --raw` 检查原始文件。
图片后缀计入 images，显式纹理格式计入 textures；不读取 importer 来推断图片的 Unity 类型。
`.anim` 数量表示文件数，不包含 FBX 内嵌动画；`.asset` 不细分 YAML 对象，
`.dll` 仅列为插件二进制，不仅凭后缀判定托管/原生属性。

`info/list/find/cat/show/verify` 不创建临时文件，也不解包到磁盘。
压缩包需要顺序扫描，访问指定内容可能多次扫描输入。
`show` 默认最多预览 4096 字节，`--limit` 最大为 16 MiB；`cat` 输出完整原始字节。

在 macOS/Linux，可以将内容保存为文件：

```sh
uup cat demo.unitypackage --entry .icon.png > icon.png
```

这里的落盘由 shell 的 `>` 完成。Windows 保存二进制时可直接用 `extract --entry`；
如使用 PowerShell 的 `>` 保存原始二进制，需要 PowerShell 7.4 或更新版本。
`cat` 和 `metadata get` 即使带 `--json`，stdout 仍是原始内容。

`info/list` 可以展示有资源诊断的包；命令成功不表示所有资源都有效。
严格验收请运行 `verify`，并检查其退出码。

## 5. 完整和选择性解包

```sh
# 完整解包：按资源路径还原文件、meta 和空目录
uup extract demo.unitypackage -o extracted

# 指定资源，默认带出对应 meta
uup extract demo.unitypackage -o selected --path Assets/Demo/readme.txt
uup extract demo.unitypackage -o scripts --glob '**/*.cs'

# 同时选取多个资源；选择器可重复，结果取并集
uup extract demo.unitypackage -o selected --path Assets/Demo/readme.txt --path Assets/Demo/image.png

# 选中目录自身与全部后代资源
uup extract demo.unitypackage -o demo-only --path Assets/Demo --glob 'Assets/Demo/**'

# 2.1.0：直接指定目录范围，包含后代，支持没有目录记录的路径
uup extract demo.unitypackage Assets/Demo -o scoped
uup extract demo.unitypackage Assets/Demo -o scoped-scripts --glob '**/*.cs'

# 不提取资源 meta
uup extract demo.unitypackage -o content-only --glob '**/*.cs' --no-meta

# 原始条目操作，可保存图标、未知内容等
uup extract demo.unitypackage -o icon-only --entry .icon.png
uup extract demo.unitypackage -o extras-only --entry extras/data.bin

# 整包按 tar 路径解出：GUID/asset、GUID/pathname 等
uup extract demo.unitypackage -o physical --raw
```

原有 `--path` 仍为精确匹配；目录位置参数会递归限定范围。
位置参数与已有选择器组合时，先取选择器并集，再用范围限定。
选择性资源解包默认包含 asset 和 meta，preview/未知兄弟条目可通过 `--entry` 单独选取。
完整解包同时保留特殊、preview 和未知条目，放在它们原来的安全 tar 路径下。

输出目录可以不存在，工具会创建所需目录；已存在目录可以使用，但已有目标文件不会被覆盖。
`extract` 没有 `--force`，冲突时应换一个输出目录。
路径越界、符号链接、文件/目录冲突和当前平台不支持的文件名会报错。
tar 链接可保留在重打包中，解包时不创建链接。

资源语义损坏时，可尝试 `--raw` 或只使用 `--entry` 恢复内容；仍会检查归档及输出路径安全。
解包不是整目录事务，文件系统写入失败可能留下部分输出；不要同时用其他程序修改目标目录。

## 6. 从目录打包与重打包

```sh
# source 下的相对路径默认映射到 Assets/
uup pack ./source demo.unitypackage
uup build ./source demo.unitypackage

# 保留指定的导入根目录，并允许生成缺少的 meta
uup pack ./source generated.unitypackage --prefix Assets/Demo --generate-meta

# 按源目录相对路径排除内容
uup pack ./source filtered.unitypackage --prefix Assets/Demo --generate-meta --exclude 'Tests/**'

# 直接保留包内容重新压缩
uup repack demo.unitypackage -o repacked.unitypackage
```

例如 `source/Runtime/Tool.cs`：默认输出路径为 `Assets/Runtime/Tool.cs`；
指定 `--prefix Assets/Demo` 后为 `Assets/Demo/Runtime/Tool.cs`。
打包选中的目录名不会自动加入目标路径。
输出文件使用给定名称，不会自动补 `.unitypackage` 扩展名；建议显式填写完整扩展名。

工具读取并保留现有 `.meta` 和 GUID。应有 meta 的文件/目录缺失时会报错，
使用 `--generate-meta` 才会生成最小 metadata。生成 GUID 由目标路径决定，
相同目标路径可重复生成；改动目标映射会改变新生成的 GUID。
已有 importer 配置保持原始内容，源目录也不会被写入生成的 meta。
`ProjectSettings`、`PackageSettings`、`UserSettings`、点开头和 `~` 结尾路径允许没有 meta。
孤立 meta、重复 GUID 和源目录中的符号链接会报错。

`pack` 和 `from-upm` 默认排除 `.git/.svn/.hg`、`node_modules`、`target`、
`Library/Temp/Logs`、`obj/bin` 和 `.DS_Store`，可重复使用 `--exclude`。
meta 跟随其资源一起排除，输出包必须放在源目录之外。

`repack` 适合保留原始包结构；解包再 `pack` 会重新组织目录，不能代替原包重打包。

## 7. 新增、替换和删除

所有包修改都要求另一个输出文件；以下示例是独立操作，默认读取同一个原包。
要连续修改，下一步的输入应使用上一步的输出。

```sh
# 替换 asset，保留原 GUID、meta 和 preview
uup replace demo.unitypackage ./new.txt --path Assets/Demo/readme.txt -o replaced.unitypackage

# 替换 meta，替换文件必须保持原 GUID 和 folderAsset
uup replace demo.unitypackage ./readme.meta --path Assets/Demo/readme.txt --part meta -o meta-edited.unitypackage

# 替换已经存在的 preview
uup replace demo.unitypackage ./preview.png --path Assets/Demo/image.png --part preview -o preview-edited.unitypackage

# 新增资源：指定 meta，也可自动使用本地同名 .meta
uup add demo.unitypackage ./New.cs --path Assets/Demo/New.cs --meta ./New.cs.meta -o added.unitypackage
uup add demo.unitypackage ./Other.cs --path Assets/Demo/Other.cs --generate-meta -o generated-add.unitypackage

# 新增非 Assets 资源，PackageSettings 可没有 meta
uup add demo.unitypackage ./settings.json --path PackageSettings/settings.json -o settings-added.unitypackage

# 新增/替换未知原始内容
uup add demo.unitypackage ./extra.bin --entry extras/data.bin -o raw-added.unitypackage
uup replace raw-added.unitypackage ./new-extra.bin --entry extras/data.bin -o raw-replaced.unitypackage

# 删除资源：同时删除该资源的 meta、preview 和其他兄弟条目
uup remove demo.unitypackage --path Assets/Demo/readme.txt -o removed.unitypackage

# 删除非空资源目录，显式包含后代
uup remove demo.unitypackage --path Assets/Demo --recursive -o directory-removed.unitypackage

# 精确删除一个原始条目
uup remove raw-added.unitypackage --entry extras/data.bin -o raw-removed.unitypackage
```

新增已存在的位置会失败，需改用 `replace`。
替换 asset 不自动更新 preview；要同步预览，应另行替换已有 preview 或按原始路径新增。
非 `Assets` 内容同样可用 `--path` 增删替换；未定义资源记录的内容用 `--entry`。
删除原始目录及其后代也需要 `--recursive`。
`add` 传入目录时只新增该目录条目，不递归导入目录树；整目录内容用 `pack` 或 `from-upm`。

已有输出包需要显式 `--force`，例如：

```sh
uup repack demo.unitypackage -o repacked.unitypackage --force
```

`--force` 允许替换输出文件，仍不能把输入包本身作为输出。
工具先写同目录临时文件、校验成功后完成输出；失败不会提前截断输入或已有输出。
未修改的 GUID、payload、pathname、meta、preview、未知内容和支持的 tar 属性保持不变，
压缩后的整个包不保证逐字节一致。

## 8. metadata、package.json 和设置文件

```sh
# 不带子命令时自动汇总；JSON 模式包含 counts 和 entries
uup metadata demo.unitypackage
uup metadata demo.unitypackage summary --json
uup metadata demo.unitypackage summary --path Packages/com.example.tool --json

# 自动发现，可按类型或路径限定
uup metadata demo.unitypackage list --json
uup metadata demo.unitypackage list package-json --json
uup metadata demo.unitypackage list settings --path PackageSettings --json

# 原样导出发现的元数据，不导出所有 asset.meta 或资源 preview
uup metadata demo.unitypackage dump -o metadata
uup metadata demo.unitypackage dump --kind settings --path PackageSettings -o settings-dump

# 查看包清单和设置
uup metadata demo.unitypackage get package-json --path Packages/com.example.tool/package.json
uup metadata demo.unitypackage get settings --path PackageSettings/settings.json
uup metadata demo.unitypackage get project-manifest

# 按 JSON Pointer 编辑字段，保留其他字段
uup metadata demo.unitypackage edit package-json --path Packages/com.example.tool/package.json --set '/description="Updated description"' -o description-edited.unitypackage
uup metadata demo.unitypackage edit settings --path PackageSettings/settings.json --set '/enabled=false' -o settings-edited.unitypackage
uup metadata demo.unitypackage edit manifest --set '/dependencies/com.example.tool="1.2.0"' -o dependency-edited.unitypackage
uup metadata demo.unitypackage edit package-json --path Packages/com.example.tool/package.json --delete '/description' -o description-removed.unitypackage

# YAML/二进制设置使用完整文件替换
uup metadata demo.unitypackage set settings --path ProjectSettings/ProjectSettings.asset --file ./ProjectSettings.asset -o project-settings-edited.unitypackage
```

支持六种 KIND：`manifest/icon/cover/package-json/project-manifest/settings`。
设置文件按非 Assets 路径中名为 Settings 或以 Settings 结尾的目录组件发现，
例如 ProjectSettings、PackageSettings、UserSettings、Packages/.../PluginSettings。
这是路径约定，不保证识别所有第三方设置格式；其他内容仍可用原始条目命令处理。

清单的 `path` 是资源目标路径或原始路径，`entry` 是 tar 路径，另有 `guid/size/format/kind`。
`get/set/edit/remove` 可用一种 `--path/--guid/--entry` 明确选择。
未指定选择器时必须恰有一个候选；多个候选会报错并列出路径，不自动选择第一个。
`summary/list/dump --path` 是目录范围；`get/set/edit/remove --path` 是精确选择。

自动汇总只解析受限 JSON 信息和 PNG 头部尺寸，PNG summary 不代表完整图片已通过校验。
损坏或大于 16 MiB 的 JSON 保留在清单中并带 `warning`，可继续 `get/dump` 原样取出。
`summary/list/get` 不落盘；`dump` 明确写入输出目录，保留字节及路径关系，提前检查冲突。
已有目标文件不会覆盖，冲突时换一个目录；文件系统错误仍可能留下部分输出。

三种常被称为 manifest 的文件各自独立：

| 内容 | 包内位置 | 操作方式 |
| --- | --- | --- |
| unitypackage 依赖清单 | `packagemanagermanifest/asset` | `metadata ... manifest` |
| UPM 包定义 | `Packages/<name>/package.json` 等路径 | `metadata ... package-json` 或资源命令 |
| 项目依赖清单 | `Packages/manifest.json` | `metadata ... project-manifest` 或资源命令 |

表中后两项按实际包内位置操作；没有 `pathname` 的原始文件使用 `--entry`。

```sh
uup metadata demo.unitypackage get manifest
uup metadata demo.unitypackage set manifest --file dependencies.json -o manifest-edited.unitypackage
uup metadata demo.unitypackage remove manifest -o no-manifest.unitypackage

uup metadata demo.unitypackage set icon --file icon.png -o with-icon.unitypackage
uup metadata demo.unitypackage set cover --file cover.png -o with-cover.unitypackage
uup metadata demo.unitypackage remove icon -o no-icon.unitypackage

uup cat demo.unitypackage --path Packages/com.example.tool/package.json
uup replace demo.unitypackage ./package.json --path Packages/com.example.tool/package.json -o upm-manifest-edited.unitypackage
```

依赖 manifest 示例：

```json
{
  "dependencies": {
    "com.unity.textmeshpro": "3.0.6"
  }
}
```

manifest 必须是 JSON 对象，`dependencies` 若存在，必须是字符串值组成的对象。
`set` 写入完整文件，最多 64 MiB，不做字段合并。manifest/icon/cover 缺失时可新增；
package-json/project-manifest/settings 操作已有候选，新增文件使用 `add`。
`edit` 支持 JSON 字段编辑，文件最多 16 MiB，输出为缩进 JSON 加末尾换行。
可重复 `--set '/pointer=JSON_VALUE'` 和 `--delete '/pointer'`；先依次设置，再依次删除。
字符串值须带 JSON 双引号，shell 外层使用单引号。指针中的 `/` 写为 `~1`、`~` 写为 `~0`。
可创建缺失的对象父级；数组使用现有下标或末尾 `-` 追加，下标不允许前导零。
空指针可通过 `--set '={...}'` 替换整个对象，不能删除根；指针最多 128 个组件。
JSON 文档须保持对象，依赖清单的 dependencies 仍须为字符串值对象。
`.json` 设置支持字段编辑；YAML、二进制及其他 opaque 设置用 `set --file`。
set 保留用户提交文件的字节和格式；edit 保留未涉及字段，按上述格式重新序列化。
未修改的其他 JSON 文件保持原始字节。

`remove` 删除资源时同时删除该记录的 meta、preview 和未知兄弟；原始文件只删除该条目。
所有包修改输出新文件，并保留未修改记录和内容。

`icon` 操作根目录 `.icon.png`；`cover` 操作旧版的 `.cover.png`；
资源自己的 `preview.png` 是第三种独立内容。
设置 icon/cover 会实际解码并验证 PNG，不能只改文件扩展名。

## 9. UPM 目录转 unitypackage

源目录必须包含合法 `package.json`，其中需要 `name` 和 SemVer `version`。

```sh
# 默认导入到 Packages/<package.json 的 name>/
uup from-upm ./com.example.tool tool.unitypackage

# 显式补足缺失的受 Unity 跟踪的 meta，并保存转换报告
uup from-upm ./com.example.tool generated-tool.unitypackage --generate-meta --json > conversion-report.json

# 映射到 Assets/<name>/
uup from-upm ./com.example.tool assets-tool.unitypackage --layout assets --generate-meta

# 指定 Assets 中的位置，排除测试内容
uup from-upm ./com.example.tool filtered-tool.unitypackage --layout assets --prefix Assets/Tools/MyTool --generate-meta --exclude 'Tests/**'
```

完整收集 `Runtime`、`Editor`、`Tests`、`Samples~`、`Documentation~` 和包定义。
`~` 结尾、点开头目录的内容会保留，默认缓存/VCS 排除项仍然生效。
`Samples~`、`Documentation~` 无须生成 meta，已有 GUID/meta 不变。

依赖保留在 `package.json`，工具不下载或内嵌依赖，也不修改用户项目的 manifest。
转换报告列出目标路径、资源数、依赖、兼容警告和 `unity_import: not_verified`。
自定义 registry、Git/本地依赖、程序集和依赖固定路径的代码需要在目标 Unity 项目中检查。
Assets 映射不会自动重写代码或修复引用，也不等同于通过 UPM 安装。

## 10. 脚本、退出码与排错

结构化输出适用于脚本：

```sh
uup list demo.unitypackage --json > contents.json
uup verify demo.unitypackage --json
```

日志和错误走 stderr，结果走 stdout。读取原始内容的命令保持原始字节输出。

| 退出码 | 含义 |
| --- | --- |
| `0` | 操作成功 |
| `1` | 读写、格式或校验失败 |
| `2` | CLI 参数/用法错误 |
| `3` | 未找到选择器或搜索匹配项 |

| 常见提示 | 处理方式 |
| --- | --- |
| `destination exists` | 解包换目录；写包确认后用 `--force` 替换输出 |
| `missing .meta` | 提供原有 meta，或显式 `--generate-meta` |
| `resource not found` / `entry not found` | 对照 `list` / `list --raw` 检查路径和选择器 |
| `... missing` | 资源没有所选 asset/meta/preview，改选实际存在的部分 |
| `directory contains resources` | 确认删除范围后增加 `--recursive` |
| `replacement meta must preserve GUID and folderAsset` | 使用属于原资源、类型一致的 meta |
| 路径或 GUID 冲突 | 修正源数据；工具不会静默改名 |

默认上限：1,000,000 个物理 tar 记录、单条目 8 GiB、解压流 64 GiB。
必要时用全局 `--max-entries`、`--max-entry-bytes`、`--max-expanded-bytes` 调整，字节参数为整数：

```sh
uup --max-expanded-bytes 137438953472 info large.unitypackage
```

支持 USTAR、GNU 长路径和 local PAX；global PAX、sparse tar 等不支持的归档构造会明确失败。
三平台归档/CLI 测试已通过。Unity 2022.3 LTS 和 Unity 6 的真实 Editor 导入尚未验证，
包括 UI 图标、依赖解析、importer、资源引用和旧版提到的重复资源问题。
实际导入验收见 [TESTING.md](TESTING.md)，不能以 CLI 往返结果代替 Editor 验证。
本项目只在推送 `v*` tag 时自动构建并发布，发版规则见 [RELEASING.md](RELEASING.md)。

## 11. 与旧版的使用区别

旧命令以旧 README 中的 `UnUnityPackage` 为准。旧源码和文档保存在 [legacy/dotnet](../legacy/dotnet/README.md)。

| 项目 | 旧版 | 新版 |
| --- | --- | --- |
| 程序名与运行环境 | `UnUnityPackage`，.NET 8，仓库构建脚本面向 Windows x64 | `uup`，原生 Rust 可执行文件，三平台 CI |
| 项目名与命令名 | 旧文档使用 `UnUnityPackage` 命令 | 项目/Cargo 包名是 `uup-cli`，命令和可执行文件是 `uup` / `uup.exe` |
| 完整解包 | `UnUnityPackage extract demo.unitypackage -o out` | `uup extract demo.unitypackage -o out`，参数基本兼容 |
| 输出目录 | 需要提前创建目录 | 自动创建所需目录 |
| 已有目标文件 | 解包时允许覆盖 | 拒绝覆盖；解包需换目录，写包覆盖需 `--force` |
| 打包命令 | `UnUnityPackage build DIR OUTPUT` | 推荐 `uup pack DIR OUTPUT`，`build` 仍是别名 |
| 导入根目录 | 固定 `Assets/` | 默认 `Assets/`，可用 `--prefix` 指定 |
| 缺少 meta | 打包主要通过 `.meta` 收集资源，未关联内容可能遗漏 | 收集目录内容，缺少应有 meta 时明确报错，可显式生成 |
| 输出扩展名 | 尝试将输出改为 `.unitypackage` | 保留指定的文件名，请显式填写扩展名 |
| 封面参数 | `build ... -c cover.png` / `--cover` | 改为 `metadata ... set cover --file cover.png -o OUTPUT` |
| Unity 图标 | `.icon.png` 在解包中被跳过 | 保留，可独立 `metadata ... icon` 增删替换 |
| 查看/查找/内容预览 | `list` 未接入，主要操作为 extract/build | info/list/find/cat/show/verify，不解包落盘 |
| 目录范围与别名 | 无对应接口 | `ls/list PACKAGE PATH`、`find PACKAGE PATTERN PATH`、`extract PACKAGE PATH` |
| 文件类型统计 | 无对应功能 | `info PACKAGE [PATH]`，按类别和扩展名统计数量及字节 |
| 元数据与设置查询 | 无专用接口 | metadata 自动汇总/导出；package.json、项目 manifest、非 Assets 设置支持查询编辑 |
| 选择性解包 | 无对应参数 | 目录位置参数及 `--path` / `--guid` / `--glob` / `--entry` |
| 非 Assets 与未知内容 | 普通非 Assets 资源可解出，但 manifest/未知条目等存在遗漏 | 区分资源、特殊条目、原始条目并保留，支持编辑 |
| 空目录 | 可能只留下 meta | 还原目录与 meta |
| 修改与重打包 | 没有专用命令 | repack/add/replace/remove/metadata，默认另存新包 |
| UPM 转换 | 没有专用命令 | from-upm，两种映射和转换报告 |
| 脚本结果 | 主要输出成功/失败文本，错误退出码不完整 | JSON、stderr 分离和明确的非零退出码 |

旧版带封面打包的迁移示例：

```sh
# 旧版
UnUnityPackage build ./source demo.unitypackage -c cover.png

# 新版：保留旧封面含义，分两步完成
uup pack ./source base.unitypackage
uup metadata base.unitypackage set cover --file cover.png -o demo.unitypackage
```

如果目标是 Unity 导入界面的图标，第二步选择 `icon`：

```sh
uup metadata base.unitypackage set icon --file icon.png -o demo.unitypackage
```

`build` 别名只保留子命令名称，旧版的 `-c/--cover` 参数不再接受。
旧脚本至少需要替换可执行文件名，并按上述规则迁移封面、覆盖和退出码处理。

# uup-cli (ununitypackage-cli)

无需 Unity Editor 的 Rust 命令行工具，用于查看、解包、修改和创建 `.unitypackage`。
支持 macOS、Linux 和 Windows；使用纯 Rust gzip/tar，不依赖系统 tar 或 .NET。

## 安装 / 构建

需要 Rust 1.88 或更新版本。直接安装当前 `next` 分支：

```sh
cargo install --git https://github.com/CKylinMC/ununitypackage --branch next --locked
```

或在仓库内构建：

```sh
cargo build --release --locked
./target/release/uup-cli --help
```

Windows 可运行 `target\release\uup-cli.exe`。GitHub Actions 的 Rust CI 在三个
原生平台测试并生成可下载的构建 artifacts（包含可执行文件、README 和 LICENSE）。

[已通过的三平台 CI 与构建产物](https://github.com/CKylinMC/ununitypackage/actions/runs/36724536480)：
Linux x64、macOS ARM64、Windows x64。Intel macOS 可从源码构建，本次没有对应原生 runner 验证。

## 查看与查找（不落盘）

```sh
uup-cli info demo.unitypackage --json
uup-cli list demo.unitypackage
uup-cli list demo.unitypackage --raw
uup-cli find demo.unitypackage '*.cs' --glob
uup-cli find demo.unitypackage 'Runtime/.*\.cs$' --regex
uup-cli show demo.unitypackage --path Assets/Demo/readme.txt
uup-cli show demo.unitypackage --entry .icon.png --hex --limit 64
uup-cli cat demo.unitypackage --path Assets/Demo/readme.txt
uup-cli cat demo.unitypackage --path Assets/Demo/readme.txt --part meta
uup-cli cat demo.unitypackage --entry .icon.png > icon.png
uup-cli verify demo.unitypackage --json
```

这些命令不创建临时文件。索引只保存条目信息、小型 pathname 和 meta；payload
始终流式读取。压缩包需要顺序扫描，查看指定资源可能扫描多次。
`cat` 输出原始字节；`show` 的预览有长度限制，二进制内容可使用 `--hex`。

`--path` 指 Unity 资源路径，`--guid` 指资源 ID，`--entry` 指 tar 内原始条目路径。
单条目操作中三者互斥。`--part asset|meta|preview` 默认选择 asset。
不以文件后缀识别资源，也不只识别 `Assets`：`Packages`、`ProjectSettings`、
`PackageSettings`、图标、manifest、未知条目都可以访问。

## 解包

```sh
uup-cli extract demo.unitypackage -o extracted
uup-cli extract demo.unitypackage -o selected --path Assets/Demo/readme.txt
uup-cli extract demo.unitypackage -o scripts --glob '**/*.cs'
uup-cli extract demo.unitypackage -o icon-only --entry .icon.png
uup-cli extract demo.unitypackage -o physical --raw
```

选择资源时默认同时提取 `.meta`，可用 `--no-meta` 关闭。
完整解包还原资源路径和空文件夹，并保留特殊/未知条目（包括 preview）到其原始
安全 tar 路径。`--raw` 则全部使用 tar 路径，可用于恢复存在资源语义问题的包。
已有文件、路径越界、符号链接、文件/目录冲突和平台文件名冲突会报错。
不要在其他程序同时修改的目录中解包；文件系统写入错误可能留下部分输出。

## 打包和修改

```sh
uup-cli pack ./Assets demo.unitypackage
uup-cli build ./Assets demo.unitypackage
uup-cli pack ./source demo.unitypackage --prefix Assets/MyTool --generate-meta
uup-cli repack demo.unitypackage -o repacked.unitypackage

uup-cli replace demo.unitypackage ./new.txt --path Assets/Demo/readme.txt -o edited.unitypackage
uup-cli replace demo.unitypackage ./readme.meta --path Assets/Demo/readme.txt --part meta -o edited.unitypackage
uup-cli add demo.unitypackage ./new.cs --path Assets/Demo/New.cs --generate-meta -o added.unitypackage
uup-cli add demo.unitypackage ./settings.json --path PackageSettings/settings.json -o added.unitypackage
uup-cli add demo.unitypackage ./extra.bin --entry extras/data.bin -o added.unitypackage
uup-cli remove demo.unitypackage --path Assets/Demo/readme.txt -o removed.unitypackage
uup-cli remove demo.unitypackage --path Assets/Demo --recursive -o removed.unitypackage
uup-cli remove demo.unitypackage --entry extras/data.bin -o removed.unitypackage
```

输出包必须与输入不同。已有输出需要 `--force`；工具先写入同目录临时文件，
验证后再完成输出，不提前截断已有文件。未改动的 payload、GUID、pathname、meta、
preview、未知内容及支持的 tar 属性保持不变；gzip 的字节和容器头不保证相同。

替换 asset 默认保留 GUID、meta 和 preview（preview 可能需要自行更新）。
替换 meta 必须保持 GUID 与 folderAsset。删除资源同时删除其所有原始兄弟条目；
递归删除还包括子资源。`--entry` 用于精确修改原始条目，也可修复某些损坏的记录。

打包自动读取已有 `.meta`，新增时可用 `--meta FILE` 指定。缺少应有的 meta 时
报错，`--generate-meta` 显式生成最小 metadata。生成 GUID 由命名空间和目标路径
确定，内容变动不会改变它；不修改源目录，也不会覆盖已有 importer 配置。
源目录中的符号链接、孤立 meta、重复 GUID 会报错。生成的最小 meta 不代表
Unity 对每种 importer 自动生成的完整配置。

## manifest 和图标

```sh
uup-cli metadata demo.unitypackage get manifest
uup-cli metadata demo.unitypackage set manifest --file dependencies.json -o edited.unitypackage
uup-cli metadata demo.unitypackage set icon --file icon.png -o edited.unitypackage
uup-cli metadata demo.unitypackage set cover --file cover.png -o edited.unitypackage
uup-cli metadata demo.unitypackage remove icon -o edited.unitypackage
```

`manifest` 操作 `packagemanagermanifest/asset`，接受带字符串值 dependencies 的 JSON
对象，也保留用户文件中的自定义字段和格式。`icon` 为 `.icon.png`，设置时实际解码
验证 PNG。`cover` 为旧工具的 `.cover.png`，不将它当作 Unity 的 import UI icon。
metadata get 和 cat 一样输出原始内容。

UPM 的 `package.json` 和项目的 `Packages/manifest.json` 是独立文件，使用
`--path` 或 `--entry` 对其操作；工具不会将三种 manifest 混为一谈。

## UPM → unitypackage

```sh
uup-cli from-upm ./com.example.tool tool.unitypackage
uup-cli from-upm ./com.example.tool tool.unitypackage --generate-meta --json
uup-cli from-upm ./com.example.tool assets.unitypackage --layout assets --generate-meta
uup-cli from-upm ./com.example.tool tool.unitypackage --exclude 'Tests/**' --generate-meta
```

源目录必须含合法的 `package.json`（name、SemVer version）。默认导入位置为
`Packages/<name>/`，保留包目录和现有 GUID/metas；`--layout assets` 显式映射为
`Assets/<name>/`。`Samples~`、`Documentation~` 等隐藏目录全部收集，无须生成 meta。
`--generate-meta` 可补足受 Unity 跟踪的文件和目录所缺少的 metadata。

默认排除 `.git/.svn/.hg`、`node_modules`、`target`、`Library/Temp/Logs`、`obj/bin`
和 `.DS_Store`，可重复传入 `--exclude`。同一资源的 meta 跟随资源一起排除。
输出必须位于源目录之外，防止把上一次的输出再次打包。

依赖保留在 package.json，不下载、不内嵌，也不修改用户项目。JSON 报告列出目标、
依赖和警告。自定义 registry、Git/本地依赖、程序集和路径敏感代码仍需要验证；
Assets 映射不会自动修复源代码路径或程序集引用。

## 格式边界与验证

支持 gzip、USTAR、GNU 长路径和 local PAX 扩展。未知 tar 内容可以查看和重打包；
链接条目保留在重打包中，解包时不创建链接。Global PAX 和 sparse tar 明确拒绝，
避免解释错误和静默丢失。读取检查完整 gzip CRC、tar 结束块和尾部数据。

默认限制：1,000,000 个物理 tar 记录，单条目 8 GiB，解压流 64 GiB，pathname/
GNU 路径 64 KiB、meta 4 MiB、local PAX 1 MiB。前三项可通过
`--max-entries`、`--max-entry-bytes`、`--max-expanded-bytes` 调整。
`show --limit` 最大 16 MiB；PNG 设置时解码缓冲区最大 64 MiB。

退出码：0 成功，1 操作/校验失败，2 参数错误，3 未找到匹配项。
`--json` 提供结构化输出；日志及错误输出到 stderr，内容输出到 stdout。

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
python scripts/interoperability.py target/release/uup-cli
```

归档测试与 Unity 导入验证分开记录。当前云端没有 Unity Editor；Unity 2022.3 LTS
和 Unity 6 的 import UI、依赖解析、importer、资源引用和重复资源问题尚未进行真实
导入验证，不能用 CLI 往返测试替代。验收清单见 [TESTING.md](docs/TESTING.md)。

规划和续跑记录见 [PLAN.md](docs/PLAN.md)、[SPEC.md](docs/SPEC.md)、
[FORMAT.md](docs/FORMAT.md)、[CLOUD-RUNBOOK.md](docs/CLOUD-RUNBOOK.md) 和
[PROGRESS.md](docs/PROGRESS.md)。旧 .NET 项目保存在 `legacy/dotnet/`，供历史对照。

Unity is a trademark of Unity Technologies. This project is not affiliated with
Unity Technologies. License: WTFPL.

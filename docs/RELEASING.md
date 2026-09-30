# 版本与发布

原 .NET 版本线为 `1.0.0`；Rust 版本线从 `2.0.0` 开始。
项目/Cargo 包名为 `uup-cli`，可执行文件始终是 `uup` / `uup.exe`。

## 选择版本

用户指定版本、tag 或是否预发布时，以其要求为准。未指定时，检查上一版之后的改动：
修复问题用 patch，兼容新增功能用 minor，不兼容变化用 major。
如果变更范围或稳定/预发布意图不能从上下文确定，再询问用户。
普通开发提交不自动发版；用户已要求发版且版本明确时，完成发布操作。
供后续 Agent 遵循的规则保存在根目录 [AGENTS.md](../AGENTS.md)。

Cargo.toml、Cargo.lock 的 uup-cli 版本和 tag 的语义版本必须一致，否则 CI 在构建前失败。
推荐使用标准 SemVer tag：

| tag | Cargo 版本 | GitHub Release |
| --- | --- | --- |
| `v2.0.0` | `2.0.0` | 稳定版 |
| `v2.1.0-beta.1` | `2.1.0-beta.1` | 预发布 |
| `v2.1.0-alpha` | `2.1.0-alpha` | 预发布 |
| `v2.1.0-rc.1` | `2.1.0-rc.1` | 预发布 |
| `v2.1.0+build.1` | `2.1.0+build.1` | 稳定版（build metadata 不代表预发布） |
| `v2.1.0.beta` | `2.1.0-beta` | 预发布，兼容点号后缀 |
| `v2.1.0.alpha` | `2.1.0-alpha` | 预发布，兼容点号后缀 |
| `v2.1.0.beta-1` | `2.1.0-beta.1` | 预发布，兼容点号后缀 |

点号后缀是兼容格式，先规范化后与 Cargo 版本比对。缺少版本段、数字前导零、
无效字符及其他不合法 tag 会报错，避免把错误版本发布为稳定版。

## 发版步骤

确认版本和修改范围后，更新版本与文档，执行：

```sh
python -m unittest discover -s scripts -p 'test_release.py' -v
python scripts/release.py metadata --tag v2.0.0
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
python scripts/interoperability.py target/release/uup
```

示例版本要换成实际准备发布的版本。Windows 最后一条使用 `target/release/uup.exe`。
检查 tag 未被占用；在发布提交上创建 annotated tag，先推送分支，再推送该 tag：

```sh
git push origin next
git tag -a v2.0.0 -m 'Release 2.0.0'
git push origin v2.0.0
```

不要移动已经发布的 tag。推送后核对远端 tag 解引用后的 SHA 与发布提交一致。

## 自动发布流水线

[Release workflow](../.github/workflows/ci.yml) 只响应 `push.tags: v*`。
分支提交、PR 和其他 tag 不会自动构建或发版。

1. 校验 tag、Cargo.toml 和 Cargo.lock；判断稳定版/预发布。
2. Linux x64、macOS ARM64、Windows x64 原生执行 format、clippy、回归测试、release
   构建和独立 tar 格式检查，另行检查 Rust 1.88 最低版本。
3. 核对编译后的 `uup --version`，生成平台压缩包及对应 SHA-256 文件。
4. 全部检查通过后，核对三个平台产物及校验和，再尝试自动创建 GitHub Release。
   发布说明通过 GitHub `--generate-notes` 自动生成，包含其支持的 PR、贡献者和版本比较。
5. 预发布设置 `prerelease=true` 并禁止标为 latest；稳定版交由 GitHub 的 latest 规则处理。

产物名称示例：

```text
uup-v2.0.0-linux-x86_64.tar.gz
uup-v2.0.0-macos-aarch64.tar.gz
uup-v2.0.0-windows-x86_64.zip
```

每个压缩包都附有同名 `.sha256` 文件。解压后根目录直接包含 `uup` / `uup.exe`，
并附带 README、SKILL.md、LICENSE、docs/USAGE.md 和本发布文档。
GitHub Actions artifact 只作为构建与发布 job 之间的传输；用户下载入口是
[GitHub Releases](https://github.com/CKylinMC/ununitypackage/releases)。

## 失败与续跑

构建或版本检查失败不会创建 Release。查看具体失败 job，修正后使用新版本/tag，
不要重新指向已发布 tag。权限或网络等临时问题可在 GitHub Actions 中重跑原 tag。
发布 job 需要 `contents: write`；仓库策略拒绝时应记录失败原因。
同一 tag 的重跑会上传/替换生成的资产并校正 prerelease 标记，保留已有 Release 的说明正文。
用户手动修改的发布说明不会因重跑而重新生成。

自动发布不合并 master、不发布 crates.io，也不代表 Unity Editor 实际导入已验证。

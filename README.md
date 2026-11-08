# NEXORA

NEXORA 是一个多产品工作区。每个产品拥有独立的源码、运行时、数据目录、测试和发布流程，根目录只负责跨产品编排。

## 产品

| 产品 | 目录 | 说明 |
| --- | --- | --- |
| **NEXORA** | [`apps/nexora-launcher`](apps/nexora-launcher) | Windows 应用、文件和命令启动器 |
| **NEXORA ISLE · 星屿** | [`apps/nexora-isle`](apps/nexora-isle) | 屏幕边缘的桌面信息岛 |

两个产品互不要求另一个产品安装或运行。NEXORA ISLE 从早期版本升级时会自动迁移用户数据和邮件凭据。

## 工作区结构

```text
nexora/
├─ apps/
│  ├─ nexora-launcher/
│  └─ nexora-isle/
├─ docs/
│  └─ architecture/
├─ scripts/
│  ├─ products.ps1
│  ├─ clean-workspace.ps1
│  ├─ check-workspace-hygiene.ps1
│  └─ tests/
├─ products.psd1
├─ pnpm-workspace.yaml
└─ package.json
```

`products.psd1` 是产品清单。根级命令只读取清单并调用对应产品自己的脚本，不包含针对具体产品的构建逻辑。未来新增产品时，只需增加 `apps/<product>` 目录和一条清单配置。

## 安装依赖

```powershell
pnpm install
```

Rust 产品使用本机 Cargo 工具链；启动器同时需要 Node.js、pnpm、Rust MSVC、Visual Studio C++ Build Tools 和 Windows SDK。

## 开发

```powershell
pnpm dev:launcher
pnpm dev:isle
```

## 独立调试构建

```powershell
pnpm debug:launcher
pnpm debug:isle
pnpm debug:all
```

启动器调试产物位于 `apps/nexora-launcher/src-tauri/target/debug/nexora.exe`，Isle 调试产物位于 `apps/nexora-isle/target/debug/nexora-isle.exe`。

## 测试

```powershell
pnpm check:workspace
pnpm test:workspace
pnpm test:launcher
pnpm test:isle
pnpm test:all
```

`check:workspace` 会扫描所有文件和目录名，包括 `target`、依赖缓存和发布目录；发现旧产品构建名称时直接失败。

## 独立与联合打包

```powershell
pnpm package:launcher
pnpm package:isle
pnpm package:all
```

安装包与便携 ZIP 统一输出到根目录 `release/`。打包全部产品后只保留四个文件：

```text
release/
├─ NEXORA_LAUNCHER_<版本>_x64_setup.exe
├─ NEXORA_LAUNCHER_<版本>_portable.zip
├─ NEXORA_ISLE_<版本>_x64_setup.exe
└─ NEXORA_ISLE_<版本>_portable.zip
```

## 清理

```powershell
pnpm clean
pnpm clean:all
```

`clean:all` 同时删除依赖目录；下一次构建前需要重新运行 `pnpm install`。

跨产品约束和新增产品步骤见 [`docs/architecture/product-workspace.md`](docs/architecture/product-workspace.md)。

# Contributing

## 基本要求

- Windows 10/11 x64
- Rust stable MSVC toolchain
- Visual Studio C++ Build Tools
- Windows 10/11 SDK

## 验证命令

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets -- --test-threads=1
cargo build --bin nexora-halo
```

## 代码约定

- 产品文案统一使用 `NEXORA HALO` 或中文名 `星环`。
- 机器标识统一使用 `nexora-halo`、`nexora_halo` 或 `NexoraHalo`。
- 产品身份、数据目录和更新地址集中在 `src/product/`。
- 页面索引使用 `src/model/page.rs` 中的常量。
- 不重新引入专注、饮水、AI 额度或日历/日程模块。
- 新功能应拆分为职责明确的模块，并在 `src/lib.rs` 中声明。
- Windows 特定代码必须使用平台条件编译。

## 测试

- 纯逻辑优先单元测试。
- 系统监听、IMAP、媒体和安装器改动必须补充实机或集成说明。
- 测试不得读取真实邮箱、Cookie、API Key 或其他用户凭据。

## 许可

提交代码即表示同意按本仓库的 MIT License 分发。不要移除原始版权和第三方许可文本。

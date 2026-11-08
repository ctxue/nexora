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
cargo build --bin nexora-isle
```

## 代码约定

- 产品文案统一使用 `NEXORA ISLE` 或中文名 `岛屿`。
- 产品中文名只使用“岛屿”；`NOMI` 是岛屿表情角色，避免用其他昵称指代产品。
- 机器标识统一使用 `nexora-isle`、`nexora_isle` 或 `NexoraIsle`。
- 产品身份、数据目录和更新地址集中在 `src/product/`。
- 页面索引使用 `src/model/page.rs` 中的常量。
- 不重新引入专注、饮水、AI 额度或日历/日程模块。
- 新功能应拆分为职责明确的模块，并在 `src/lib.rs` 中声明。
- Windows 特定代码必须使用平台条件编译。
- 注释只说明设计原因、数据不变量、安全边界和 Windows 特殊行为，不记录开发过程或逐行复述代码。

## 测试

- 纯逻辑优先单元测试。
- 系统监听、IMAP、媒体和安装器改动必须补充实机或集成说明。
- 测试不得读取真实邮箱、Cookie、API Key 或其他用户凭据。

## 许可

提交代码即表示同意按本仓库的 MIT License 分发。不要移除原始版权和第三方许可文本。

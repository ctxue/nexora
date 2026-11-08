# 依赖与第三方许可

NEXORA ISLE 的修改代码采用 MIT。第三方依赖、字体和图标遵循各自许可；MIT 不授予第三方商标权。完整许可文本见 [THIRD_PARTY_NOTICES.txt](../THIRD_PARTY_NOTICES.txt)。

## 本地补丁

- `vendor/eframe`：基于 0.31.1，记录当前视口，避免 Windows WGL 在同一窗口重复切换上下文。
- `vendor/imap-proto`：基于 0.16.7，增加 QQ 空 MIME 段 `NIL` 传输编码兼容；按 RFC 2045 将其视为默认 `7BIT`。

`imap` 客户端固定为 crates.io 的 `3.0.0-alpha.15`，使用 `nom 7` 和更新后的 IMAP 协议解析器。固定精确版本用于避免 alpha 升级产生不可控的协议行为变化；3.x 稳定版发布后再恢复版本范围更新。

## 安全检查

2026 年 11 月 3 日发布前，对 Cargo.lock 的 registry 依赖查询了 OSV 公告库，并运行完整测试。已移除受 [RUSTSEC-2023-0086](https://rustsec.org/advisories/RUSTSEC-2023-0086.html) 影响的 `lexical-core 0.7.6`。

当前仍有两项上游停止维护提示：

| 依赖 | 公告 | 当前处理 |
| --- | --- | --- |
| paste 1.0.15 | [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) | 随现有 UI 依赖使用，后续跟随上游升级 |
| ttf-parser 0.25.1 | [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html) | 随字体栈使用，后续评估维护中的替代版本 |

这两项属于维护状态提示，并非本次检查确认的可利用漏洞。公告库检查不能代替完整安全审计。

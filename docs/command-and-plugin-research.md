# 自定义命令与插件功能调研

> 调研对象：把「输入 `edge` → 空格 → 文本 → 回车 → 用 Edge 打开搜索结果」这类能力做成产品功能；以及它的长期形态——插件系统。
> 基线：NEXORA v0.2.0（2026-11-01）。文中所有文件路径都指本仓库。
> 实施状态：**P0 与 P1 已落地**（自定义命令 + 关键词标签 + 指定浏览器 + 便携应用），插件（P3）仍未开始。
> 插件架构的详细设计见 [plugin-architecture-design.md](plugin-architecture-design.md)。

---

## 0. 一页结论

你举的例子**不是插件，而是「声明式命令」**：一条数据（关键词 + URL 模板 + 打开方式），由宿主解释执行，不跑第三方代码。

| 能力层 | 你举的例子 | 是否现在做 | 粗估工期（1 人） |
| --- | --- | --- | --- |
| **L1 声明式命令**：自定义搜索 / 打开 URL / 打开路径 | ✅ 就是这个 | **建议立刻做（P0）** | 10–14 人日（约 2–3 周） |
| **L1+ 更多动作 + 按名称触发 + 预设库** | 顺带覆盖 | 紧接着做（P1） | 5–8 人日 |
| **L2 脚本命令**：运行 exe/脚本，stdout 返回结果 | — | 有明确需求再做（P2） | 10–15 人日 |
| **L3 插件**：JS/WASM 运行时 + UI SDK + 市场 | — | 最后做（P3） | 4–8 周起，市场另计 |

三条关键判断：

1. **先做命令，再做插件。** L1 不需要新框架、不需要沙箱、不需要审核，2–3 周就能让用户体验到「用任意搜索引擎/任意网站搜索」；而 L3 的 80% 使用场景会被 L2 覆盖。
2. **交互上「关键词 + 空格」应自动变成标签**，比你描述的「输入命令再回车才变标签」少一步，且完全兼容后者（回车在纯关键词上同样进入标签态）。
3. 真正的技术卡点不在 UI，而在两个地方：**用指定浏览器打开**（需要浏览器枚举 + 启动参数）和**命令数据的可信边界**（自定义 URL / 将来运行脚本的权限）。

---

## 1. 现有代码盘点：能复用什么，缺什么

| 能力 | 位置 | 现状 | 对本功能意味着什么 |
| --- | --- | --- | --- |
| 搜索与分组 | `src/core/search.ts` | 纯函数：`searchApps` / `searchCommands` / `buildGroups`，分组上限 5/3/5 | 扩展成本低，但**没有「作用域 / 关键词」概念**，query 是整串参与模糊匹配 |
| 输入与键盘 | `src/App.tsx`、`src/components/SearchBar.tsx` | 输入框 + 右侧快捷键提示；`Enter` / `Esc` / `↑↓` / `Ctrl+K` / `Ctrl+,` | 需要新增 **标签（chip）UI** 和 Backspace 删除标签的键盘分支 |
| 执行模型 | `src/features/launcher/useLauncher.ts` | `LauncherItem.run()` + `keepOpen` 决定执行后是否隐藏窗口 | 自定义命令可以完全复用：生成一个 `LauncherItem` 即可 |
| 打开 URL | `src-tauri/src/commands/apps.rs` | `open_path` 走 `open::that`，对 `https://` 天然有效 | **默认浏览器打开已经能用**，只差「指定浏览器」 |
| 应用枚举 | `src-tauri/src/scanning/mod.rs`、`apps.rs` | Start Menu `.lnk` + `shell:AppsFolder\…` | **不能带参数启动**（`.lnk`/AppsFolder 都不好传 URL 参数），所以浏览器必须单独解析 exe |
| 设置存储 | `src/features/settings/storage.ts` | localStorage `nexora.settings.v1`，读时 `normalizeSettings` 兜底 | 命令建议单独一个 key，不塞进 Settings，避免设置对象膨胀 |
| 导出/导入 | `src/features/settings/transfer.ts` | 版本 1，只有 `settings`；`looksLikeSettings` 判断合法性 | 需要扩成 v2（`settings + commands`），并保持对 v1 的兼容读取 |
| 设置 UI | `src/components/SettingsPanel.tsx`、`settings/sections.ts` | 行式布局 + 滑块/分段/开关；侧边栏已有灰态「插件」入口 | 命令管理需要一个**新的设置页**（列表 + 编辑器），不是加一行开关 |
| 原生对话框 | `src-tauri/src/platform/dialogs.rs` | 手写 Win32 `GetOpenFileNameW` | 「手动选择浏览器 exe」可以直接复用这个模式 |

**缺口清单（P0 要补的）：**

1. 自定义命令的数据模型、校验、存储、跨窗口同步；
2. 查询解析（关键词 → 作用域）与标签 UI；
3. 浏览器枚举 + 指定浏览器打开；
4. 设置页的命令增删改查；
5. 导出/导入 v2。

---

## 2. 竞品给出的成熟范式

| 产品 | 机制 | 触发方式 | 参数写法 | 指定打开程序 | 管理方式 |
| --- | --- | --- | --- | --- | --- |
| **Raycast Quicklinks** | 一条「链接 + 名称 + 图标 + Open With + Tags」 | 在根搜索里按名称搜到后回车；也可配别名/热键 | `{argument name="query"}`，支持动态占位符（日期/剪贴板/选中文本） | ✅ 每条可指定 App | 设置页列表 + JSON 导入导出 + 官方预设库 |
| **Alfred Custom Searches** | 关键词 + URL 模板 | 关键词 + 空格 + 参数 | `{query}`，可选 UTF-8 编码、空格编码为 `+` | 走默认浏览器（可用 workflow 扩展） | 设置页表单 + `alfred://` 分享链接 |
| **uTools 关键词** | 关键词绑定插件 | 关键词 + 空格进入插件 | 插件自行处理 | 插件能力 | 插件市场 + 本地插件 |
| **PowerToys Run / CmdPal WebSearchShortcut** | 每个搜索引擎一条配置 | 关键词 + 参数 | 模板 URL | ✅ 每条可选浏览器 | 设置页 + winget 安装 |

**共同范式（可以直接照搬）：**

- **关键词 + 空格**进入命令作用域，后续文本即参数；
- URL 模板里用**占位符**替换参数（Raycast 用 `{argument}`，Alfred 用 `{query}`）；
- **每条命令可以指定用哪个程序打开**（这正是你要的「配置到 Edge 上」）；
- 命令库支持**导入/导出 JSON**，能分享、能备份；
- 预设库降低首次配置成本（Bing/Google/GitHub/B站…）。

---

## 3. 需求拆解：三个层次，别混在一起做

### L1 声明式命令（P0/P1）——数据驱动，不执行任意代码

一条命令 = `关键词` + `动作` + `参数模板`。动作先支持四种：

| 动作 | 例子 | 复杂度 |
| --- | --- | --- |
| `web-search` | `https://cn.bing.com/search?q={query}` + Edge | 低 |
| `open-url` | 固定打开 `https://github.com/notifications` | 低 |
| `open-path` | 打开 `D:\Projects\nexora` | 低 |
| `launch` | 启动 `code.exe "D:\Projects"` | 中（参数校验 + 安全提示） |

> 你描述的「配置一个网页搜索地址，再配置到 Edge 浏览器上」= `web-search`。

### L2 脚本命令（P2）——外部进程，stdout 返回结果

一条命令 = `关键词` + `程序/脚本路径` + `参数模板`，运行时把 `{query}` 作为参数传入，读取 stdout：

- stdout 为 JSON 数组 → 直接渲染成结果列表（Raycast Script Commands / Alfred Workflow 的做法）；
- stdout 为纯文本 → 作为一条结果展示 / 复制。

它**没有 UI 运行时**，但能覆盖「调用 API 查东西」「执行一段 PowerShell」「批处理文件」等大量真实需求，成本远低于插件。

### L3 插件（P3）——第三方代码 + 声明式或自定义 UI

一条插件 = `manifest` + `入口 JS/WASM` + 可选的 UI 描述。这才叫「插件生态」，需要：运行时隔离、能力白名单、权限确认、崩溃隔离、版本/更新、审核与签名、市场。

**建议：P0 只做 L1，P2 做 L2，L3 等有真实第三方需求再启动。**

---

## 4. 数据模型与存储

### 4.1 类型定义（放在 `src/core/commands.ts`）

```ts
export type BrowserRef = "default" | string; // string = 浏览器 id（exe 路径的稳定哈希或注册表键）

export type CommandAction =
  | { kind: "web-search"; urlTemplate: string; browser: BrowserRef }
  | { kind: "open-url"; url: string; browser: BrowserRef }
  | { kind: "open-path"; path: string };

export interface CustomCommand {
  id: string;
  keyword: string;        // 触发词：edge / bd / gh，小写，1–16 字符，不含空白
  title: string;          // 「用 Edge 搜索」
  subtitle?: string;
  icon: string;           // 复用 CommandGlyph 的图标名，或 "globe"
  action: CommandAction;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}
```

### 4.2 校验规则（写入和读取都要跑）

| 字段 | 规则 | 失败处理 |
| --- | --- | --- |
| `keyword` | `/^[a-z0-9][a-z0-9_-]{0,15}$/i`，去重（大小写不敏感），不能与内置命令关键词冲突 | 表单内联报错 |
| `urlTemplate` | 必须 `http://` 或 `https://` 开头；`web-search` 必须含 `{query}` | 表单报错 + 保存禁用 |
| `browser` | `"default"` 或枚举结果里的 id；找不到时回退 `"default"` 并提示 | 保存时提示 |
| `title` | 1–40 字符 | 沿用 keyword 作默认值 |
| 条数 | 建议上限 200 条 | 超出禁止新增 |

### 4.3 存储

- **单独 key**：`nexora.commands.v1`（不要塞进 `nexora.settings.v1`）。
- 读取时 `normalizeCommands(parsed)`：过滤非法项、去重 keyword、补默认值——与 `normalizeSettings` 同一套路，保证旧数据/手改 JSON 不会让应用白屏。
- 两个窗口通过 `storage` 事件同步（`useLauncher` 里已有 recents 的同样写法）。
- 「重置所有设置」默认**不**删除命令；单独给一个「清空全部命令」的危险操作，避免误伤。

### 4.4 导出/导入兼容

当前格式：

```json
{ "app": "NEXORA", "version": 1, "exportedAt": "…", "settings": { … } }
```

扩展为 v2：

```json
{ "app": "NEXORA", "version": 2, "exportedAt": "…", "settings": { … }, "commands": [ … ] }
```

- 导入 v2：`commands` 走 `normalizeCommands`；
- 导入 v1：视为「只有设置」，命令库保持不变（或提示「该文件不含命令」）；
- 导出始终写 v2。

---

## 5. 交互设计（这是最容易做错的部分）

### 5.1 目标时序

```
① 输入 "edge"
   → 结果里出现：[用 Edge 搜索]（自定义命令，回车进入作用域）
                  [Microsoft Edge]（应用，回车启动）

② 按空格
   → 立即进入作用域：输入框左侧出现 ⌈🌐 Edge⌉ 标签，输入框清空
   → 结果区只显示一条动作项：「在 Edge 中搜索 "…"」

③ 输入 "rust 生命周期"
   → 动作项实时更新标题与副标题（显示 cn.bing.com）

④ 回车
   → 拼 URL：https://cn.bing.com/search?q=rust%20%E7%94%9F%E5%91%BD%E5%91%A8%E6%9C%9F
   → msedge.exe "<url>"（或默认浏览器）
   → 启动器隐藏（窗口失焦本来也会隐藏）

⑤ 退格（输入框已空）
   → 移除标签，回到普通搜索
   Esc
   → 有标签时先退出标签态；再按一次才清空/关闭
```

### 5.2 两个入口都要有

| 入口 | 行为 | 目的 |
| --- | --- | --- |
| **关键词 + 空格**（自动） | 进入作用域、清空输入框、聚焦参数 | 高频路径，最少击键 |
| **纯关键词 + 回车**（选择结果项） | 同样进入作用域（`keepOpen`） | 可发现性；也满足你原始描述的「回车变标签」 |

### 5.3 作用域状态下的规则

- 结果区**只保留一条动作项**，不再展示 app/file 模糊结果——用户已经显式声明意图，混排只会干扰。
- 参数为空时：
  - 模板不含 `{query}` → 回车直接打开（等价于固定链接）；
  - 模板含 `{query}` → 动作项副标题提示「输入内容后回车」，回车打开空查询页（可接受）或 no-op（二选一，建议后者更保守）。
- 标签上点 `×` 或退格（输入框为空）退出作用域。
- 中文输入法：`App.tsx` 已有 `event.isComposing || event.keyCode === 229` 的保护，解析逻辑要基于**输入框当前文本**而不是 keydown 字符，避免拼音过程中误触发。
- 关键词与 app 重名（`edge` 既是应用又是关键词）：**只有「关键词 + 空格」才抢占**；单独输入 `edge` 时两者都展示，互不影响。

### 5.4 解析函数（纯函数，方便单测）

```ts
export function parseQuery(raw: string, commands: CustomCommand[]) {
  const match = /^(\S+)[ \t]+([\s\S]*)$/.exec(raw);
  if (!match) return { scope: null, argument: raw };
  const keyword = match[1].toLowerCase();
  const command = commands.find(c => c.enabled && c.keyword.toLowerCase() === keyword);
  return command
    ? { scope: command, argument: match[2] }
    : { scope: null, argument: raw };
}
```

> 建议引入 `vitest`，先给 `parseQuery` / `buildUrl` / `normalizeCommands` 三个纯函数补单测——它们最容易在后续迭代里回归。

### 5.5 最近使用

- 执行后记录 `{ kind: "command", id: "custom:<id>", title }` 到现有 recents；
- 从最近列表点选时**重新进入作用域**（不重放上次参数，避免误发搜索和隐私泄露）；
- 注意现有 `startItems` 会把「找不到对应内置命令」的 recent 直接丢掉（`useLauncher.ts` 里 `commands.find(...)`），自定义命令要在这里补一个分支，否则最近记录会消失；
- 排序：P1 再加「按使用频率 + 最近使用」加权。

---

## 6. 执行层实现（Rust 侧）

### 6.1 默认浏览器：已经能用

`open_path` 里 `open::that(path)` 对 `https://…` 有效。新增一个语义更明确的命令即可：

```rust
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    ensure_http_url(&url)?;           // 只允许 http/https
    open::that(&url).map_err(|e| format!("Could not open URL: {e}"))
}
```

### 6.2 指定浏览器：枚举 + spawn

Windows 上浏览器都注册在 `StartMenuInternet`；这是比「扫 Program Files」更稳的来源：

```
HKLM\SOFTWARE\Clients\StartMenuInternet\<BrowserId>
HKCU\SOFTWARE\Clients\StartMenuInternet\<BrowserId>
  (Default)                                  → 显示名
  shell\open\command (Default)               → "C:\...\msedge.exe" --single-argument %1
  LocalizedString                            → 本地化显示名（可选）
```

```rust
#[derive(serde::Serialize)]
pub struct BrowserInfo { pub id: String, pub name: String, pub exe: String }

#[tauri::command]
pub fn list_browsers() -> Vec<BrowserInfo> { /* 读注册表、去重、过滤不存在 exe */ }

#[tauri::command]
pub fn open_url_in(url: String, browser: Option<String>) -> Result<(), String> {
    ensure_http_url(&url)?;
    match browser {
        None => open::that(&url).map_err(|e| e.to_string()),
        Some(exe) => {
            let exe = std::path::PathBuf::from(exe);
            if !exe.exists() { return Err("Browser not found".into()); }
            std::process::Command::new(exe)
                .arg(&url)                    // 参数数组，绝不拼 shell 字符串
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Could not launch browser: {e}"))
        }
    }
}
```

实现要点：

- 需要给 `windows` crate 加 `Win32_System_Registry` feature（当前 `Cargo.toml` 未开）。
- 命令路径解析：去掉引号，截到 `.exe` 为止，忽略后面的参数占位（`%1`、`--single-argument`）。
- 去重按 exe 路径大小写不敏感；过滤 `!exe.exists()`。
- 便携版/企业定制浏览器不在注册表里 → 设置页提供「手动选择 exe…」，复用 `dialogs.rs` 的 Win32 打开文件对话框模式。
- 「新窗口」开关（P1）：Edge/Chrome 用 `--new-window`，Firefox 用 `-new-window`，其余浏览器不加参数（默认新标签页即可）。
- **不要用 `open::with`**：它的参数拼接在不同浏览器上行为不一致，直接 spawn exe 更可控。

### 6.3 URL 模板与编码（前端做，Rust 只校验）

```ts
const TEMPLATE_TOKEN = "{query}";

export function buildUrl(template: string, argument: string): string {
  return template.replaceAll(TEMPLATE_TOKEN, encodeURIComponent(argument));
}
```

- 先 `encodeURIComponent` 再替换，避免参数里的 `&`、`#`、空格破坏 URL；
- 记住 Alfred 的经验：少数站点要求空格变 `+`，P1 可加一个「空格编码为 +」的选项；
- 只允许 `http/https`（`javascript:`、`file:`、`data:` 一律拒绝），未来支持 `ms-settings:` 等协议时单独开白名单。

### 6.4 安全边界（L1 就已经存在）

| 风险 | 处理 |
| --- | --- |
| 恶意 URL 模板 | scheme 白名单 + 长度上限 |
| 参数注入 | `encodeURIComponent`；Rust 用参数数组，不经过 `cmd /c` |
| 命令数据被篡改 | localStorage 同源；导入文件时逐条校验 + 明确提示「导入命令会新增可执行项」 |
| `launch` 动作（P1） | 表单展示最终命令行；首次保存弹确认；禁止 `cmd.exe /c` 字符串拼接 |
| 脚本命令（P2） | 安装式信任、显示完整命令、超时、输出大小上限、无管理员权限 |

---

## 7. 设置界面

### 7.1 放哪里

建议新增一个设置页 **「命令」（Commands）**，与「通用 / 外观 / 个人」并列；**灰态「插件」页保持不动**，等 L2/L3 再点亮。

理由：命令是「你现在就能用的东西」，插件是「以后装别人写的东西」，混在一个页里会让用户以为插件已经能用。

### 7.2 页面结构

```
命令
├─ 说明 + [添加命令] 按钮
├─ 命令列表（可拖动排序，P1）
│   ├─ 🌐 Edge 搜索     edge   https://cn.bing.com/search?q={query}   [启用] [编辑] [删除]
│   └─ 🐙 GitHub        gh     https://github.com/search?q={query}
└─ [导入预设] [从 JSON 导入]（P1）
```

编辑器字段（行内展开，不另开窗口）：

| 字段 | 控件 | 说明 |
| --- | --- | --- |
| 名称 | 输入框 | 结果项标题，如「用 Edge 搜索」 |
| 关键词 | 输入框 | 前缀 `关键词直接触发`，实时查重 |
| 搜索地址 | 输入框 | 占位符提示 `{query}`；下方实时显示拼好的示例 URL |
| 打开方式 | 下拉 | 系统默认 / 枚举出的浏览器 / 手动选择 exe |
| 图标 | 下拉 | 先复用内置图标集（globe / search / 各命令图标） |
| 启用 | 开关 | 停用后不参与匹配 |

### 7.3 首次体验

- 内置 3–5 条预设（Bing、Google、GitHub、B 站、知乎），首次打开展示为「推荐添加」，一键加入；
- 空状态给一条可点击的示例（`edge` + Bing）；
- 保存后立即生效（storage 事件同步到启动器窗口）。

---

## 8. 插件架构选型（P2/P3）

### 8.1 四种形态对比

| 方案 | 代表 | 隔离级别 | 能力 | 开发成本 | 适合阶段 |
| --- | --- | --- | --- | --- | --- |
| **外部脚本**（stdout JSON） | Raycast Script Commands、Alfred Workflow | 进程级（弱隔离，靠用户信任） | 文件、网络、任意本地程序 | 低 | **P2** |
| **内嵌 JS 引擎**（QuickJS / Boa） | uTools（Node + preload）、Raycast（Node + 宿主 API） | 中（API 白名单） | 由宿主注入 | 中高 | P3 |
| **WASM**（wasmtime / Extism） | 新一代插件沙箱 | 强（内存隔离 + 能力注入） | 由宿主注入 | 高（作者工具链重） | P3+ |
| **WebView 内嵌 UI** | uTools 自定义窗口、VS Code Webview | 弱（同进程渲染） | 几乎全部 | 中 | P3（体验最好，风险最高） |

### 8.2 推荐路径

1. **P2 先做脚本命令**：manifest 只有元数据 + 程序路径 + 参数模板，运行在独立进程，读 stdout 的 JSON；不做持续运行的插件进程，不做 UI。
2. **P3 再做插件运行时**：
   - 如果优先「插件作者好写」→ QuickJS + 宿主 API（`storage` / `http` / `clipboard` / `ui`），像 Raycast/uTools；
   - 如果优先「安全与可审计」→ Extism/WASM + 能力清单（WIT），冷启动和生态成本更高。
3. **UI 先做声明式**（列表 / 表单 / 详情三件套），不要一上来开放任意 React 渲染；自定义 UI 用独立 WebView + 受限 preload 桥，作为 P3 的第二阶段。

### 8.3 脚本命令协议草案（P2）

```json
{
  "title": "查询 IP",
  "keyword": "ip",
  "program": "powershell.exe",
  "args": ["-NoProfile", "-Command", "…"],
  "queryArg": true,
  "timeoutMs": 5000,
  "output": "json"
}
```

stdout 约定：

```json
[
  { "title": "8.8.8.8 · Google", "subtitle": "Mountain View", "openUrl": "https://…" }
]
```

- 没有 stdout / 解析失败 → 展示一条错误结果 + toast，不弹系统对话框；
- 超时用 `tokio::time::timeout` 或线程 + `Child::kill`；
- 输出超过 256 KB 截断；
- 默认不继承管理员权限；需要提权的动作交给用户自己配置。

### 8.4 插件的安全模型（P3 必须提前想清楚）

- **信任链**：本地安装要显式确认；市场插件要审核 + 可撤销；
- **能力声明**：`network` / `filesystem` / `clipboard` / `shell` 分开授权，运行时按需询问；
- **崩溃隔离**：插件进程崩溃不能影响启动器；要有看门狗与超时；
- **数据边界**：插件只能读自己的存储目录，不能读 `nexora.settings.v1`；
- **更新**：插件自带版本号 + 最小宿主版本；不自动热更新。

### 8.5 明确不做的（第一版）

- 不做任意 UI 渲染的插件；
- 不做插件市场与支付；
- 不做插件间互相调用；
- 不做云同步插件配置。

---

## 9. 优先级与路线图（重点）

### 9.1 总表

| 优先级 | 范围 | 价值 | 依赖 | 粗估 | 验收标准 |
| --- | --- | --- | --- | --- | --- |
| **P0** | 自定义 Web 搜索命令：关键词 + 空格 → 标签；URL 模板；指定浏览器；设置页增删改；导入导出 v2 | ★★★★★ 直接满足你举的例子，覆盖 80% 日常「打开某站搜索」需求 | 无 | 10–14 人日 | 输入 `edge ` + 文本 + 回车，Edge 打开 Bing 结果；重启后仍在；导出的 JSON 能还原 |
| **P1** | 更多动作（open-url / open-path / launch）、按名称触发、预设库、别名与排序、新窗口选项 | ★★★★☆ 让命令从「搜索」扩展到「一切快捷动作」 | P0 | 5–8 人日 | 能配置「打开项目文件夹」「启动 VS Code 并打开工程」 |
| **P2** | 脚本命令：manifest + 进程执行 + stdout JSON → 结果列表 + 信任提示 | ★★★★☆ 覆盖「查天气/查 IP/调 API/跑脚本」，是插件之前的最大公约数 | P0 的存储与设置页 | 10–15 人日 | 写一条 `ip` 命令，输入 `ip ` 回车能看到 API 返回的 3 条结果 |
| **P3** | 插件运行时（QuickJS 或 Extism）+ 声明式 UI + 权限 + 安装/更新 | ★★★☆☆ 生态价值高，但只有前两层跑顺后才值得投入 | P2 的协议与权限模型 | 4–8 周（MVP），市场另计 | 能安装一个第三方插件并调用宿主 API 完成一次完整交互 |

### 9.2 P0 任务拆解（可以直接排期）

| # | 任务 | 位置 | 人日 |
| --- | --- | --- | --- |
| 1 | 命令类型 + 校验 + `normalizeCommands` + 单测 | `src/core/commands.ts`、`src/features/commands/storage.ts` | 1.5 |
| 2 | 查询解析 `parseQuery` / `buildUrl` + 单测 | `src/core/commands.ts` | 0.5 |
| 3 | 启动器作用域状态 + 结果项 + 执行（作用域下要跳过文件搜索的 35ms 防抖请求） | `useLauncher.ts`、`App.tsx` | 2 |
| 4 | 搜索框标签 UI（chip、退格、Esc、IME） | `SearchBar.tsx`、`launcher.css` | 1.5 |
| 5 | Rust：`list_browsers` / `open_url_in` / 手动选 exe | `commands/browsers.rs`、`dialogs.rs`、`Cargo.toml` | 2 |
| 6 | 设置页「命令」：列表 + 编辑器 + 浏览器选择 + 校验 | `SettingsPanel.tsx`、`settings/sections.ts`、`settings.css`、`i18n.tsx` | 3 |
| 7 | 导出/导入 v2 + 预设库 + 空状态 | `transfer.ts`、`sections.ts` | 1 |
| 8 | 联调与 QA（深浅色、11–17px、中文输入法、重名关键词、0 条命令） | 全链路 | 1.5 |

> 依赖顺序：1 → 2 → 3/4 并行 → 5/6 并行 → 7 → 8。

### 9.3 为什么是这个顺序（而不是先做插件）

1. **成本差一个数量级**：P0 是「数据 + 宿主解释」，P3 是「运行时 + 权限 + 生态 + 审核」。
2. **P0 能验证真实需求**：先看用户会不会配置第二、第三条命令，再决定 L2 的脚本协议长什么样。
3. **P0 的产物是 P2/P3 的地基**：命令模型、关键词作用域、设置页、导入导出、执行通道，全都会复用。
4. **风险可控**：P0 不执行任意代码，最坏情况只是打开一个错误 URL；P3 一旦权限设计失误就是安全事件。

### 9.4 附：值得同时补的「内置可执行功能」

这些不依赖插件系统，直接在 `useLauncher` 的 `commands` 数组里加即可，能显著提升「这个软件能干活」的感知：

| 功能 | 例子 | 价值 | 成本 |
| --- | --- | --- | --- |
| 计算器 | 输入 `12*8+3` 直接出结果 | ★★★★★ | 低 |
| 单位/汇率换算 | `100 usd to cny` | ★★★★☆ | 中（汇率需联网，P2） |
| 内置剪贴板历史 | 已有灰态命令，补全能力 | ★★★★☆ | 中 |
| 打开常用系统页面 | 已部分具备（下载/桌面/回收站） | ★★★☆☆ | 低 |
| 窗口管理 | `left` / `max` 之类 | ★★★☆☆ | 中 |
| 时间戳/编码/颜色工具 | `ts 1700000000`、`base64 …` | ★★★☆☆ | 低 |

建议把「计算器」放进 P0.5（1–2 人日），它是启动器类产品里投入产出比最高的内置命令。

---

## 10. 风险与待决策

| # | 问题 | 影响 | 建议 |
| --- | --- | --- | --- |
| 1 | 关键词与 app 重名（`edge`） | 用户想搜应用却被带进搜索命令 | 只在「关键词 + 空格」时抢占；单独输入仍展示应用 |
| 2 | 浏览器检测不全（便携版、企业定制、被卸载残留） | 「用 Edge 打开」失败 | 注册表枚举 + 手动选 exe + 失败回退默认浏览器并 toast |
| 3 | URL 模板失效（站点改版） | 打开了 404 | 编辑器里提供「测试」按钮 + 示例 URL 预览 |
| 4 | 参数被记录进最近使用 | 隐私 | 只记命令，不记参数；或提供「不记录搜索内容」开关 |
| 5 | 导入的 JSON 带恶意 `launch` 命令 | 任意代码执行 | P1 起：`launch` 首次使用需确认；导入时逐条展示 |
| 6 | 命令库与设置导出格式分叉 | 兼容性 | 统一 v2 格式，保留 v1 读取；写入时总是带 `commands` |
| 7 | 「插件」灰页与「命令」页定位重叠 | 用户困惑 | 命令页独立；插件页文案明确「后续支持」 |
| 8 | 是否需要云同步 | 范围膨胀 | P0 只做本地 + 文件导入导出 |

---

## 11. 参考来源

1. Raycast Manual — Quicklinks（URL 模板、`{argument}`、Open With、Tags、JSON 导入导出）  
   https://manual.raycast.com/quicklinks
2. Alfred Help — Create your own custom searches（`{query}`、UTF-8 编码、空格编码、无参数搜索）  
   https://www.alfredapp.com/help/features/web-search/custom-searches/
3. uTools 开发者文档 — 插件应用架构（Web UI + preload 桥 + Runtime、plugin.json、生命周期）  
   https://www.u-tools.cn/docs/development/plugin-architecture.html
4. Tauri v2 — Opener 插件（打开文件/URL、默认或指定应用、权限）  
   https://v2.tauri.app/plugin/opener/
5. `open` crate 文档（`open::that` / `open::with` 的语义与阻塞行为）  
   https://docs.rs/open/5.3.2/open/fn.with.html
6. PowerToys Run / CmdPal WebSearchShortcut（每条搜索配置可指定浏览器）  
   https://github.com/Daydreamer-riri/CmdPal-WebSearchShortcut
7. 本仓库既有调研 — 《Windows 启动器框架调研》第 7 节路线图（插件与平台化排序）  
   `docs/windows-launcher-framework-research.md`

---

## 12. 一句话建议

**先用两三周把「自定义搜索命令 + 关键词标签 + 指定浏览器」做成 P0，让用户真正用起来；再用脚本命令（P2）验证插件需求是否真实存在，最后才谈插件运行时（P3）。**

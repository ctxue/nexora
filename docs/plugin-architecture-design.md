# NEXORA 插件系统：可行性、竞品实现与设计方案

> 目标：回答三个问题——现在的 NEXORA 具备加插件的能力吗？市面上的插件是怎么做的？如果做，应该怎么设计、先做什么？
> 基线：NEXORA v0.2.0（2026-11-01），已完成自定义命令（网页搜索/打开网址/打开路径/运行程序）、关键词标签、便携应用。
> 关联文档：[自定义命令与插件功能调研](command-and-plugin-research.md)、[插件开发指南](plugin-development.md)
> 实施状态：**进程外插件 MVP 已落地**（manifest、安装/卸载、JSON-RPC search、启停、独立进程、日志到 stderr），宿主 API、匹配规则与自定义 UI 尚未实现。

---

## 0. 结论先行

1. **能加，但不是"打开一个开关"就能加。** 命令层（数据结构、关键词、执行通道、设置页、导入导出、i18n）已经具备，约占插件系统一半的表面工作；**运行时层（隔离执行、通信协议、生命周期、权限、分发）目前是 0**。
2. **市面主流有三条路线**：宿主内运行时（Raycast、uTools、PowerToys Run、Flow Launcher）、进程外脚本（Alfred Workflow）、WASM 沙箱（Extism 一类）。能力越强，安全与维护成本越高。
3. **最合适的做法不是发明，而是组合四家已被验证的成熟做法**：
   - **uTools** 的插件清单与关键词模型（`plugin.json` + `features/cmds`）
   - **Alfred** 的执行模型与结果协议（进程外 + JSON items）
   - **Chrome** 的权限模型（manifest 声明 + 安装时逐条确认）
   - **VS Code** 的宿主隔离思路（独立 host 进程 + RPC，简化为每插件一进程）
4. **不建议照搬 Raycast / uTools 的运行时**：它们是 Electron/Node 生态，NEXORA 是 Tauri + WebView2，没有 Node，也没有必要为了插件再塞一个 Node/Electron 运行时。
5. **建议路线：先做「进程外插件」（L2），4–6 周能出可用版本；JS/WASM 运行时与自定义 UI 放到 L3，市场最后做。** 理由：能复用现有 Rust 进程启动能力、语言无关、崩溃隔离、不需要引入新运行时依赖，且足够覆盖 80% 的真实插件需求。
6. **不要做**：一上来做 Raycast/uTools 那种"任意 UI + 商店"；也不要允许插件直接在自己的 DLL/Node 里跑（PowerToys/Flow 的模型）——对独立开发者来说，安全与审核成本会失控。

---

## 1. 现在的 NEXORA 具备什么

| 能力 | 现状 | 对插件的价值 |
| --- | --- | --- |
| 命令数据模型 | `CustomCommand`：关键词 + 4 种 action（web-search / open-url / open-path / launch） | 可直接映射成插件 manifest 的 commands，L1 扩展零成本 |
| 关键词作用域 | `parseQuery` + 标签 + `acceptsArgument` | 插件的"关键词 + 空格 + 参数"交互可以完全复用 |
| 搜索结果渲染 | `LauncherItem` → `ResultList` / `RecentList` | 插件只需返回同构的 item，不用碰 UI |
| 执行通道 | `LauncherItem.run()` + Tauri `invoke` | 插件动作最终也走同一通道 |
| 进程启动 | `launch_program`（`Command::new` + `raw_arg`） | 进程外插件的雏形已经有了 |
| 设置与持久化 | `nexora.commands.v1`、设置页、导出/导入 v2 | 插件列表、启用、权限的 UI 有现成模式 |
| 国际化 | 全局 `t()` + 中英词典 | 插件名称/描述可走 manifest 自带文案，宿主 UI 复用现有词典 |
| 原生能力 | 浏览器枚举、URL 打开、文件/文件夹选择、目录扫描 | 可直接包装成宿主 API 暴露给插件 |

### 缺什么（按重要性排序）

1. **动态结果提供者**：现在的结果来自 `buildGroups()` + 命令数据，是"编译期/数据期"的；插件需要在运行时异步返回结果，并支持取消与流式更新。
2. **插件包与清单**：没有 manifest 规范、安装目录、版本、依赖、卸载。
3. **宿主 ↔ 插件协议**：没有 IPC、请求/响应、超时、背压。
4. **生命周期与隔离**：没有懒启动、保活、崩溃重启、并发上限、进程树清理。
5. **权限模型**：没有"这个插件能联网/读剪贴板/执行命令"的声明与授权。
6. **插件 UI**：没有自定义界面（列表之外的详情、表单、WebView）。
7. **分发与更新**：没有插件市场、签名、自动更新、审核。
8. **基础设施**：没有插件日志、诊断页、崩溃上报、存储隔离。

> 一句话评估：**命令层约 80% 就绪，运行时层 0%。** 插件不是"再加一个页面"，而是要在现有命令系统上再长出一层"可执行扩展"的宿主。

### 加插件前必须先还的架构债

| 债 | 影响 | 建议 |
| --- | --- | --- |
| `LauncherItem.run` 是闭包 | 插件结果无法序列化/跨进程执行 | 给 item 增加可序列化的 `action` 描述，由宿主执行；内置项保留闭包 |
| 结果构建是同步的 | 插件搜索必然异步 | 抽 `ResultProvider` 接口，`useLauncher` 合并多来源（含取消） |
| 存储只有 localStorage | 插件需要独立目录、配额、隔离 | Rust 增加 app data 目录与插件存储命令 |
| 没有日志/诊断 | 插件失败不可观测 | 每插件一份日志 + 设置页"最近错误" |
| `platform` 调用散落 | 宿主 API 边界不清 | 收敛成 `host` 模块，插件只能通过它访问系统 |
| 单一 React 窗口 | 自定义 UI 无处安放 | L3 再用独立 WebView 窗口/面板，v1 不做 |

---

## 2. 市面上的插件都是怎么实现的

### 2.1 横向对比

| 产品/体系 | 插件形态 | 运行位置 | UI 能力 | 结果协议 | 隔离/权限 | 分发 |
| --- | --- | --- | --- | --- | --- | --- |
| **Raycast** | 扩展 = React + TS + Node 包 | 宿主内 Node 进程 | 宿主提供的声明式组件（List/Detail/Form…） | 扩展调用宿主 API 返回组件树 | 宿主 API + 商店审核；扩展本身拥有用户权限 | 官方 Store，审核 + 版本 |
| **uTools** | `plugin.json` + 网页入口 + `preload.js` | Electron `BrowserView` + preload（Node） | 完整网页 UI（可用 React/Vue） | 插件直接操作页面 + `window.utools` API | preload 暴露 Node 能力；商店审核；无强沙箱 | 插件市场，审核 |
| **Alfred** | Workflow = 脚本/二进制 + 配置 | 独立进程（脚本） | 无自定义 UI（结果列表 + 少量内置面板） | stdout JSON：`items[].title/subtitle/arg/icon`、`rerun`、`cache` | 无沙箱，脚本拥有用户权限 | 社区分享/可导入 |
| **PowerToys Run** | C# 插件 DLL | 宿主进程内加载 | 无（标准结果列表） | 实现 `IPlugin` 接口返回结果 | 进程内、无沙箱；与 PowerToys 版本耦合 | 随仓库/社区分发 |
| **Flow Launcher** | 以 .NET 插件为主 | 宿主进程内加载 | 无（标准结果列表） | 插件接口 / JSON-RPC 桥接其他语言 | 进程内、无沙箱 | 插件商店 + 手动安装 |
| **VS Code** | 扩展 = Node 包 + `package.json` | 扩展宿主进程（独立 Node） | Webview（自定义 HTML） | 扩展 API | 扩展宿主隔离 + 工作区信任 + Marketplace 审核 | Marketplace |
| **Chrome 扩展** | `manifest.json` + JS | 浏览器渲染进程/Service Worker | 弹窗/注入页面 | 浏览器 API | manifest 权限 + 沙箱 + 商店审核 | Chrome Web Store |
| **Extism（WASM）** | 任意语言编译成 wasm | WASM 沙箱 | 无（返回数据） | 导出函数 + host functions | 内存隔离 + 能力注入 | 自建/任意 |

> 参考：[Raycast API 介绍](https://developers.raycast.com/readme.md)、[uTools 插件架构](https://www.u-tools.cn/docs/development/plugin-architecture.html) / [plugin.json](https://www.u-tools.cn/docs/development/plugin-json.html)、[Alfred Script Filter JSON](https://www.alfredapp.com/help/workflows/inputs/script-filter/json/)、[PowerToys 新插件清单](https://microsoft.github.io/PowerToys/modules/launcher/new-plugin-checklist/)、[Flow Launcher](https://github.com/Flow-Launcher/Flow.Launcher)、[Extism](https://extism.org/docs/overview)。

### 2.2 五种实现范式

**A. 声明式结果（Alfred / uTools 的 match 指令）**
插件只返回数据（title/subtitle/icon/arg），UI 完全由宿主渲染。成本最低、最安全，但只能做"列表 + 执行一个动作"。

**B. 进程外脚本（Alfred Workflow）**
插件是任意可执行文件/脚本，宿主通过 stdout 读 JSON。语言无关、崩溃不影响宿主，但每次调用有进程启动开销，且无法做长驻状态。

**C. 宿主内运行时（Raycast / uTools / PowerToys / Flow）**
插件代码跑在宿主进程或宿主管理的 Node/Webview 里，能力最强（Node、任意 UI），但**插件与宿主同权限**，安全完全依赖商店审核与作者自觉；PowerToys/Flow 的 DLL 模型还会与宿主版本强耦合。

**D. WASM 沙箱（Extism / wasmtime）**
插件编译成 wasm，只能调用宿主显式注入的 host functions。隔离最好、跨语言，但作者工具链重、生态小、冷启动与调试体验一般。

**E. 自定义 UI（uTools BrowserView / VS Code Webview）**
插件自带完整网页界面。体验上限最高，但要做窗口/面板管理、主题注入、焦点与快捷键隔离，且安全面最大。

### 2.3 可以照搬的通用做法

- **manifest 先行**：id、name、version、minHost、入口、关键词、匹配规则、权限。
- **关键词 / 正则 / 文件 / 选中文本 多入口匹配**（uTools 的 `cmds` 模型最完整）。
- **声明式结果 + 宿主执行动作**：插件不直接调系统 API，而是返回 `actions`。
- **首次安装权限确认 + 商店审核 + 签名**。
- **懒启动 + 超时 + 输出上限 + 崩溃隔离**。
- **每插件独立存储 + 日志**。

---

### 2.4 抄哪几家、不抄哪几家

既然"市面上已经有现成的"，关键不是选一家整体照搬，而是**按层次取成熟方案**：

| 来源 | 抄什么 | 不抄什么 | 原因 |
| --- | --- | --- | --- |
| **uTools** | `plugin.json` 的清单结构、`features/cmds` 的功能/匹配指令、关键词 + 空格进入 | Electron `BrowserView` + preload 的运行时 | 清单模型被验证过、可读性好；但 NEXORA 不是 Electron，塞一个 Chromium 运行时得不偿失 |
| **Alfred** | 进程外执行、stdout JSON items（title/subtitle/icon/arg/actions）、rerun/cache 思路 | 每次按键都起进程 | JSON 结果协议简单、语言无关；但逐键拉起进程延迟不可接受，改成常驻 + 懒启动 |
| **Chrome** | manifest 权限声明、安装时逐条确认、最小权限 | 渲染进程沙箱（WebView2 已替我们做了一部分） | 权限模型是插件生态的通用语言 |
| **VS Code** | 独立 host 进程 + RPC + API 版本化的思路 | Node 扩展宿主、Webview 自定义 UI | 隔离思路正确；但不需要 Node，也不需要 v1 就做 Webview |
| **Raycast** | 主机渲染声明式结果（List/Detail/Action）、商店审核（远期） | React/Node 扩展运行时 | "宿主渲染、插件只给数据"是正确边界；运行时不可直接搬 |
| **Extism** | WASM 沙箱 + host functions 的模型（远期） | 现在就上 | 隔离最强，但工具链与生态成本高，适合 L3 |

**为什么不能直接接一个现成插件框架？**

- Raycast 的 API 不是可嵌入的库，而是它自家宿主 + Store 的契约；
- uTools 的插件是 Electron 页面，依赖它的 `window.utools` 与 preload 桥；
- PowerToys / Flow 的插件是 .NET DLL，进程内加载，和 Rust 宿主 ABI 不通、也没有隔离；
- Extism 是通用 WASM 插件底座，不提供 launcher 的清单、搜索、结果协议，仍要自己设计上层。

所以最终方案是**组合**，一句话：

```
清单（uTools） → 宿主监管（VS Code 思路） → 子进程（Alfred） → JSON 结果（Alfred） → 宿主渲染 + 权限（Chrome）
```

这套组合的额外好处：**MVP 不需要新增任何运行时依赖**——`serde_json`、`dirs`、`std::process` 都已在项目里，Rust 侧只写监管与协议，前端只写 provider 与设置页。

---

## 3. 给 NEXORA 的三层扩展模型

### L1 声明式扩展（现在就绪度最高）

只写 JSON，不跑代码：网页搜索、打开网址/路径、运行程序，加上 uTools 风格的匹配规则（正则、文件类型、选中文本）。

- 复用：`CustomCommand`、设置页、导入导出、作用域标签。
- 新增：匹配规则类型、插件包（一个 JSON 可含多条命令）。
- 风险：低；无需权限模型。
- 适合：把"命令"正式命名为"扩展"，让用户先形成心智。

### L2 进程外插件（建议的 MVP）

`plugin.json` + 一个可执行入口；宿主用 stdin/stdout JSON-RPC 调用；插件返回声明式结果；宿主提供受控 API。

- 复用：Rust 进程启动、`LauncherItem` 渲染、设置页。
- 新增：manifest、安装目录、RPC、生命周期、权限、日志。
- 风险：中；进程有用户权限，但至少崩溃隔离。
- 适合：需要联网/读文件/调本地程序的真实插件（查 IP、翻译、Git 操作、FFmpeg…）。

### L3 内嵌 JS / WASM + 自定义 UI

QuickJS（rquickjs）或 WASM（Extism）运行插件代码，宿主注入 API；自定义 UI 用独立 WebView 面板。

- 复用：L2 的协议与权限模型。
- 新增：JS/WASM 运行时、UI DSL 或 WebView 容器、签名与市场。
- 风险：高；工期 1–2 个月起。
- 适合：生态化之后再考虑。

> **推荐顺序：L1 → L2 →（视生态）L3。** 不建议跳过 L2 直接做 L3。

---

## 4. L2 详细设计

### 4.1 插件包结构

```
plugins/
  com.example.ip/
    plugin.json
    ip.exe            # 或 index.js（L3）
    icon.png
    README.md
```

安装目录：`%LOCALAPPDATA%\com.nexora.desktop\plugins\<id>\`（Rust 用 `dirs::data_dir()` 推导；不要放进 WebView 的 localStorage）。

### 4.2 manifest 示例

```json
{
  "id": "com.example.ip",
  "name": "IP 查询",
  "version": "1.0.0",
  "minHost": "0.3.0",
  "author": "example",
  "description": "查询 IP 归属地",
  "icon": "icon.png",
  "runtime": { "type": "process", "entry": "ip.exe" },
  "keywords": ["ip"],
  "match": [
    { "type": "regex", "pattern": "^\\d{1,3}(?:\\.\\d{1,3}){3}$", "label": "查询 IP" }
  ],
  "permissions": ["network", "clipboard"],
  "timeoutMs": 3000
}
```

字段约束：

| 字段 | 说明 |
| --- | --- |
| `id` | 反向域名，全局唯一；同时是存储目录名 |
| `version` / `minHost` | 语义化版本；宿主低于 minHost 拒绝加载 |
| `runtime` | `process`（L2）/ `js`、`wasm`（L3） |
| `keywords` / `match` | 关键词触发、正则/文件/选中文本匹配 |
| `permissions` | `network` / `filesystem` / `clipboard` / `shell` / `notifications`，安装时逐条确认 |
| `timeoutMs` | 单次调用超时上限（宿主还有硬上限） |

### 4.3 通信协议（JSON-RPC 2.0，按行分隔）

宿主 → 插件：

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"apiVersion":1,"pluginId":"com.example.ip"}}
{"jsonrpc":"2.0","id":2,"method":"search","params":{"query":"8.8.8.8","keyword":"ip","locale":"zh"}}
{"jsonrpc":"2.0","id":3,"method":"execute","params":{"itemId":"ip:8.8.8.8","action":"open"}}
{"jsonrpc":"2.0","id":4,"method":"shutdown"}
```

插件 → 宿主（响应）：

```json
{"jsonrpc":"2.0","id":2,"result":{"items":[
  {"id":"ip:8.8.8.8","title":"8.8.8.8 · Google","subtitle":"Mountain View, US",
   "icon":{"type":"glyph","value":"globe"},
   "actions":[{"id":"open","type":"openUrl","url":"https://ipinfo.io/8.8.8.8"}]}
]}}
```

插件 → 宿主（主动调用宿主能力）：

```json
{"jsonrpc":"2.0","id":101,"method":"host.call","params":{"name":"clipboard.write","args":{"text":"8.8.8.8"}}}
```

规则：

- 一行一个 JSON，UTF-8；单行上限 256 KB，整次响应上限 1 MB。
- `search` 必须快速（建议 < 3 s）；超时由宿主 kill 并提示。
- 插件可以返回空结果；错误用 JSON-RPC error（code/message）。
- 支持 `notify` 做流式增量（P2 再上）。

### 4.4 生命周期

```
首次命中关键词/匹配
        │
        ▼
   spawn（进程 + 握手 initialize，最长 2s）
        │
        ▼
      idle ──(5 分钟无请求)──► kill
        │
        ├── search（并发上限 4，排队）
        ├── execute（超时 10s，超时 kill 整个进程树）
        └── 崩溃/异常输出 ──► 记录日志 + 禁用该插件（可手动恢复）
```

- **懒启动**：不预先启动所有插件，避免开机拖慢。
- **保活**：命中过的插件保活 5 分钟，减少冷启动。
- **并发**：全局最多 4 个插件进程同时工作；同一插件的请求串行。
- **清理**：Windows 必须杀进程树（Job Object 或 `taskkill /T`），避免残留。
- **失败熔断**：连续 3 次崩溃/超时 → 自动禁用 + 设置页提示。

### 4.5 宿主 API（按权限开放）

| API | 权限 | 说明 |
| --- | --- | --- |
| `storage.get/set/remove` | 无 | 每插件独立 JSON 文件，按 id 隔离 |
| `clipboard.read/write` | `clipboard` | 读/写剪贴板 |
| `open.url` / `open.path` | 无（受 scheme 白名单） | 只允许 http/https；路径需存在 |
| `notify` | `notifications` | 系统通知/toast |
| `http.request` | `network` | 由宿主代理请求（绕过 CORS、统一超时/大小限制）；L2 插件也可自己联网，但需要声明 |
| `log` | 无 | 写入插件日志 |
| `settings.get` | 无 | 读取宿主注入的插件配置 |

> 进程外插件在 OS 层面仍拥有用户权限（能直接读文件/联网）。**权限声明是"知情 + 宿主 API 管控"，不是真沙箱**，文档必须写清楚。真正的沙箱要等 L3 的 WASM/AppContainer。

### 4.6 结果与动作模型

```ts
interface PluginItem {
  id: string;
  title: string;
  subtitle?: string;
  icon?: { type: "glyph"; value: string } | { type: "url"; value: string };
  actions: PluginAction[];
  score?: number;          // 0–100，宿主排序参考
}

type PluginAction =
  | { id: string; type: "openUrl"; url: string }
  | { id: string; type: "openPath"; path: string }
  | { id: string; type: "copy"; text: string }
  | { id: string; type: "run"; program: string; args?: string }
  | { id: string; type: "plugin"; handler: string };  // 回调插件 execute
```

- 宿主执行动作前做参数校验（scheme、路径存在、长度）。
- `type: "plugin"` 表示交回插件处理（插件可调用宿主 API 完成复杂操作）。
- 插件结果进入独立的「插件」分组（或关键词作用域内），不污染内置结果。

### 4.7 搜索集成

```ts
export interface ProviderContext {
  query: string;
  argument: string;
  locale: "zh" | "en";
  signal: AbortSignal;
}

export interface ProviderResult {
  label: string;
  color: string;
  items: LauncherItem[];
}

export interface ResultProvider {
  id: string;
  /** 关键词命中或匹配规则命中时才被调用 */
  matches(query: string): boolean;
  search(context: ProviderContext): Promise<ProviderResult | null>;
}
```

`useLauncher` 的改造：

1. 把内置命令/自定义命令包装成 `ResultProvider`；
2. 查询变化时并行调用所有命中的 provider，带 `AbortController`；
3. 合并结果：命令/插件 → 应用 → 文件（沿用现有的分组排序）；
4. 插件慢时先渲染内置结果，插件返回后增量更新（或显示"插件搜索中…"）。

### 4.8 安装、更新与卸载

- **安装**：选择 `.zip` 或文件夹 → 校验 `plugin.json`（id/version/权限）→ 解压到 plugins 目录（防 Zip Slip：拒绝 `..` 与绝对路径）→ 首次安装弹权限确认。
- **更新**：同 id 新版本覆盖；`minHost` 不满足则拒绝；不做自动更新（P3 再说）。
- **卸载**：停止进程、删除目录、可选择保留插件数据。
- **签名**：P2 只做 SHA-256 校验和记录；P3 再做签名与市场。

### 4.9 插件设置页

启用灰色「插件」页，包含：

- 已安装列表（名称、版本、作者、状态、权限标签）
- 安装 / 卸载 / 启用 / 停用
- 权限详情与"撤销"
- 日志与最近错误（"复制诊断信息"）
- 开发者模式：打开插件目录

### 4.10 开发体验

- 提供 `create-nexora-plugin` 模板（Rust/Go/Node/Python 任一，输出 JSON-RPC）。
- 提供调试模式：`NEXORA_PLUGIN_DEBUG=1` 时打印完整 RPC 日志。
- 提供 3 个官方示例：`hello`（最小）、`ip`（联网）、`git`（本地命令）。

---

## 5. 与现有代码的改造清单

| 模块 | 改动 | 说明 |
| --- | --- | --- |
| `src/core/providers.ts`（新） | `ResultProvider` 接口 + 内置/自定义命令适配器 | 插件接入点 |
| `src/features/launcher/useLauncher.ts` | 多 provider 并行搜索、取消、合并、插件分组 | 保持现有 35ms 防抖 |
| `src/core/types.ts` | `LauncherItem` 增加可序列化 `action`，保留 `run` 兼容 | 插件 item 由宿主执行 |
| `src/platform/tauri.ts` | 新增 plugins API（list/install/remove/start/search/execute/logs） | 全部走 invoke |
| `src-tauri/src/commands/plugins.rs`（新） | 安装、卸载、进程管理、JSON-RPC、日志 | 核心工作量 |
| `src-tauri/src/state.rs` | 插件注册表与运行中进程表 | 线程安全 |
| `src-tauri/src/config.rs` | 插件目录、超时、并发、输出上限常量 | 与现有常量集中管理 |
| `SettingsPanel.tsx` | 点亮「插件」页 | 现有灰态入口 |
| `i18n.tsx` | 插件相关文案 | 中英 |
| 文档 | 插件开发指南 + 示例 | 生态的前提 |

Rust 依赖：进程管理用 `std::process` + 线程/通道即可；如果要异步，`tokio` 已在依赖树里。HTTP 代理若需要，`reqwest` 已在 `Cargo.lock` 中（注意 feature 裁剪）。**QuickJS/WASM 需要新增 crate（首次构建需联网）**，这也是建议 L2 先用进程外方案的原因之一。

---

## 6. 路线图与优先级

| 阶段 | 内容 | 工期（1 人） | 验收标准 |
| --- | --- | --- | --- |
| **P0 地基** | `ResultProvider` 抽象、异步合并、取消、插件分组骨架（无真实插件） | 4–6 天 | 内置命令与自定义命令行为不变；慢 provider 不阻塞 UI |
| **P1 MVP** | 插件 manifest、安装/卸载、进程管理、JSON-RPC、2 个示例插件、基础权限（network/clipboard）、日志 | 10–15 天 | 安装 `ip` 插件后输入 `ip 8.8.8.8` 看到结果并回车打开网页；崩溃/超时不影响启动器 |
| **P2 体验** | 插件设置页、权限确认/撤销、每插件存储、匹配规则（regex/files/选中文本）、更新与版本检查、诊断 | 8–12 天 | 插件可启停、可看日志、可撤销权限；错误可定位 |
| **P3 生态** | JS/WASM 运行时、声明式设置表单、自定义 UI 面板、签名、市场/审核 | 6–10 周 | 第三方插件可发布、安装、升级；有审核与回滚机制 |

**建议先做 P0 + P1（约 4 周），拿到真实反馈后再决定 P2/P3。**

### 为什么不直接做 L3（JS/WASM）

- 需要新增运行时依赖（rquickjs/wasmtime/extism），构建体积与启动成本上升；
- 需要设计完整的 JS API 与能力注入，工作量是 L2 的 2–3 倍；
- 没有 L2 的协议与权限实践，L3 的 API 很容易设计错；
- 市场上"能跑脚本"的插件已经覆盖大多数需求，UI 插件是少数。

### 不建议现在做

- 插件市场、支付、自动更新；
- 插件间互相调用；
- 任意 React/HTML 插件 UI；
- 云端插件配置同步。

---

## 7. 风险与决策点

| 风险 | 说明 | 缓解 |
| --- | --- | --- |
| 安全 | 进程外插件拥有用户权限，可能读文件/联网/执行命令 | 权限声明 + 首次确认 + 日志 + 仅本地安装；L3 再上沙箱 |
| 恶意插件 | 用户从网上装来路不明的插件 | 明确"仅安装可信来源"提示；P3 签名/审核 |
| API 不稳定 | 插件依赖宿主 API，改版会破坏生态 | `apiVersion` + `minHost` + 兼容层；L2 只暴露稳定子集 |
| 冷启动 | 每次启动插件进程有开销 | 懒启动 + 保活 + 并发上限 |
| 崩溃/卡死 | 插件死循环或崩溃 | 超时 kill 进程树 + 熔断禁用 |
| 版本耦合 | 插件按旧 API 写 | 语义化版本 + 宿主兼容策略 |
| 分发 | 没有市场就没人用 | 先做本地 zip + 官方示例 + 文档，P3 再市场 |
| Windows 特有 | 进程树清理、杀软误报、签名成本 | Job Object、代码签名、安装包白名单 |

### 关键决策（需要你拍板）

1. **L2 的插件运行时用"进程 + JSON-RPC"还是直接上 QuickJS？**
   建议先进程 + JSON-RPC：实现快、语言无关、隔离够用。
2. **是否允许插件自己联网，还是全部走宿主代理？**
   建议 P1 允许自己联网但必须声明 `network`；P2 再提供 `http.request` 代理给 JS 插件。
3. **插件 UI 做到什么程度？**
   建议 v1 只做声明式列表 + 动作，不做自定义 UI。
4. **是否做插件市场？**
   建议 P3 再评估；先本地安装 + 官方示例。
5. **权限提示的强度？**
   建议安装时逐条确认，运行中可撤销；不搞"全部允许"。

---

## 8. 结论

- **NEXORA 现在可以加插件，而且起点不低**：命令模型、作用域、渲染、执行、设置、导入导出都能复用；缺的是运行时与信任模型。
- **最短路径是 L2 进程外插件**：manifest + JSON-RPC + 声明式结果，4 周左右能跑通；L1 声明式扩展可以顺手并入同一套 manifest。
- **插件系统的难点不是"跑代码"，而是"安全、稳定、可维护"**：超时、崩溃隔离、权限、版本、日志、审核，一个都不能少。
- **建议的下一步**：先做 P0（provider 抽象）→ P1（进程外插件 MVP + 2 个示例）→ 用真实插件验证 → 再决定是否投入 L3 与市场。

---

## 9. 参考来源

1. Raycast API Introduction（React/Node/TS、宿主 UI 组件、Store）  
   https://developers.raycast.com/readme.md
2. uTools 插件应用架构（BrowserView + preload + Runtime）  
   https://www.u-tools.cn/docs/development/plugin-architecture.html
3. uTools plugin.json（features/cmds、regex/files/img/window 匹配）  
   https://www.u-tools.cn/docs/development/plugin-json.html
4. Alfred Script Filter JSON（items/arg/rerun/cache/variables）  
   https://www.alfredapp.com/help/workflows/inputs/script-filter/json/
5. PowerToys Run 新插件清单（C# 插件、进程内加载）  
   https://microsoft.github.io/PowerToys/modules/launcher/new-plugin-checklist/
6. Flow Launcher（.NET 插件体系）  
   https://github.com/Flow-Launcher/Flow.Launcher
7. Extism（WASM 插件系统与 host functions）  
   https://extism.org/docs/overview
8. 本仓库既有调研：《Windows 启动器框架调研》《自定义命令与插件功能调研》  
   `docs/windows-launcher-framework-research.md`、`docs/command-and-plugin-research.md`

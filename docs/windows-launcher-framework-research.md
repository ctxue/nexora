# Windows 启动器技术选型调研

> 调研时间：2026-10-31  
> 目标：Windows 优先，做一个 uTools / Raycast 类启动器；视觉现代、每个交互有顺滑动效；团队规模暂按 1–3 人估算。  
> 结论会随版本变化，文中的版本号用于说明“当前事实”，不是长期承诺。

---

## 1. 结论先行

### 1.1 它们用的是什么

| 产品 | 可确认的架构 | 关键事实 |
|---|---|---|
| **uTools** | **Electron + Web 插件** | 官方文档写明 Runtime 基于 Electron；当前文档版本为 **Electron 34.5.8 / Chromium 132 / Node.js 20.19.1**。插件主界面跑在 Electron `BrowserView` 里，自定义窗口用 `BrowserWindow`；前端可用 React、Vue 等任意 Web 框架，`preload.js` 作为 Node/Electron 能力桥接层。 |
| **Raycast v1（2020–2026）** | **Swift + AppKit 原生** | 核心是原生 macOS，绝大多数 UI 是自绘 AppKit 组件，几乎不用 SwiftUI；扩展用 React + TypeScript + Node.js，但 UI 是声明式的，由原生层统一渲染。 |
| **Raycast v2（2026）** | **自研混合架构** | macOS 宿主：Swift + AppKit；Windows 宿主：**C# + .NET 8 + WPF**；UI：**React + TypeScript**，跑在 **WKWebView / WebView2**；业务后端：**Node.js**；性能敏感层：**Rust**（自研文件索引器、同步等）。Raycast for Windows 于 2025-11-20 公测，Raycast 2.0 于 2026-05-14 公测。官方给出的内存量级为 **350–450 MB**，高于 v1 的 200–300 MB。 |

一句话：**uTools 是 Electron 插件容器；Raycast 已经不是“纯原生”，而是“原生壳 + Web UI + Node + Rust”的混合体。** 两者的共同点是：**今天的新版本都把 UI 放在了 Web 技术上**，区别在壳和插件模型。

### 1.2 我的推荐

**默认首选：Tauri 2 + React 19 + TypeScript + Vite + Tailwind CSS + Motion，Rust 负责 Win32 原生层。**

理由：

1. **动效上限和 Electron 一样**：Windows 上 Tauri 用 WebView2（Chromium），CSS transition/animation、Motion、View Transitions、Canvas/WebGL 都能用；决定动效好坏的是设计系统和渲染性能，不是 Electron 与 Tauri 的差别。
2. **更适合“常驻启动器”**：不打包 Chromium，安装包通常只有几 MB 到十几 MB，空闲内存通常明显低于 Electron；启动器是常驻软件，这一点是长期体验差异。
3. **原生能力够深**：Rust 可以直接调 Win32 / COM / DWM，做全局热键、窗口定位、Mica/Acrylic、应用枚举、USN/MFT 文件索引，比在 Electron 里写 native addon 更可控。
4. **未来可扩展**：UI 和平台适配层分离后，以后加 macOS 不需要重写 UI；Raycast 已经证明“共享 Web UI + 每平台原生壳”可行。

**但有一个重要前提：先做 2 周技术验证。** Tauri 在 Windows 上最容易翻车的不是 React，而是 **透明窗口 + Acrylic/Mica + 无白闪 + 中文输入法 + 多显示器 DPI**。这些如果过不了，立刻换 Electron，UI 代码基本可以复用。

**备选：Electron 45 + React。** 如果你是一个人、前端出身、要在 2–3 个月内出可用版本，或者第一天就要兼容 Node 插件生态，Electron 更稳。Raycast 官方也明确说：**对大多数公司，Electron 大概率是正确选择**；他们不用 Electron，是因为要深度控制 macOS 原生层、不想在 macOS 上打包 Chromium，而不是因为 Electron 做不出好 UI。

**不建议作为 v1 主方案：**

- **WinUI 3 纯原生**：微软官方推荐、Fluent 观感好，但 XAML 的设计迭代速度、第三方生态、复杂自定义动效的成本，对 1–3 人团队偏高；Raycast 团队也明确说 WinUI 3 仍年轻、风险大。
- **WPF 纯原生**：成熟，但默认视觉偏旧；要做出现代感，等于自己写一套控件和动效系统，成本不低于 Web。
- **Flutter**：动画引擎强，但 Windows 上的中文 IME、无障碍、原生壳集成、插件生态都有额外成本；做“Windows 原生感”要大量定制。
- **Qt/QML**：能力强，但 C++/QML 门槛、授权模式、团队招聘面都不适合小团队。
- **自研 C# + WebView2 + React + Node 混合**：这就是 Raycast 路线，控制力最强，但要维护 4 个运行时和一套 IPC/codegen；除非已经有规模，否则 v1 不值得。

---

## 2. 两者技术栈拆解

### 2.1 uTools：Electron 插件容器

官方文档的关键信息：

- Runtime 基于 **Electron** 构建，为插件提供 Chromium 渲染环境和 Node.js 能力。
- 当前文档标注：**Electron 34.5.8、Chromium 132、Node.js 20.19.1**。
- 插件主界面运行在 uTools 管理的 **Electron `BrowserView`** 中，默认附着在搜索框主窗口；用户可以把插件“分离”到独立窗口。
- 自定义窗口通过 `utools.createBrowserWindow()` 创建，本质是 Electron `BrowserWindow`。
- 前端是标准 Web（HTML/CSS/JS，可用 React/Vue），但前端不能直接访问 Node；需要 `preload.js` 暴露桥接接口。
- 插件退出到后台时，`BrowserView` 会从主窗口移除。

这套架构的优点是**插件开发门槛极低**：会写网页就能写插件，Node 能力通过 preload 暴露，生态容易做大。缺点是**宿主对插件 UI 的控制力弱**：插件是一张完整的网页，宿主很难统一它的组件、间距、焦点、动效和性能；插件作者用什么组件库、怎么写过渡，完全取决于作者。uTools 的“生硬感”很大一部分来自这里，而不是 Electron 本身。

> 另一个侧面：uTools 仍以 `BrowserView` 承载插件主界面。Electron 新版本已把 `BrowserView` 标记为 deprecated，推荐 `WebContentsView`；这并不代表 uTools 不能好用，但说明它的运行时/窗口模型相对偏旧，升级会牵动插件兼容性。

### 2.2 Raycast：从纯原生到混合架构

Raycast 官方技术博客给出了非常完整的演进：

**v1：**

- Swift + AppKit，自绘绝大多数 UI。
- 几乎不用 SwiftUI；唯一例外是 Wrapped 这种隔离功能。
- 扩展：React + TypeScript + Node.js；扩展用声明式组件描述 UI，由原生 AppKit 渲染。
- 文件搜索依赖 macOS Spotlight 元数据。

**v2：**

- 每个平台一个原生宿主：
  - macOS：Swift + AppKit
  - Windows：**C# + .NET 8 + WPF**
- Web 前端：**一个 React + TypeScript 项目**，按窗口拆 entry（Launcher、AI Chat、Notes、Settings 等），跑在 **WKWebView / WebView2**。
- Node 后端：一个长期运行的 Node 进程，负责数据库、扩展运行时、长生命周期服务。
- Rust core：文件索引器、同步、需要性能/可移植性的数据层。
- 平台消息用 message handler + stdio，接口集中声明并生成各端 typed client。

**为什么不用 Electron / Tauri：**

- Electron：能做的很多，但 Raycast 需要深度 OS 集成（全局热键、剪贴板、辅助功能、窗口管理、不抢焦点的浮动面板），且不想在 macOS 上打包 Chromium；Web 与 native 的边界会带来痛苦。
- Tauri：当时觉得 native 侧控制不够、项目还不够成熟；他们不想把公司押上去。
- 最终选择自研 hybrid：每个平台一个真正的原生工程，系统 WebView 渲染 UI，自己掌控 IPC 和每一层。

**关键判断：**

- Raycast 2.0 的 Windows 端不是 WinUI 3，而是 **WPF 宿主 + WebView2 + React**。这说明“现代好看的 Windows 启动器”不一定非要纯原生 UI。
- 他们明确说：**Web 技术是 UI 的正确选择**，因为大部分代码是 UI，Web 能跨平台复用、生态大、招人容易；但**原生壳必须自己掌控**，否则透明、焦点、热键、窗口层级这些细节会毁掉“native feel”。
- 他们花了大量精力处理 WebView 的节流、白闪、窗口 resize 卡顿、 acrylic 与自定义标题栏的配合。**这些才是“高级感”的来源，不是换一个框架就自动拥有。**

### 2.3 为什么它们会显得“生硬”

**uTools 的原因：**

1. **插件是任意网页**，宿主无法统一动效和组件。你打开不同插件，像打开不同网站，过渡自然不一致。
2. **主壳偏工具化**：搜索框、结果列表、窗口显示/隐藏以功能为先，没有统一的 motion token。
3. **运行时较旧**：Electron 34 / Chromium 132 是 2025 年初的版本；新版本 Chromium 的合成器、CSS 特性、字体渲染优化用不上。
4. **BrowserView 组合方式**：插件页加载、显示、切换的时机由宿主和插件共同决定，容易出现“先白一下再出来”的断裂感；这不是 Electron 的硬限制，但需要宿主做很细的编排。

**Raycast 的原因：**

1. **它是刻意“快”**：启动器最重要的体验是“按下热键，立刻能打字”，不是播放 300ms 的开场动画。Raycast 的动效通常很短、很克制，甚至接近没有，这是设计选择。
2. **原生壳优先**：窗口出现、焦点切换、列表选中都追求零延迟；如果动画拖慢输入，他们宁可不要。
3. **它的“高级感”来自细节**：没有 `cursor: pointer`、没有网页式 hover、弹层用原生窗口、没有白闪、没有错误动画。这些是“像桌面软件”的关键。

**给你的启示：**

> 你想要的“现代好看、每个操作都有过渡”，不是选 Electron 还是 Tauri 能直接解决的。它是一个**设计系统 + 渲染架构 + 性能预算**的问题。  
> 如果插件 UI 仍然是一张任意网页，你依然会得到 uTools 式的参差不齐；如果宿主统一渲染组件和动效，即使是 Electron 也能做出 Raycast 级的连贯感。

---

## 3. 候选框架对比

评分是**针对本项目目标**的主观评分（5 最好），不是框架绝对优劣。

| 方案 | UI/动效上限 | Windows 原生集成 | 常驻内存/体积 | 开发速度 | 插件生态 | 主要风险 | 结论 |
|---|---:|---:|---:|---:|---:|---|---|
| **Electron + React** | 5 | 4 | 2 | 5 | 5 | 内存、安装包、启动 | 最适合快速 MVP |
| **Tauri 2 + React** | 5 | 5 | 4 | 3 | 3 | WebView2 透明/acrylic、Rust 门槛 | 最适合长期产品 |
| **C# WPF/WinUI + WebView2 + React + Node** | 5 | 5 | 4 | 2 | 4 | 四运行时、IPC、构建 | Raycast 路线，规模化后再考虑 |
| **WinUI 3 纯原生** | 3 | 5 | 5 | 2 | 2 | 设计迭代、生态、复杂动效 | 不推荐 v1 |
| **WPF 纯原生** | 3 | 5 | 5 | 3 | 2 | 默认视觉旧、控件自研 | 不推荐 |
| **Avalonia** | 4 | 4 | 5 | 3 | 2 | 生态、Windows 细节 | 跨平台备选 |
| **Flutter** | 4 | 3 | 4 | 3 | 2 | IME、无障碍、原生壳 | 不推荐 Windows 优先 |
| **Qt/QML** | 4 | 5 | 5 | 2 | 2 | 授权、C++/QML 门槛 | 不推荐小团队 |

### 3.1 Electron

**优势：**

- 开发速度最快：主进程 Node + 渲染进程 Chromium，前后端都是 TypeScript。
- 窗口能力强：frameless、transparent、alwaysOnTop、skipTaskbar、多显示器、`ready-to-show` 防白闪、`backgroundMaterial`（Windows 11 Mica/Acrylic）都有成熟 API。
- 插件生态最顺：Node 模块直接可用，`utilityProcess` / `worker_threads` 做隔离，`WebContentsView` 做插件视图。
- 动画生态最成熟：React + Motion、View Transitions、Canvas/WebGL 都能用。
- 自动更新、安装器、崩溃上报、DevTools 都是现成的。
- uTools 已经证明 Electron 可以撑起一个插件平台。

**代价：**

- 安装包通常 80–150 MB，空闲内存 150–300 MB，加载后 300–500 MB（具体看前端和插件）。
- 常驻启动器对内存敏感，低配办公本上会被用户感知。
- 需要自己做性能纪律：懒加载、单 renderer、禁用不必要模块、避免阻塞主进程。
- Chromium 版本跟随 Electron，升级节奏比系统 WebView 快，但也意味着你要跟版本。

**适合：** 前端背景的小团队、要快速验证、插件生态优先、视觉概念依赖复杂透明窗口。

### 3.2 Tauri 2

**优势：**

- 不打包 Chromium，安装包小、空闲内存通常更低；对常驻启动器很友好。
- Rust 后端可以直接做 Win32 / COM / DWM / 文件系统 / 索引器，性能和控制力强。
- 权限模型（capabilities）比 Electron 默认更细，适合插件平台。
- 官方插件覆盖：global-shortcut、autostart、single-instance、updater、store、clipboard、opener、window-state、sql 等。
- 前端完全自由：React/Vue/Svelte/Solid 都行；Windows 上是 WebView2（Chromium），动效能力和 Electron 基本一致。
- 未来加 macOS/Linux 时，Rust core 可复用；UI 也可复用。

**代价：**

- Rust 学习曲线；真正做 Win32 细节时，绕不开 `windows` crate 和不安全代码。
- WebView2 版本随系统更新，行为有差异；Windows 10 需要确保 Runtime 存在（Win11 一般自带）。
- **透明窗口 + Acrylic/Mica 是最大工程风险**：需要处理 WebView2 背景色、DWM backdrop、窗口圆角、阴影、无白闪；不同 Windows 版本/显卡/缩放比例都可能出问题。
- 没有内置 Node：要做 uTools 式 Node 插件，需要 sidecar（会抵消一部分体积优势），或换 QuickJS/Deno core/WASM。
- 生态和排错资料比 Electron 少；遇到冷门 Windows 问题，很多时候要自己读源码/提 issue。
- 全局热键、托盘、更新这些官方插件能用，但“双击 Ctrl 唤起”“不抢焦点浮窗”“多显示器定位”等细节仍要自己写。

**适合：** 有 Rust 能力或愿意投入 2–4 周原生层、在意内存/启动速度、希望长期做平台的产品。

### 3.3 自研 C# + WebView2 + React + Node

Raycast 2.0 的同款思路。优势是原生壳完全可控、Web UI 复用、Node 插件生态成熟；代价是：

- 4 个运行时：C# 宿主、WebView2、Node、Rust（可选）。
- 要自己设计 IPC、typed client、进程生命周期、崩溃恢复。
- 构建/调试/发版链路明显更复杂。
- 对 1–3 人团队，v1 会被基础设施拖慢。

**建议：** 等产品验证后，如果确实需要 Raycast 级原生控制，再逐步把热点模块下沉到 C#/Rust；不要一开始就上四层。

### 3.4 WinUI 3 / WPF 纯原生

- WinUI 3 是微软当前推荐的原生 Windows UI 框架，Fluent Design、性能、高 DPI、输入支持都不错；但 XAML 的自定义控件、复杂转场、设计稿还原速度，和 Web 技术不在一个量级。WinUI 3 仍相对年轻，第三方控件/社区方案不如 Web。
- WPF 更成熟，但默认视觉是 2006 年的风格；要做现代感，需要大量自定义 ControlTemplate、动画和窗口特效，等于自研一套 UI 框架。
- 纯原生最大的优势是内存/启动/系统集成；如果你只做“极简、极快、零动效”的启动器，这是合理路线。但你的目标是“现代好看、每个操作有过渡”，纯原生会显著拉长设计迭代周期。

### 3.5 Flutter / Avalonia / Qt

- **Flutter**：动画和自绘能力很强，但 Windows 桌面端的 IME、无障碍、原生菜单/托盘/窗口层级需要插件或 platform channel；UI 默认是 Material/Cupertino，不像 Windows 原生应用，需要自己设计一套 Fluent 风格组件。
- **Avalonia**：C# + XAML，跨平台，Skia 渲染，动画能力不错；但生态比 WinUI/WPF 小，Windows 特有的 Mica/Acrylic、任务栏、UWP 应用枚举仍要自己补。
- **Qt/QML**：性能好、QML 做动效自然，但 C++/QML 双栈、授权（LGPL/商业）、招聘面、包体都不适合小团队快速迭代。

---

## 4. 推荐架构（Tauri 版）

```text
┌─────────────────────────────────────────────────────────┐
│ React 19 + TypeScript + Vite + Tailwind + Motion        │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ Command UI   │  │ Design System│  │ Plugin UI     │  │
│  │ Search/List  │  │ Tokens +     │  │ Renderer      │  │
│  │ Detail/Action│  │ Motion       │  │ (declarative) │  │
│  └──────────────┘  └──────────────┘  └───────────────┘  │
│            │               │                  │         │
│            └───────────────┴──────────────────┘         │
│                         Tauri IPC (typed)               │
└────────────────────────────┬────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────┐
│ Rust Core                                               │
│                                                         │
│  Win32 Shell        Search/Index        Platform        │
│  ├─ global hotkey   ├─ Start Menu apps  ├─ tray         │
│  ├─ window show/hide├─ UWP apps         ├─ autostart    │
│  ├─ DWM Mica/Acrylic├─ files (USN/MFT)  ├─ updater      │
│  ├─ monitor/DPI     ├─ SQLite FTS5      ├─ clipboard    │
│  └─ foreground app  └─ ranking           └─ single inst. │
│                                                         │
│  Plugin Host (phase 2): Node sidecar / QuickJS / WASM   │
└─────────────────────────────────────────────────────────┘
```

### 4.1 关键设计原则

1. **UI 与平台适配层分离**：React 不直接调 `invoke` 散落各处，而是通过 `PlatformAdapter` 接口；以后换 Electron 或加 macOS，只换 adapter。
2. **搜索/索引不阻塞 UI**：Rust 侧用独立线程/进程 + channel 流式返回；前端用虚拟列表，先显示前 20 条，再增量更新。
3. **窗口显示/隐藏由 Rust 编排**：热键触发 → 预渲染 → 定位到鼠标所在显示器 → `show` + `focus`；隐藏时恢复上一个前台窗口。不要等前端 `useEffect` 再显示。
4. **插件 UI 宿主化**：插件只返回声明式组件树/JSON，宿主用统一组件渲染；这样每个插件的过渡、焦点、间距、快捷键都一致。**这是你超越 uTools 的关键。**
5. **插件逻辑隔离**：Node sidecar 或 QuickJS，每个插件独立 worker/进程，限制内存/文件/网络权限；不要让插件直接拿到完整 Node。
6. **崩溃隔离**：索引器、插件宿主、UI 分进程/线程；插件崩溃不能拖垮启动器。

### 4.2 插件模型建议

| 模型 | 一致性 | 能力 | 安全 | 开发体验 | 建议 |
|---|---|---|---|---|---|
| 任意 HTML 页面（uTools 式） | 低 | 最高 | 低 | 最简单 | 只作为高级逃生口 |
| 声明式组件 + 宿主渲染（Raycast 式） | 高 | 中 | 中 | 好 | **默认方案** |
| 纯 JSON UI Schema | 最高 | 低 | 高 | 一般 | 适合简单命令 |
| WASM 插件 | 高 | 中 | 高 | 较差 | 后期可选 |

**推荐路线：**

- v1：只做内置命令，不开放插件。先把启动器核心体验做到 90 分。
- v1.5：开放“声明式 UI SDK”（List、Detail、Form、ActionPanel、Toast），插件逻辑跑在沙箱 JS runtime；宿主统一渲染和动画。
- v2：如果需要复杂插件，再提供受控的 WebView 逃生口，但明确标记“可能破坏统一动效”，并做性能/安全审核。

---

## 5. 动效与视觉设计规范

### 5.1 原则

1. **快是最高级**：热键到可输入 < 100ms；按键到视觉反馈 < 16ms；搜索结果首屏 < 50ms。动画不能增加等待。
2. **动效解释因果关系**：面板从哪来、到哪去、为什么出现；不要为了“炫”而加。
3. **短、轻、可打断**：启动器动效以 120–200ms 为主；用户继续输入时，动画要能被立刻打断。
4. **只动 transform / opacity**：不要动画 `width/height/top/left/backdrop-filter`；这些会触发布局/重绘，容易掉帧。
5. **一致性**：所有列表、面板、Toast 用同一套 duration/easing/spring；插件也必须用宿主组件。
6. **尊重 reduced motion**：`prefers-reduced-motion` 下改为淡入/直接切换。

### 5.2 建议 token

| Token | 值 | 用途 |
|---|---|---|
| `motion.instant` | 90ms | 按下、hover、焦点 |
| `motion.fast` | 140ms | 列表选中、Toast、Tooltip |
| `motion.normal` | 180ms | 面板展开、push/pop |
| `motion.slow` | 240ms | 窗口/大面积转场（尽量少用） |
| `ease.out` | `cubic-bezier(0.16, 1, 0.3, 1)` | 进入 |
| `ease.in` | `cubic-bezier(0.7, 0, 0.84, 0)` | 退出 |
| `spring.selection` | stiffness 500 / damping 35 | 选中框移动 |
| `spring.panel` | stiffness 380 / damping 30 | 面板展开 |

### 5.3 交互模式

| 交互 | 建议 |
|---|---|
| 窗口出现 | opacity 0→1 + scale 0.98→1 + translateY 6→0，140ms；内容 stagger 20ms；不要 300ms 弹跳 |
| 窗口关闭 | 反向 100ms；先让焦点回到上一个应用，再淡出 |
| 列表选中 | 用 shared layout 的选中背景滑动，120ms spring；文字/图标不跟着动 |
| 进入详情 | 共享元素（图标/标题）+ 内容 crossfade，180ms；返回 160ms |
| Action Panel | 从底部/右侧 8–12px 滑入 + 淡入，140ms；背景只做透明度，不做 blur 动画 |
| Toast | 8px 上滑 + 淡入，160ms；2s 后自动消失；可点击撤销 |
| 搜索加载 | < 300ms 不显示 loading；超过则骨架屏，避免 spinner 闪烁 |
| 错误 | 轻微 shake（±2px，120ms）或红色边框；不要大面积红屏 |

### 5.4 视觉建议

- **材质**：Windows 11 用 DWM Mica（主窗口）/ Acrylic（浮层）；Windows 10 用 Acrylic 或纯色 + 细噪点。**不要用 CSS `backdrop-filter` 大面积模糊**，性能差且和 DWM 材质叠加会脏。
- **圆角**：主窗口 12–16px，浮层 10–14px；用 DWM 圆角优先，透明窗口再考虑 CSS 圆角。
- **边框/阴影**：1px 内描边 `rgba(255,255,255,.08)`；阴影要软、低对比，不要网页式大黑阴影。
- **字体**：拉丁用 `Segoe UI Variable` / `Inter`；中文用 `Microsoft YaHei UI`，可评估 MiSans/HarmonyOS Sans（注意授权）。正文 13–14px，行高 1.45–1.55。
- **密度**：列表行高 36–40px；左右 padding 12–16px；图标 18–20px；不要做成“网页卡片大全”。
- **键盘优先**：上下选择、Enter 执行、Tab 打开 Action Panel、Esc 返回/关闭；底部显示快捷键提示；焦点环必须可见。
- **去掉网页痕迹**：禁用文本选择、禁用浏览器右键菜单、隐藏滚动条、列表行不用 `cursor: pointer`（Raycast 的经验）、不要有 300ms 的页面路由动画。

### 5.5 不要做的事

- 不要给每个 hover 都加 200ms 动画；高频操作会显得黏。
- 不要用弹跳/回弹做生产力工具的主转场；会显得玩具化。
- 不要动画 `backdrop-filter`、`box-shadow`、`width/height`。
- 不要让窗口开场动画挡住输入；用户按键时立即跳过动画。
- 不要让插件自带 CSS 决定全局动效；否则永远不统一。
- 不要为了“每个操作都有过渡”而给“输入字符”加动画；输入必须即时。

---

## 6. 两周技术验证清单（Go / No-Go）

在写业务代码前，用 Tauri 做一个最小原型，只验证风险点。每项给 pass/fail。

| # | 验证项 | 通过标准 |
|---|---|---|
| 1 | 全局热键唤起 | 按热键到输入框可打字 < 100ms；重复按不会叠加窗口 |
| 2 | 无白闪 | 冷启动/热唤起都看不到白底/透明闪烁 |
| 3 | 中文 IME | 微软拼音、搜狗、微信输入法都能正常候选/上屏；焦点不丢 |
| 4 | Mica/Acrylic | Win11 上材质正确、圆角正确、拖动/resize 不闪；Win10 有可接受 fallback |
| 5 | 透明/浮层 | 浮层能超出主窗口、不抢焦点、不进入任务栏；阴影/圆角正常 |
| 6 | 多显示器/DPI | 在 100%/125%/150%/200% 缩放、双屏不同 DPI 下位置正确 |
| 7 | 应用枚举/启动 | Win32 快捷方式、UWP/Store 应用都能列出并启动；图标正确 |
| 8 | 文件搜索 | 10 万条索引可搜；首屏 < 50ms；索引时 UI 不卡 |
| 9 | 剪贴板历史 | 文本/图片记录、去重、隐私排除、快捷粘贴可用 |
| 10 | 空闲内存 | Tauri 目标 < 150MB；若 > 250MB 且无法优化，重新评估 |
| 11 | 安装/更新/签名 | NSIS/MSI 安装、自动更新、代码签名、SmartScreen 流程跑通 |
| 12 | 120Hz 动画 | 选中框/面板动画在 120Hz 屏上不掉帧；60Hz 也稳定 |

**判定规则：**

- 1、2、3、4 任意一项失败，且 3 天内无法解决 → **换 Electron**。
- 1–4 通过，第 8、10 项达标 → **Tauri 继续**。
- 如果插件 Node 生态是第一优先级，直接用 Electron，不要为了“轻”牺牲产品节奏。

---

## 7. 建议路线图

### Phase 0：技术验证（2 周）

- Tauri 最小窗口 + 热键 + 透明/Mica + 中文 IME + 列表动画。
- 枚举开始菜单应用 + 启动。
- 用 Everything SDK 或 USN/MFT 做一版文件搜索。
- 产出：性能数据、录屏、Go/No-Go 结论。

### Phase 1：核心启动器（4–6 周）

- 应用搜索/启动、文件搜索、计算器、剪贴板历史、常用命令。
- 托盘、开机自启、单实例、自动更新、设置页。
- 设计系统 v1：颜色/字体/间距/圆角/阴影/动效 token。
- 键盘导航、焦点管理、无障碍基础。

### Phase 2：体验打磨（4–6 周）

- 窗口出现/关闭、列表选中、详情 push/pop、Action Panel、Toast 全链路动效。
- 命令面板、预览、快速操作、拖拽、多选。
- 性能优化：虚拟列表、懒加载、图标缓存、索引增量更新。
- 真实用户测试：低配机、Win10、双屏、中文输入法。

### Phase 3：插件与平台（8 周+）

- 声明式插件 UI SDK + 沙箱 JS runtime。
- 插件市场/审核/权限/崩溃隔离。
- AI 命令、窗口管理、同步、团队配置。
- 这时再评估是否要把部分模块下沉到 C#/Rust 或做 macOS。

---

## 8. 参考来源

1. uTools 官方文档《插件应用架构》  
   https://www.u-tools.cn/docs/development/plugin-architecture.html
2. uTools 官方文档《自定义窗口》  
   https://www.u-tools.cn/docs/development/custom-window.html
3. Raycast Blog: *A Technical Deep Dive Into the New Raycast*（2026-05-14）  
   https://www.raycast.com/blog/a-technical-deep-dive-into-the-new-raycast
4. Raycast Blog: *Raycast for Windows*（2025-11-20）  
   https://www.raycast.com/blog/raycast-for-windows
5. Raycast Blog: *How the Raycast API and extensions work*（2023-05-31）  
   https://www.raycast.com/blog/how-raycast-api-extensions-work
6. Tauri 官方文档《Architecture》  
   https://v2.tauri.app/concept/architecture/
7. Tauri 官方文档《Global Shortcut》  
   https://v2.tauri.app/plugin/global-shortcut/
8. Tauri Core Releases（2.12.3，2026-10-10）  
   https://tauri.app/release/core/
9. `window-vibrancy` 0.8.0 文档（Acrylic/Mica/Blur）  
   https://docs.rs/window-vibrancy/0.8.0/window_vibrancy/
10. Microsoft Learn: `DWM_SYSTEMBACKDROP_TYPE`（Mica/Acrylic，Windows 11 Build 22621+）  
    https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type
11. Electron BrowserWindow 文档（`ready-to-show`、transparent、backgroundThrottling）  
    https://www.electronjs.org/docs/latest/api/browser-window
12. Electron Releases Schedule（Electron 45 stable，2026-10-20）  
    https://releases.electronjs.org/schedule
13. Microsoft Learn: WinUI 3  
    https://learn.microsoft.com/en-us/windows/apps/winui/winui3/
14. Flutter 官方文档: Building Windows apps  
    https://docs.flutter.dev/platform-integration/windows/building
15. Avalonia 官方文档  
    https://docs.avaloniaui.net/
16. Qt 官方文档: QML Applications  
    https://doc.qt.io/qt-6/qmlapplications.html

---

## 9. 最终一句话

**如果你有 2–4 周做原生层、并且在意“常驻但不臃肿”，选 Tauri 2 + React；如果你是一个人、要最快验证、插件 Node 生态优先，选 Electron + React。**  
但无论选哪个，**先把“宿主统一渲染 + 动效 token + 无白闪 + 输入零延迟”做成架构约束**，否则换框架也只会得到另一个“有点生硬”的 uTools。

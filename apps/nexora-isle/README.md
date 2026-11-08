<div align="center">

<img src="assets/nexora-isle-logo.png" width="112" alt="NEXORA ISLE" />

# NEXORA ISLE · 星屿

**桌面边缘，一眼即达。**

NEXORA ISLE 是一款独立的 Windows 桌面信息岛屿。它把音乐、消息提醒、邮件、文件和系统状态收束到屏幕边缘，不依赖 NEXORA 全局搜索应用。

</div>

## 产品边界

- 独立可执行文件：`nexora-isle.exe`
- 独立设置窗口、托盘、数据目录和安装包
- 不读取、不修改、也不要求安装 NEXORA 启动器
- 未来可选择由 NEXORA 管理启停，但当前版本没有运行时依赖

## 功能

### 桌面信息岛

- 刘海与胶囊两种形态
- 默认待机刘海
- 顶部、左侧和右侧停靠
- 多显示器定位、大小与位置调整
- 全屏隐藏、浏览器避让和指定应用避让
- “岛屿”动态眼神、极简模式、页面显隐与顺序调整

### 音乐

- Windows 媒体会话播放控制
- 播放进度、音量和切歌
- 同步歌词、文本歌词与岛屿单行歌词
- 网易云、QQ 音乐、酷狗和汽水音乐兼容

### 浏览器视频

- 捕获浏览器后台正在播放的视频会话
- 独立播放器卡片显示标题、来源、进度和封面
- 点击卡片返回原浏览器窗口继续播放
- 没有视频会话时自动隐藏“视频”分类

### 消息与系统状态

- 微信任务栏闪动提醒，多账号分别识别
- 微信与 Windows 系统通知统一设置
- 电量、电源、网络、音量、大写锁定、输入语言和蓝牙设备状态

### 系统监控

- CPU、内存、磁盘、GPU 与进程数量实时采样
- 1 分钟 / 5 分钟历史曲线与 CPU / 内存 Top 应用排行
- CPU、内存、GPU 阈值警报，持续 5 秒触发并带 60 秒冷却
- 最近 200 条监控事件保存在本机

### 文件与剪贴板

- 文件口袋：记录文件位置，不复制文件正文
- 文本剪贴板：保存最近复制的文本并支持固定、删除和清空
- 图片剪贴板：截屏或复制图片后本地保存，横向缩略图、悬浮预览、点击打开

### 邮箱

- QQ、163、126、腾讯企业邮、iCloud、Gmail 和自定义 IMAP
- 验证码识别、隐私遮罩和点击复制
- 邮件正文阅读和网页链接打开

## 明确不包含

NEXORA ISLE 不包含以下功能或相关后台服务：

- 专注/番茄钟
- 饮水记录与提醒
- AI 额度监控
- 日历、CalDAV、Exchange、`.ics` 导入和会议提醒

## 系统要求

- Windows 10 / Windows 11 x64
- OpenGL 或 Direct3D 12
- 从源码构建需要 Rust stable、MSVC C++ Build Tools 和 Windows SDK

## 开发与运行

```powershell
cargo check --all-targets
cargo test --all-targets -- --test-threads=1
cargo build --bin nexora-isle
.\target\debug\nexora-isle.exe
```

运行后右键单击系统托盘中的 NEXORA ISLE 图标，选择“设置…”。开发时也可以直接打开设置窗口：

```powershell
.\target\debug\nexora-isle.exe --settings
```

## 发布打包

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\package.ps1
```

脚本生成：

- `dist/NEXORA_ISLE_<版本>_x64_setup.exe`
- `dist/NEXORA_ISLE_<版本>_portable.zip`

## 本地数据

默认目录：

```text
%APPDATA%\NEXORA\Isle
```

测试或便携运行时可通过 `NEXORA_ISLE_DATA_DIR` 指定其他目录。

邮箱密码保存在 Windows 凭据管理器。独立会话和敏感缓存使用当前 Windows 用户的 DPAPI 保护。

## 代码组织

| 目录 | 职责 |
| --- | --- |
| `src/app.rs` | 应用状态、外层形变、四页路由与顶栏 |
| `src/pages.rs` | 音乐、文件口袋、剪贴板、邮箱页面 |
| `src/extras.rs` | 通知、设置窗口与通用设置控件 |
| `src/product/` | 产品身份、数据路径、更新仓库和机器标识 |
| `src/platform` / Windows 模块 | Win32 窗口、通知、蓝牙、媒体与系统状态 |
| `src/mail*.rs` | IMAP、MIME、验证码、隐私和全文阅读 |
| `installer/` | NSIS 安装、升级和卸载 |

## 许可

NEXORA ISLE 的修改代码与第三方组件许可文本保留在 `LICENSE`、`THIRD_PARTY_NOTICES.txt` 与 `docs/DEPENDENCIES.md`。

MIT 许可不授予任何既有名称、Logo 或其他商标使用权。NEXORA ISLE 使用独立品牌资源。

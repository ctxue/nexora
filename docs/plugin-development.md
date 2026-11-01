# NEXORA 插件开发指南（v1 / MVP）

> 状态：进程外插件 MVP。宿主负责关键词、搜索、渲染与动作执行；插件只做数据。
> 设计背景见 [plugin-architecture-design.md](plugin-architecture-design.md)。

---

## 1. 插件是什么

一个插件就是一个文件夹：

```
com.example.ip/
  plugin.json     # 清单（必需）
  plugin.ps1      # 入口脚本或 exe
  icon.png        # 可选
```

宿主启动插件进程，通过 **stdin/stdout 的 JSON-RPC 2.0（一行一个 JSON）** 通信。插件返回声明式结果，宿主负责显示；插件不接触 UI，也不直接操作窗口。

## 2. 安装

1. 打开 `设置 → 插件 → 安装插件`；
2. 选择包含 `plugin.json` 的文件夹；
3. 插件会被复制到 `%APPDATA%\com.nexora.desktop\plugins\<id>\`；
4. 在启动器输入插件关键词 + 空格 + 参数，例如 `ip 8.8.8.8`。

安装示例插件：选择本仓库的 `examples/plugins/ip` 文件夹。

## 3. plugin.json

```json
{
  "id": "com.nexora.samples.ip",
  "name": "IP 查询",
  "version": "1.0.0",
  "author": "NEXORA",
  "description": "输入 ip 8.8.8.8，回车打开归属地页面",
  "entry": "powershell.exe",
  "args": ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "plugin.ps1"],
  "keywords": ["ip"],
  "permissions": ["network"],
  "timeoutMs": 3000
}
```

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `id` | ✅ | 反向域名，3–64 位 `a-z 0-9 . - _`，全局唯一 |
| `name` | ✅ | 显示名称（也是结果分组标题） |
| `version` | ✅ | 语义化版本 |
| `entry` | ✅ | 可执行文件（插件目录内相对路径）或 PATH 中的命令 |
| `args` | | 传给入口的参数数组 |
| `keywords` | | 触发关键词；`ip 8.8.8.8` 会把 `8.8.8.8` 作为 query 传入 |
| `permissions` | | `network` / `filesystem` / `clipboard` / `shell` / `notifications`，**当前仅展示，不强制** |
| `timeoutMs` | | 单次调用超时，默认 3000，最大 15000 |

## 4. 协议

### 宿主 → 插件

```json
{"jsonrpc":"2.0","id":1,"method":"search","params":{"query":"8.8.8.8","keyword":"ip"}}
```

- 一行一个 JSON，UTF-8；
- `id` 由宿主递增，响应必须带回相同的 `id`；
- 除 JSON 外**不要往 stdout 打印任何东西**（日志走 stderr）。

### 插件 → 宿主

```json
{"jsonrpc":"2.0","id":1,"result":{"items":[
  {
    "id":"ip:8.8.8.8",
    "title":"8.8.8.8",
    "subtitle":"Open this address on ipinfo.io",
    "icon":{"type":"glyph","value":"globe"},
    "action":{"type":"openUrl","url":"https://ipinfo.io/8.8.8.8"}
  }
]}}
```

结果 item：

| 字段 | 说明 |
| --- | --- |
| `id` | 插件内唯一 |
| `title` / `subtitle` | 列表显示 |
| `icon` | 可选：`{"type":"glyph","value":"globe"}`；`globe/search/link/folder/terminal/...` |
| `action` | 回车时执行的动作（见下） |

支持的动作：

| action | 说明 |
| --- | --- |
| `{"type":"openUrl","url":"https://…"}` | 打开网页（仅 http/https） |
| `{"type":"openPath","path":"C:\\…"}` | 打开文件/文件夹 |

没有 `action` 的 item 只展示，回车无操作。

错误响应：

```json
{"jsonrpc":"2.0","id":1,"error":{"code":1,"message":"Something went wrong"}}
```

## 5. 生命周期

- **懒启动**：第一次命中关键词时启动插件进程；
- **常驻**：进程保持运行，后续每次输入直接复用；
- **超时**：单次 `search` 超时（默认 3s）会杀掉进程，下次输入时重建；
- **退出**：启动器退出时统一结束所有插件进程；
- **隔离**：插件进程崩溃不影响启动器。

## 6. 示例

`examples/plugins/ip/plugin.ps1`（PowerShell）核心：

```powershell
$line = [Console]::In.ReadLine()
$message = $line | ConvertFrom-Json
$query = [string]$message.params.query

$item = [ordered]@{
  id = "ip:$query"
  title = $query
  subtitle = "Open this address on ipinfo.io"
  icon = @{ type = "glyph"; value = "globe" }
  action = @{ type = "openUrl"; url = "https://ipinfo.io/$query" }
}

$itemJson = $item | ConvertTo-Json -Compress -Depth 6
$response = '{"jsonrpc":"2.0","id":' + $message.id + ',"result":{"items":[' + $itemJson + ']}}'
[Console]::Out.WriteLine($response)
[Console]::Out.Flush()
```

手动验证协议（不经过启动器）：

```powershell
'{"jsonrpc":"2.0","id":1,"method":"search","params":{"query":"8.8.8.8"}}' |
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File examples\plugins\ip\plugin.ps1
```

## 7. 当前限制（v1）

- 只支持 `search`：没有 `execute` 回调和宿主 API（剪贴板、存储、HTTP 代理）；
- 不支持正则/文件/选中文本匹配，只支持关键词；
- 不支持自定义 UI（结果由宿主渲染）；
- 没有签名、自动更新和插件市场；
- `permissions` 只用于展示，进程本身仍拥有当前用户权限——**只安装可信来源的插件**；
- 插件如果自己派生子进程，宿主目前不会递归回收（后续用 Job Object 处理）。

## 8. 调试

- 插件 stderr 会打印到启动器的 stderr（`eprintln!`），可用 `pnpm tauri dev` 查看；
- 宿主对非 JSON 行会跳过，但不要依赖这一点；
- 输入超时/崩溃后插件会被杀掉，下次查询自动重启；
- 插件目录：`设置 → 插件 → 打开目录`。

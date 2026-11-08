# NEXORA Product Workspace

## 边界

根目录是工作区，不是某个产品的源码目录。

- `apps/nexora-launcher` 是 NEXORA 启动器产品。
- `apps/nexora-isle` 是 NEXORA ISLE · 岛屿产品。
- `products.psd1` 是产品注册表。
- `scripts/products.ps1` 是唯一的产品命令分发器。
- 根 `release/` 是联合发布暂存目录。

## 产品接口

每个产品必须提供以下脚本：

| 动作 | 脚本 |
| --- | --- |
| 开发运行 | `scripts/dev.ps1` |
| 调试构建 | `scripts/build-debug.ps1` |
| 测试 | `scripts/test.ps1` |
| 打包 | `scripts/package.ps1`（Isle）或 `scripts/build-release.ps1`（启动器） |

打包脚本必须接受 `-OutputDirectory`，并只写入两个文件：一个安装包和一个便携 ZIP。

## 新增产品

1. 创建 `apps/<product-id>`。
2. 在目录内实现上述四个脚本。
3. 在 `products.psd1` 增加产品条目。
4. 在根 `package.json` 增加可选的直达命令。
5. 为 CI 增加独立工作流和路径过滤。
6. 运行 `pnpm test:workspace`，确认产品分发器能够自动调用新条目。

根分发器不维护产品白名单，产品数量由清单决定。

## 数据与安装隔离

每个产品使用自己的安装目录、单实例标识、更新缓存和用户数据。NEXORA ISLE 保留从早期版本数据和凭据迁移的兼容入口；产品改名不能导致设置、邮箱授权码和历史记录丢失。

## 发布约束

联合打包会先清理根 `release/`，然后依次执行清单中的所有产品打包脚本，并验证每个产品恰好产生两个文件。当前两个产品联合发布必须得到四个文件，不生成额外校验文本；GitHub Release 的资产摘要由平台提供。

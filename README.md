<div align="center">
  <img src="ui/assets/app-icon.svg" width="88" height="88" alt="MTool" />
  <h1>MTool</h1>
  <p>开发工具 · 剪贴板 · Finder 扩展 · 鼠标增强</p>
  <p><strong>macOS · Windows</strong></p>
  <p>
    <code>Rust</code> · <code>Tauri 2</code> · <code>Vue 3</code> · <code>TypeScript</code> · <code>Tailwind CSS</code>
  </p>
</div>

---

## 功能

| 模块        | 功能                                                 |
| ----------- | ---------------------------------------------------- |
| 启动台      | 搜索、收藏、最近使用                                 |
| 工具箱      | 12 项离线工具、独立工作区                            |
| 剪贴板      | 文本与图片历史、搜索、收藏、滚动加载、快捷浮窗       |
| 历史管理    | 暂停记录、应用排除、容量限制、过期清理               |
| 桌面集成    | 全局快捷键、托盘、登录启动                           |
| 主题        | Dracula / GitHub Light                               |
| Finder 扩展 | 新增文件、文件名编辑、使用 VS Code 打开；仅 macOS    |
| 鼠标增强    | 平滑滚动、按键映射、空间手势、罗技硬件 DPI；仅 macOS |

### 工具

| 分类       | 工具                                 |
| ---------- | ------------------------------------ |
| 编解码     | Base64、URL、Unicode、Hex / String   |
| 数据与文本 | JSON、文本处理、文本对比、正则表达式 |
| 时间与生成 | 时间戳、UUID / 随机字符串            |
| 哈希与令牌 | 哈希 / HMAC、JWT 解码（不验签）      |

## 开发

### 环境

| 依赖    | 版本 / 组件                                                           |
| ------- | --------------------------------------------------------------------- |
| Node.js | 22.18+                                                                |
| pnpm    | 10.28.2                                                               |
| Rust    | 1.99.1                                                                |
| macOS   | 13+ · Apple Silicon / Intel · Xcode Command Line Tools                |
| Windows | 10 22H2 / 11 · x64 · Visual Studio C++ Build Tools、Windows SDK、Perl |
| Make    | GNU Make                                                              |

### 启动

```bash
pnpm install --frozen-lockfile
pnpm desktop
```

### 命令

| 命令                 | 用途                                         |
| -------------------- | -------------------------------------------- |
| `pnpm dev`           | 界面预览 · `http://127.0.0.1:1420`           |
| `pnpm bindings`      | 生成 `ui/bindings/` 类型、默认设置与工具目录 |
| `pnpm signing:setup` | macOS 本地开发签名 · 一次配置                |
| `make help`          | 命令索引                                     |
| `make fmt`           | 格式化                                       |
| `make check`         | 格式、类型、Clippy 检查                      |
| `make build`         | 界面与 Rust 构建                             |
| `make clean`         | 构建产物清理                                 |

## 快捷键

| 操作       | macOS     | Windows        | 范围       |
| ---------- | --------- | -------------- | ---------- |
| 搜索启动台 | `⌥ Space` | `Alt Space`    | 全局       |
| 剪贴板浮窗 | `⌘ ⇧ C`   | `Ctrl Shift C` | 全局       |
| 搜索工具   | `⌘ K`     | `Ctrl K`       | 主窗口     |
| 运行工具   | `⌘ ↵`     | `Ctrl Enter`   | 工具工作区 |
| 选择记录   | `↑` / `↓` | `↑` / `↓`      | 剪贴板浮窗 |
| 复制并关闭 | `↵`       | `Enter`        | 剪贴板浮窗 |
| 关闭浮窗   | `Esc`     | `Esc`          | 剪贴板浮窗 |

## 数据与权限

### 鼠标增强 · macOS

| 项目     | 配置                                                     |
| -------- | -------------------------------------------------------- |
| 滚轮     | 平滑滚动、横纵向反转                                     |
| 滚动速度 | 0.25–4 倍                                                |
| 平滑时长 | 80–400 ms                                                |
| 按键     | 中键、按键 4–32、罗技 MX Master 额外按键                 |
| 映射动作 | 前进、后退、调度中心与空间手势、显示桌面、快速查看、禁用 |
| 按键识别 | 按键编号检测                                             |
| 硬件 DPI | 罗技 HID++ DPI 调节、档位识别、唤醒恢复                  |
| 空间手势 | 左右切换空间、上移调度中心、下移应用窗口                 |
| 设置     | 自动保存、即时生效                                       |

### 存储与系统权限

| 项目                    | 配置                                  |
| ----------------------- | ------------------------------------- |
| 数据库                  | SQLCipher · `mtool.db`                |
| 密钥                    | 本地文件 · `mtool.key`                |
| 普通历史                | 默认 1,000 条 · 30 天                 |
| 内容容量                | 默认 512 MiB · 含收藏                 |
| 工具输入 / HMAC 密钥    | 进程内存                              |
| Finder 菜单             | 打包应用 · 启用「MTool 右键菜单」扩展 |
| Finder 新增文件与重命名 | macOS 辅助功能权限                    |
| 鼠标增强                | macOS 辅助功能权限                    |
| 罗技额外按键            | macOS 输入监控权限                    |
| VS Code 打开            | 已安装 VS Code                        |

## 打包

| 平台    | 命令               | 产物            |
| ------- | ------------------ | --------------- |
| macOS   | `make package-app` | `.app`          |
| macOS   | `make package-dmg` | `.app` / `.dmg` |
| Windows | `make package-exe` | NSIS `.exe`     |
| Windows | `make package-msi` | `.msi`          |

| 参数             | 值                                                    |
| ---------------- | ----------------------------------------------------- |
| 通用入口         | `make package` / `pnpm package`                       |
| Make 参数        | `TARGET=<Rust target>` · `BUNDLES=<app,dmg,nsis,msi>` |
| 默认产物目录     | `target/release/bundle/`                              |
| 指定目标产物目录 | `target/<TARGET>/release/bundle/`                     |
| macOS 本地签名   | 开发证书 / 临时签名                                   |
| Windows 签名     | 默认未签名                                            |

## 许可证

[MIT](LICENSE)

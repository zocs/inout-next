# inout

独立实现的 Rust HTTP 文件服务、静态 Next.js 文件管理页面和 Tauri 桌面宿主。一台设备选择共享目录，其他设备通过浏览器访问。品牌配置、存储提供者、功能模块和原生生命周期分开设计，便于替换名称或扩展平台。应用显示名为 **inout**，仓库及分发 slug 保留 **inout-next**。

目前是实验性 **0.1.0 preview**。可[下载桌面预发布版本](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.1)，产物选择见[预览运行说明](docs/distribution/preview.md)。[CI](https://github.com/zocs/inout-next/actions) 覆盖 Linux x64、Windows x86/x64、macOS ARM64/x64 的构建与测试，包含真实浏览器文件操作和原生打包。安装、签名、离线分发和移动平台分别验收，具体见[平台状态](docs/platforms.md)。接下来优先推进 Android arm64 构建；iOS 适配和实机验证安排在后续。尚无稳定正式版本。

- [English README](README.md)
- [需求](docs/requirements.md) · [路线图](ROADMAP.md) · [平台状态](docs/platforms.md)
- [架构](docs/architecture.md) · [API v1](contracts/api-v1.md) · [模块](docs/modules.md)
- [品牌与身份](docs/branding.md) · [设计系统](DESIGN.md) · [包体积与运行依赖](docs/distribution/size-and-runtime.md)
- [默认分享地址与二维码规划](docs/network-selection.md)
- [应用商店前期准备](docs/distribution/store-readiness.md) · [隐私数据清单](docs/distribution/privacy-data-inventory.md)
- [签名复用与应用身份](docs/distribution/signing.md)

## 启动文件服务

构建固定使用 Rust 1.99.1、Node 26.13.1；Python 脚本用于配置生成、打包和验证。运行已打包的 CLI 文件服务及静态网页不需要 Node 或 Python。

```sh
npm ci --prefix apps/web
npm --prefix apps/web run build
cargo run -p share-cli --locked -- \
  --root /path/to/shared-folder --web-dir apps/web/out \
  --listen 127.0.0.1:5000
```

把 `/path/to/shared-folder` 换成已有目录，再打开 `http://127.0.0.1:5000`。共享目录必须与网页资源目录分开。默认只读，Ctrl-C 停止服务。

需要上传和管理文件时，在共享目录外生成密钥文件，读取后填入浏览器：

```sh
python3 scripts/create_key.py ./access.key
cargo run -p share-cli --locked -- \
  --root /path/to/shared-folder --web-dir apps/web/out \
  --listen 0.0.0.0:5000 --writable --key-file ./access.key
```

其他设备使用共享电脑的局域网地址访问。配置密钥后，文件 API 都需要验证；HTTP 本身不加密密钥和文件内容。`--modules files` 会关闭上传和管理路由，更多参数见 `share-server --help`。

## 桌面宿主

安装所在平台的 Tauri 构建依赖，然后执行：

```sh
npm ci --prefix apps/desktop
npm --prefix apps/desktop run dev
```

通过系统对话框选择目录，设置仅本机或局域网访问、上传和管理权限、访问密钥，再启动服务并打开文件管理窗口。页面重载后显示实际运行设置，状态接口不会返回访问密钥。关闭文件管理窗口会停止共享。Android 的存储适配和前台服务仍需独立实现。

保留可复用的点阵字标，随产品配置更名。启动呈现使用轻量过渡，跟随实际就绪状态结束，没有强制等待，并尊重系统“减少动态效果”设置。清理时间读取实际 CSS 过渡时长和延迟，上限一秒；退出动画期间，已就绪的控件始终可以操作。

在原生宿主内，五秒内激活应用名称按钮五次可打开临时诊断面板；键盘操作同样可用。下载缓冲可在 16–128 KiB 间按 16 KiB 调整，默认 128 KiB，只影响后续下载。面板可对相对共享文件路径进行一次只读本机回环测速，选择 1/2/4/8 个连接，最多读取文件前 64 MiB，整体超时 30 秒；测试文件必须非空，且每个连接至少分到一个字节。可取消测试，关闭诊断或停止共享会恢复默认值。结果反映共享设备上的本机采样，不能代表另一台设备的网络速度；HTTP 文件管理窗口没有原生诊断权限。

Linux 原生生产编译：

```sh
npm --prefix apps/desktop run tauri -- build --no-bundle -- --locked
```

## 验证与打包

```sh
python3 scripts/sync_product.py --check
python3 -m unittest discover -s tests -v
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
cargo fmt --all -- --check
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check
cargo test --workspace --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo build -p share-cli --locked
npm --prefix apps/web run test:e2e
python3 scripts/verify_runtime.py --mib 128
```

浏览器测试使用真实 Rust 服务和文件系统，原生命令边界模拟与实际设备测试分别记录。可在 `apps/web` 中运行 `npx playwright install --with-deps chromium` 安装测试浏览器，或通过 `PLAYWRIGHT_CHROMIUM_EXECUTABLE` 指定已有 Chromium。

当前本地验收通过 45 项核心、23 项原生、19 项 Python 工具及 89 项 Chromium 测试，同时完成严格 Clippy、生产构建、流式校验和 ZIP 迁移运行。使用完整文件校验的核心对比测量了 128 KiB 缓冲及原句柄 Range 定位，并与旧引擎、FileInfra 实际发布的 dufs 对照。[体积与运行报告](docs/distribution/size-and-runtime.md) 同时记录下载改善、多连接负载下更高的峰值内存和本机缓存采样的边界；这些结果尚不能代表 GUI 或物理局域网性能。

[桌面检查点 c004d68](https://github.com/zocs/inout-next/actions/runs/43649949553) 已按顺序通过全部五个目标。下载后的五份 CLI ZIP、六份原生包已核对程序架构及本轮测试的全部 35 个网页资源，Debian 包与 Windows 安装器的生成身份检查也已通过。产物验证与安装、真机、离线运行及签名验收分别记录。

[CI 工作流](.github/workflows/ci.yml) 使用仓库级单队列：先 Linux 验证，再 Windows x86/x64，最后 macOS arm64/x64；矩阵均设置 `max-parallel: 1`。macOS 只编译一次，随后仅对 `hdiutil create` 的资源繁忙错误最多尝试三次打包并保留每次输出；其他错误立即失败。产物区分 CLI ZIP 和原生安装包，Android、Windows 离线便携版和旧 Linux 兼容版有独立里程碑。

```sh
cargo build -p share-cli --release --locked
python3 scripts/package_portable.py --binary target/release/share-server \
  --target linux-x64 --output .local/packages
```

ZIP 包含固定名称的服务程序、构建好的 `web/`、产品配置、许可证和运行说明，不包含密钥或构建工具。运行 CLI ZIP 与运行 Tauri 桌面包的依赖不同。

## 从初期准备应用商店

[商店调研](docs/distribution/store-readiness.md) 按 2026-11-14 的官方要求分别记录 Google Play、F-Droid、iOS/iPadOS 和 Mac App Store。Android 需要当前 API 36 提交目标、16 KB 原生库验证、SAF 目录授权、正确的前台服务，以及实际运行的 F-Droid 源码构建 recipe；Apple 需要分别实现 iOS 前台共享和 Mac App Store 沙盒、签名及隐私要求。稳定应用身份、独立构建号、签名延续、第三方许可及真实的数据/传输安全披露都纳入早期设计。当前桌面 preview 尚无 Android/iOS 产物或商店审核结果。

## 当前范围

新项目没有 dufs 依赖、复制的服务端或 sidecar；文件操作由 Rust 实现，Next.js 只导出静态网页。模块使用明确接口和配置注册，尚未实现动态第三方插件 ABI。修改 `product.json` 后同步和重建即可更换网页、窗口显示名称及便携包文件名；`config/native-identity.json` 单独保存关联的应用 ID、稳定安装名和 WiX 升级 UUID。显示改名时，系统安装名保持稳定；更换安装身份需要明确迁移和平台升级验证。

上传默认上限 2 GiB，支持连续流式传输，尚无断点续传和崩溃恢复。目录复制未实现。Windows 的文件及目录移动、重命名暂时关闭，上传和复制需要支持硬链接的文件系统；Linux/Apple 使用原生排他重命名。服务端拒绝根目录修改、路径越界、符号链接分享和隐式覆盖。

传输完成或取消后，磁盘上保留空的内部 `INOUTTMP` 目录，避免并发上传竞争临时目录清理。公共列表隐藏该目录，取消操作仍会移除未发布文件；进程或系统崩溃后的恢复仍待实现。

公共网页资源必须经过存储提供者确认隔离，启动时生成不可变快照，上限 128 MiB、10,000 个资源文件、128 层目录，并最多扫描 100,000 个共享条目检查重叠及身份别名。过大或无法证明隔离的目录会拒绝网页启动；CLI 可省略 `--web-dir` 仅提供 API。文件系统范围下载在原句柄上定位，纯流式提供者保留向前读取支持；完整 ETag/If-Range 验证和持久续传尚未实现。部分文件系统操作同步执行，会话保存在进程内，HTTPS 反向代理的来源策略仍待设计。支持范围见[平台状态](docs/platforms.md)。

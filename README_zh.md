# inout

独立实现的 Rust HTTP 文件服务、静态 Next.js 文件管理页面和可独立构建的 Slint 原生桌面试验。现有 Tauri 桌面/Android 安装包保留为过渡预览。桌面端选择共享目录，Android 前台预览版使用应用私有存储，其他设备通过浏览器上传、下载和管理文件。品牌配置、存储提供者、功能模块和原生生命周期分开设计，便于替换名称或扩展平台。应用显示名为 **inout**，仓库及分发 slug 保留 **inout-next**。

最终应用采用 Rust 原生绘制的 GUI，Next.js 继续用于通过 HTTP 访问的浏览器文件管理器。**先验证 Slint，不合适再切换 egui**；平台、无障碍、分发许可及实际性能是采用条件。当前桌面试验已直接控制真实 Rust 服务，不嵌入 WebView。[原生 GUI 评估](docs/native-gui_zh.md)区分功能试验、初测和正式选型，目前尚未发布替代原生应用。

目前是实验性 **0.1.0 preview**。可下载[桌面 preview.1](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.1) 或 [Android arm64 preview.2](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.2)，使用说明分别见[桌面预览指南](docs/distribution/preview.md)和 [Android 预览指南](docs/distribution/android-preview.md)。[CI](https://github.com/zocs/inout-next/actions) 在同一串行队列中覆盖 Linux x64、Android arm64 打包、Windows x86/x64 和 macOS ARM64/x64。Android 运行检查使用 API 36、4 KiB 页面、x86_64 模拟器；arm64 实机、16 KiB 实际运行及两台设备 Wi-Fi 传输尚未验证。安装、离线运行和商店验收分别记录，具体见[平台状态](docs/platforms.md)。iOS 适配安排在后续。尚无稳定正式版本。

- [English README](README.md)
- [需求](docs/requirements.md) · [路线图](ROADMAP.md) · [平台状态](docs/platforms.md)
- [架构](docs/architecture.md) · [API v1](contracts/api-v1.md) · [模块](docs/modules.md)
- [品牌与身份](docs/branding.md) · [设计系统](DESIGN.md) · [包体积与运行依赖](docs/distribution/size-and-runtime.md)
- [默认分享地址与二维码规划](docs/network-selection.md)
- [原生 GUI 选型与性能验收](docs/native-gui_zh.md)
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

添加 `--writable` 可明确启用上传、创建目录及存储提供者支持的重命名、移动、复制，无须强制设置访问密钥。不设置密钥时，已发布文件的删除权限保持关闭。如果需要保护全部文件访问并启用删除，在共享目录外生成密钥文件，读取后填入浏览器：

```sh
python3 scripts/create_key.py ./access.key
cargo run -p share-cli --locked -- \
  --root /path/to/shared-folder --web-dir apps/web/out \
  --listen 0.0.0.0:5000 --writable --key-file ./access.key
```

其他设备使用共享电脑的局域网地址访问。配置密钥后，文件 API 都需要验证；HTTP 本身不加密密钥和文件内容。`--modules files` 会关闭上传和管理路由，更多参数见 `share-server --help`。

当前源码支持英语、简体中文、繁体中文、跟随系统语言及四种本地主题。切换语言或主题保留文件操作与传输状态。这些功能及可不设密钥的写入策略属于尚未发布的源码更新；现有 preview 下载包保留原先行为。

## 试用原生桌面应用

完成上面的网页导出，并安装目标系统的图形和字体构建依赖，然后执行：

```sh
cargo run --manifest-path apps/native-slint/Cargo.toml --locked
```

选择目录、仅本机或局域网访问、端口和权限后启动分享。口令可选，删除需要配置口令并通过认证。可选择实际分享地址，复制或在系统浏览器打开。停止和关闭正在分享的应用需要确认；“继续分享”保留监听和已接纳的传输。本轮传输记录在停止后保留，下一次成功启动时重置。语言、主题切换保留共享和输入内容。

试验支持英语、简繁中文、四种主题、原生点阵字标及跟随真实就绪状态结束的短启动动画。偏好暂不持久保存；启动前设置 `INOUT_NATIVE_REDUCE_MOTION=1` 可关闭启动动画。系统减少动态效果检测、持久记录和续传、单任务取消、二维码、实时设置及诊断控件尚待实现。Linux 的输入和布局验证不代表完整读屏或中文输入法验收；当前无障碍桥接存在明确的采用前检查项。

优化版 ZIP 打包、系统依赖及独立的 MIT/框架许可说明见[原生试验指南](apps/native-slint/README.md)。运行包不需要构建工具；原生试验不代表安装、旧系统、移动端或商店验收完成。

[3d0e656 的 Slint 试验 CI 产物](https://github.com/zocs/inout-next/actions/runs/43844556784)提供 Linux x64、Windows x86/x64 和 macOS ARM64/x64 ZIP。下载对应平台的 artifact 后，完整解压其中的 `inout-next-0.1.0-<target>-native-slint-trial.zip`。它们是试验产物，与已发布的预览安装包及 CLI ZIP 分开；各平台体积和运行依赖见[本轮 CI 包测量](docs/distribution/size-and-runtime.md#slint-trial-serial-ci-packages)。

## 过渡 Tauri 宿主

安装所在平台的 Tauri 构建依赖，然后执行：

```sh
npm ci --prefix apps/desktop
npm --prefix apps/desktop run dev
```

桌面宿主通过系统对话框选择目录，设置仅本机或局域网访问、上传和管理权限、访问密钥，再启动服务并打开文件管理窗口。页面重载后显示实际运行设置，状态接口不会返回访问密钥。关闭文件管理窗口会停止共享。

Rust 服务库已支持运行期间原子更新权限和密钥，并提供有界传输记录。已接纳的任务保留开始时的权限快照。原生实时设置控件，以及认证状态切换时浏览器传输任务的完整所有权处理仍待实现；当前宿主仍在停止共享后修改访问设置。

保留可复用的点阵字标，随产品配置更名。启动呈现使用轻量过渡，跟随实际就绪状态结束，没有强制等待，并尊重系统“减少动态效果”设置。清理时间读取实际 CSS 过渡时长和延迟，上限一秒；退出动画期间，已就绪的控件始终可以操作。

在原生宿主内，五秒内激活应用名称按钮五次可打开临时诊断面板；键盘操作同样可用。下载缓冲可在 16–128 KiB 间按 16 KiB 调整，默认 128 KiB，只影响后续下载。面板可对相对共享文件路径进行一次只读本机回环测速，选择 1/2/4/8 个连接，最多读取文件前 64 MiB，整体超时 30 秒；测试文件必须非空，且每个连接至少分到一个字节。可取消测试，关闭诊断或停止共享会恢复默认值。结果反映共享设备上的本机采样，不能代表另一台设备的网络速度；HTTP 文件管理窗口没有原生诊断权限。

Linux 原生生产编译：

```sh
npm --prefix apps/desktop run tauri -- build --no-bundle -- --locked
```

## Android 前台预览版

Android 宿主将应用自己的存储共享给另一台设备的浏览器。启用上传/管理，生成访问密钥，再启动共享并使用页面显示的局域网地址。上传文件在停止、重启及 Activity 重建后保留；清除应用数据或卸载会删除文件。当前没有任意目录选择、文件导入界面或本机文件管理窗口。

共享时保持应用在屏幕上。Home 或系统返回键会停止共享并取消未完成传输；回到应用后需手动重新启动。原生前台 Service、SAF 目录授权和通知控制仍是后续任务。预览版使用独立的 `io.github.zocs.inoutnext.preview` 包名与保留的预览证书，与 FileInfra 及未来正式应用分别安装。构建、安装和测试边界见 [Android 指南](docs/distribution/android-preview.md)。

## 验证与打包

```sh
python3 scripts/sync_product.py --check
python3 -m unittest discover -s tests -v
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
cargo fmt --all -- --check
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check
cargo fmt --manifest-path apps/native-slint/Cargo.toml -- --check
cargo test --workspace --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cargo test --manifest-path apps/native-slint/Cargo.toml --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo clippy --manifest-path apps/native-slint/Cargo.toml --all-targets --locked -- -D warnings
cargo build -p share-cli --locked
npm --prefix apps/web run test:e2e
python3 scripts/verify_runtime.py --mib 128
```

浏览器测试使用真实 Rust 服务和文件系统，原生命令边界模拟与实际设备测试分别记录。可在 `apps/web` 中运行 `npx playwright install --with-deps chromium` 安装测试浏览器，或通过 `PLAYWRIGHT_CHROMIUM_EXECUTABLE` 指定已有 Chromium。

Android 扩展增加了真实资源/索引、Kotlin 文件系统、私有宿主和前台生命周期检查。实际 release APK 已用于认证 HTTP 操作、浏览器上传/下载、监听关闭、重启文件保留及 Activity 生命周期测试；模拟器结果与真机、商店验收分别记录。此前使用完整文件校验的核心对比测量了 128 KiB 缓冲及原句柄 Range 定位，并与旧引擎、FileInfra 实际发布的 dufs 对照。[体积与运行报告](docs/distribution/size-and-runtime.md) 同时记录下载耗时、多连接峰值内存和本机缓存采样边界；这些结果尚不能代表 GUI 或物理局域网性能。

[桌面检查点 c004d68](https://github.com/zocs/inout-next/actions/runs/43649949553) 已按顺序通过全部五个目标。下载后的五份 CLI ZIP、六份原生包已核对程序架构及本轮测试的全部 35 个网页资源，Debian 包与 Windows 安装器的生成身份检查也已通过。产物验证与安装、真机、离线运行及签名验收分别记录。

[CI 工作流](.github/workflows/ci.yml) 使用仓库级单队列：Linux 验证、Android arm64、Windows x86/x64，最后 macOS arm64/x64；矩阵均设置 `max-parallel: 1`。各桌面目标分别测试、检查和构建 Slint，并在 CLI 与过渡 Tauri 产物旁提供独立原生 ZIP。Android 继续构建 Tauri 前台组合，复用 Linux 已测网页导出，验证 Cargo 子进程锁文件、优化后的未签名 APK、manifest、资源、ABI、ELF 和原生 ZIP 对齐；直接预览签名在产物验收后单独进行。macOS 只编译一次，随后仅对 `hdiutil create` 的资源繁忙错误最多尝试三次打包；其他错误立即失败。Windows 离线便携版和旧 Linux 兼容版仍有独立里程碑。

[3d0e656 的 CI 43844556784](https://github.com/zocs/inout-next/actions/runs/43844556784)六项任务全部成功，实际运行区间没有重叠。各桌面目标通过 15 项 Tauri 组合及 14 项 Slint 测试；核心测试分别为 Linux 88 项、Windows 各 80 项、macOS 各 86 项。Linux 另通过 56 项工具、131 项浏览器用例及 128 MiB 流式、8 MiB 移动目录运行检查。Android 通过七项 Kotlin 用例和两个锁定依赖的 arm64 编译子进程。五份 Slint ZIP、五份 CLI ZIP、六份过渡桌面包和一份未签名 Tauri APK，均匹配本轮精确的 36 文件 / 1,084,607 字节网页导出。这证明目标构建与产物检查通过；各平台原生实际运行、签名、商店及正式选型仍分别验收。

Android preview.2 来自 [4dd4b51 的 CI 43704226914](https://github.com/zocs/inout-next/actions/runs/43704226914)，六个目标全部串行通过。签名后的 arm64 APK 为 15.39MiB；[安装指南](docs/distribution/android-preview.md) 记录精确字节、校验值、各桌面目标32项原生测试、53工具/91浏览器/7Kotlin测试，以及单独的模拟器运行证据。

较早的多语言与实时权限核心检查点已通过 [abeabd1 的 CI 43744797175](https://github.com/zocs/inout-next/actions/runs/43744797175)，六个目标全部串行成功。各桌面目标通过33项 Tauri 宿主测试，Linux另通过53项工具和131项浏览器测试；Android通过七项Kotlin测试、锁定依赖的arm64实际编译及APK检查。五份服务端ZIP、六份桌面包和未签名APK均与该轮已测试的36个网页资源一致。该 CI 早于宿主抽取和 Slint 试验，不能作为新原生源码的 CI 验收。[平台状态](docs/platforms.md)记录核心测试数量与尚未完成的设备验收，[体积报告](docs/distribution/size-and-runtime.md)分别记录各检查点和已发布预览的测量。

Slint 集成本地通过 88 项核心/宿主、15 项 Tauri 组合、14 项 Slint、56 项工具及 131 项浏览器测试，并通过严格 Clippy 和格式检查。Linux 原生交互验证覆盖移出源码目录后的传输哈希、系统文件夹选择/取消、地址选择/复制、外部浏览器访问以及暂停上传期间的停止/正常关闭确认与清理。当前本地优化可执行文件/ZIP 为 17.88/8.79 MiB，包含精确的 36 文件浏览器导出；[体积与运行报告](docs/distribution/size-and-runtime.md)分别记录本地包的系统依赖、测量限制及 CI/发布产物。

```sh
cargo build -p share-cli --release --locked
python3 scripts/package_portable.py --binary target/release/share-server \
  --target linux-x64 --output .local/packages
```

ZIP 包含固定名称的服务程序、构建好的 `web/`、产品配置、许可证和运行说明，不包含密钥或构建工具。运行 CLI ZIP 与运行 Tauri 桌面包的依赖不同。

## 从初期准备应用商店

[商店调研](docs/distribution/store-readiness.md) 按 2026-11-14 的官方要求分别记录 Google Play、F-Droid、iOS/iPadOS 和 Mac App Store。Android 需要当前 API 36 提交目标、16 KB 实际运行、SAF 目录授权、正确的前台服务，以及真实 F-Droid 源码构建 recipe；前台 APK 预览并不代表通过这些关卡。Apple 需要分别实现 iOS 前台共享和 Mac App Store 沙盒、签名及隐私要求。稳定应用身份、独立构建号、签名延续、第三方许可及真实的数据/传输安全披露都纳入早期设计。iOS 产物与商店审核仍未完成。

## 当前范围

新项目没有 dufs 依赖、复制的服务端或 sidecar；文件操作由 Rust 实现，Next.js 只导出静态网页。模块使用明确接口和配置注册，尚未实现动态第三方插件 ABI。修改 `product.json` 后同步和重建即可更换网页、窗口显示名称及便携包文件名；`config/native-identity.json` 单独保存关联的应用 ID、稳定安装名和 WiX 升级 UUID。显示改名时，系统安装名保持稳定；更换安装身份需要明确迁移和平台升级验证。

上传默认上限 2 GiB，支持连续流式传输，尚无断点续传和崩溃恢复。目录复制未实现。Windows 的文件及目录移动、重命名暂时关闭，上传和复制需要支持硬链接的文件系统；Linux/Android/Apple 使用原生排他重命名。服务端拒绝根目录修改、路径越界、符号链接分享和隐式覆盖。

传输完成或取消后，磁盘上保留空的内部 `INOUTTMP` 目录，避免并发上传竞争临时目录清理。公共列表隐藏该目录，取消操作仍会移除未发布文件；进程或系统崩溃后的恢复仍待实现。

公共网页资源必须经过存储提供者确认隔离，启动时生成不可变快照，上限 128 MiB、10,000 个资源文件、128 层目录，并最多扫描 100,000 个共享条目检查重叠及身份别名。过大或无法证明隔离的目录会拒绝网页启动；CLI 可省略 `--web-dir` 仅提供 API。文件系统范围下载在原句柄上定位，纯流式提供者保留向前读取支持；完整 ETag/If-Range 验证和持久续传尚未实现。部分文件系统操作同步执行，会话保存在进程内，HTTPS 反向代理的来源策略仍待设计。支持范围见[平台状态](docs/platforms.md)。

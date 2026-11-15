# 应用商店前期准备：Google Play、Apple App Store、F-Droid

政策核查日期：**2026-11-14**；实现状态更新：**2026-11-15**。对应实验性 **0.1.0 preview**。已有 Android 应用私有存储前台 APK 和模拟器验证，尚无移动商店候选包或审核结果。本报告记录官方政策、早期设计要求和当前平台差距。官方来源清单见 [store-sources.json](store-sources.json)，政策变动及正式候选版前须重新核查。

## 结论与渠道划分

Rust + 静态 Next.js + Tauri 路线可以继续，也有 Tauri 官方的 [Google Play](https://v2.tauri.app/distribute/google-play/) 和 [Apple App Store](https://v2.tauri.app/distribute/app-store/) 打包路线。商店收录需要原生平台适配、隐私资料和真实构建/运行证据。当前桌面 preview 没有完成这些条件。

| 渠道 | 目标产物与用途 | inout 当前差距 |
| --- | --- | --- |
| Google Play | 签名 AAB，Play App Signing；用于正式 Android 分发 | 当前仅直接分发前台 APK；SAF、原生前台服务、AAB、Play 签名与审核资料未完成 |
| F-Droid 主仓库 | 从公开源码构建 APK；可选验证并保留上游签名 | 已有可提交的 Android 文本构建骨架与直接 APK；fdroiddata recipe、完整许可核对及可复现验证未通过，新项目仍需满足实际用途和维护条件 |
| iOS/iPadOS App Store | Xcode 归档、签名 IPA，通过 TestFlight 后再正式审核 | 没有 iOS 适配、目录授权、生命周期、隐私清单或设备验证 |
| Mac App Store | 沙盒化、签名 `.app` 和签名 `.pkg`、对应 provisioning profile | 现有目标为桌面 DMG；尚无商店配置、沙盒目录授权或签名验证 |
| GitHub/直接分发 | CLI ZIP、AppImage/deb、NSIS、DMG、Android APK | 桌面 preview 与 Android 前台 preview 分开验收，延续串行 CI；这些产物的通过不能替代商店准入 |

推荐先完善共享的自由软件实现和 Android 适配，尽早验证 F-Droid 构建，再推进 Play；Mac App Store 独立验证沙盒；iOS 先验证用户可理解的前台共享体验。所有渠道复用 Rust 文件操作和静态网页，平台权限、服务所有权、签名与商店资料分别管理。

## Google Play：已经生效与需要提前处理的规则

| 官方要求 | 当前日期下的含义 | 项目准备与通过证据 |
| --- | --- | --- |
| [Target API](https://support.google.com/googleplay/android-developer/answer/11926878?hl=en) | 自 **2026-08-31** 起，普通 Android 新应用及更新须 target **Android 16 / API 36 或更高**；延长期至 2026-11-01 已过去。已有应用的可发现性要求与新应用提交要求不同 | Android 首个计划须明确 `compileSdk`/`targetSdk`，至少满足当时的普通应用提交要求。`minSdk=24` 是兼容目标，不能代替 target 要求；Tauri/Next 的旧设备运行须另测 |
| [AAB](https://developer.android.com/guide/app-bundle) 与 [签名](https://developer.android.com/studio/publish/app-signing) | 新应用采用 AAB/Play App Signing；上传密钥与应用签名密钥职责不同 | 分开保管 app-signing key、upload key、密码及证书指纹；核对控制台包名和实际 AAB manifest。APK 用于直接分发/F-Droid，两者不可混为一项验收 |
| [64 位](https://developer.android.com/google/play/requirements/64-bit) 与 [16 KB 页面](https://developer.android.com/guide/practices/page-sizes) | 原生代码需要相应 64 位支持。当前页面规定 target API 35+ 的 64 位应用支持 16 KB，并列出 **2027-02-01** 起不兼容更新无法发布；以本次抓取的当前原文为准 | Android 首个产物先覆盖 arm64。若增加 32 位 ABI，按政策提供对应 64 位 ABI。所有打包 `.so` 都检查 ELF LOAD/RELRO、APK ZIP 对齐、AAB 配置及 16 KB 实际运行；Rust 编译成功不是页面兼容证据 |
| [原生页面构建指导](https://developer.android.com/guide/practices/page-sizes) | 官方推荐 AGP **8.5.1+**、NDK **r28+**；仍须检查实际打包结果，不能只依赖版本号 | Android 计划固定 JDK/Gradle/AGP/SDK/NDK/Rust 和工具校验值；验证 Rust、Tauri 及全部间接原生库。使用 `zipalign -c -P 16 -v 4`、`bundletool dump config`、ELF 检查和 `adb shell getconf PAGE_SIZE` 的实际结果 |
| [所有文件权限](https://support.google.com/googleplay/android-developer/answer/10467955?hl=en) | `MANAGE_EXTERNAL_STORAGE` 受限，允许类别也需证明核心用途及为何 SAF 不够，并经权限声明审核 | **默认使用 SAF 用户选择的目录与应用私有存储**。不因“文件管理器”名称就加入所有文件、媒体、定位或电话权限；只有真实必要且已论证的功能再引入 |
| [前台服务类型](https://developer.android.com/develop/background-work/services/fgs/service-types) 与 [Play 声明](https://support.google.com/googleplay/android-developer/answer/13392821?hl=en) | API 34+ 要求正确类型/权限，Play 要求用途、停止/延迟影响和演示视频；每种类型都受审核 | 由 Android 原生 Service 持有 Rust 服务，用户显式启动、可感知通知、明确停止和安全清理。持续与外部网络设备交互可评估 `connectedDevice`，须实际满足用途和前置条件，不能靠无关权限满足检查；`specialUse` 需具体理由与审核 |
| [`dataSync` 时限](https://developer.android.com/develop/background-work/services/fgs/timeout) | Android 15+ 对后台 `dataSync` 累计 **6 小时/24 小时**，并要求及时处理 `Service.onTimeout`；不能把它视为无限常驻服务 | 有限传输评估 user-initiated data transfer jobs；共享监听与传输分别设计所有权。测试超时、通知 Stop、Activity/WebView 销毁、权限撤销和进程退出，不承诺绕过系统限制的永驻行为 |
| [局域网权限](https://developer.android.com/privacy-and-security/local-network-permission) | 当前文档规定 target **Android 17 / API 37+** 时声明/请求 `ACCESS_LOCAL_NETWORK`；target 36 及以下用 `INTERNET` 隐式授权，不提前申请新权限。Android 16 的旧 opt-in 指导已标为 deprecated | 原生适配须区分 SDK，支持拒绝/撤销；测试 Rust TCP 接收与 WebView 请求。发现、广播和直接监听分别按实际操作申请能力，不能复制过时的 `NEARBY_WIFI_DEVICES` 方案 |
| [Data safety](https://support.google.com/googleplay/android-developer/answer/10787469?hl=en) 与 [User Data](https://support.google.com/googleplay/android-developer/answer/10144311?hl=en) | closed/open/production 渠道须填写，只有纯 internal testing 有列明豁免；不收集数据的应用也需表单和隐私政策。共享文件可能包含敏感数据 | 以 [数据清单](privacy-data-inventory.md) 和最终 APK 网络/依赖行为填写。当前 HTTP 无传输加密，不能勾选“全部加密”或宣传 E2E；商店候选版需完成适当的传输安全、授权与撤销设计 |
| [个人账号测试](https://support.google.com/googleplay/android-developer/answer/14151465?hl=en) | **2023-11-13 后创建的个人账号**需至少 **12 名连续加入 14 天**的 closed testers，再申请 production access；不适用于所有账号 | 核对用户实际账号类型/创建日期后安排测试者和反馈记录。GitHub CI、自动测试和 internal testing 都不能代替该条件 |

[Play 注册说明](https://support.google.com/googleplay/android-developer/answer/6112435?hl=en) 还涉及年满 18 岁、身份核验、账号类型和一次性 **US$25** 注册费；新个人账号有 Android 设备核验要求。是否已满足这些条件需由现有账号实际状态判断。

[Android developer verification](https://developer.android.com/developer-verification) 的当前公开时间表列出 **2026-09-30** 起在巴西、印度尼西亚、新加坡、泰国的认证 Android 7+ 设备上，针对参与商店开始保护措施，并在 2027 年进一步扩展；另有 limited-distribution 与 advanced flow。它是分发环境的独立事项，不等于 F-Droid 收录，不能推断所有地区/所有侧载方式已经一律禁止。正式分发时重新核对适用渠道、地区、包名和签名登记规则。

## Apple：分别准备 iOS 和 Mac App Store

### 当前提交要求

[Apple Upcoming Requirements](https://developer.apple.com/news/upcoming-requirements/) 当前列出：

- 自 **2026-04-28** 起，上传相应移动/设备平台应用须使用 **Xcode 26+** 及 iOS/iPadOS/tvOS/visionOS/watchOS **26** SDK。此条没有把 macOS 也列入同一 SDK 清单，不能把移动 SDK 规则直接套到现有 DMG 构建。
- 自 **2026-09-09** 起，iOS/iPadOS 上传应用最低目标为 **iOS 13+**；构建 SDK 与最低部署版本分开设置。
- 更新后的年龄分级问卷期限为 **2026-01-31**，EU 分发要处理并核验 DSA trader status。
- Mac App Store/TestFlight 上传包不能含 `com.apple.quarantine` 扩展属性；这是上传包的构建/核查要求，不是删除用户下载包安全标记的理由。
- [Apple Developer Program](https://developer.apple.com/programs/enroll/) 个人/组织身份、组织 D-U-N-S 等要求需按账号类型处理；会员通常 **US$99/年**，以所在地区和官方资格/费用为准。

### iOS 产品和平台约束

[审核指南 2.5.4](https://developer.apple.com/app-store/review/guidelines/#software-requirements) 只允许后台服务用于其规定用途。普通 HTTP 文件服务不具备 Android 式无限常驻前台服务；不能为了继续监听而伪装音频、VoIP、定位或其他 background mode。

首个 iOS 设计应提供：用户选择目录、显式开始共享、前台有效状态、可理解的中断/停止、取消和重新开始。退到后台时允许的有限任务完成与清理须按真实系统行为设计和验证；不要宣称整晚后台共享。通过 [目录选择 API](https://developer.apple.com/documentation/uikit/providing-access-to-directories) 获取 security-scoped URL，并在提供者/授权对象中管理访问生命周期；不能把外部目录 URL 当成无限有效的普通 POSIX 路径。

[2.5.5](https://developer.apple.com/app-store/review/guidelines/#software-requirements) 要求 IPv6-only 可用：现有 native/CLI 的 IPv4 假设须在 Apple 候选版验证 IPv6 地址显示、绑定、URL、连接和下载。静态 Next 页面与 WebKit 宿主可继续复用；[4.2](https://developer.apple.com/app-store/review/guidelines/#minimum-functionality) 要求超过网站包装的实际用途。真实原生目录选择、共享服务控制、停止/权限和平台文件交互是应交付的产品能力，单纯加载网页不构成通过。

### Mac App Store 沙盒与打包

[2.4.5](https://developer.apple.com/app-store/review/guidelines/#hardware-compatibility) 要求 App Sandbox。按 [Tauri 商店配置说明](https://v2.tauri.app/distribute/app-store/#macos)，使用独立商店配置覆盖层、相应签名与 provisioning profile，并生成签名 `.pkg`。直接分发 DMG 的签名/notarization 和 Mac App Store 分发证书/沙盒验收分别记录。

原生实现须按 [macOS 沙盒目录访问](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox) 和 [入站网络 entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.network.server) 设计：用户选择目录的适当 read/write 权限、需要持久化时的 security-scoped bookmark、访问 scope 的开启/释放、入站监听以及实际需要的出站连接权限。长时间异步文件操作期间保持授权；撤销/失效后停止或返回明确错误。主应用、框架/辅助进程、资源与 entitlement 的实际签名结果都要查验。

目前 `config/native-identity.json` 保存稳定的原生身份，但现有构建未配置这些 Apple 商店权限或证书，不能据此宣称沙盒支持。

### 隐私、局域网与代码完整性

| 要求 | inout 的准备 |
| --- | --- |
| [Privacy labels](https://developer.apple.com/app-store/app-privacy-details/) 和 [5.1.1 隐私政策](https://developer.apple.com/app-store/review/guidelines/#privacy) | 所有提交平台都要根据实际应用和第三方数据处理填写，隐私政策有公开 URL 且应用内易访问；没有账户就不添加账号体系。以后若引入账号创建，补充应用内账号删除及对应渠道要求 |
| [Privacy manifests](https://developer.apple.com/documentation/bundleresources/privacy-manifest-files)、[required-reason APIs](https://developer.apple.com/documentation/bundleresources/describing-use-of-required-reason-api)、[指定第三方 SDK](https://developer.apple.com/support/third-party-SDK-requirements/) | iOS 等列明平台的实际 required-reason API 需要准确的理由和各自 bundle 内 `PrivacyInfo.xcprivacy`；文件时间、磁盘空间、UserDefaults/启动时间等是待核对类别，不能凭模板填写理由。逐项检查 Tauri、插件及间接库、资源位置和 Xcode privacy report。当前官方 required-reason 说明列明移动/设备平台，不把这条自动宣称为 macOS 相同的强制规则；macOS 数据披露和适用 SDK 要求仍需处理 |
| [Local network privacy / TN3179](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy) | 原生访问局域网按操作添加 `NSLocalNetworkUsageDescription`；注册/浏览 Bonjour 时列 `NSBonjourServices`，iOS 某些 multicast/broadcast 另需 entitlement。官方区分入站 TCP、原生出站 TCP 和 WKWebView 等豁免，不能假设打开入站 HTTP 监听一定触发授权，也不能把 Android 规则套用到 Apple。macOS 15+ 的隐私行为另测 |
| [ATS 本地网络](https://developer.apple.com/documentation/bundleresources/information-property-list/nsapptransportsecurity/nsallowslocalnetworking) | 按实际 URL/WebKit/原生请求验证最小例外；不以全局 `NSAllowsArbitraryLoads` 代替传输安全设计。ATS 例外不等于局域网隐私授权或加密 |
| [2.5.2 软件完整性](https://developer.apple.com/app-store/review/guidelines/#software-requirements) | Next 页面在构建时导出并随已签名包提供，Rust 内嵌共享服务；商店版采用编译时模块组合。禁止下载可执行插件/服务、远程脚本改变原生功能或把共享用户文件执行为应用代码。已有未实现的动态插件 ABI 不进入商店基线 |
| [加密出口合规](https://developer.apple.com/help/app-store-connect/manage-app-information/overview-of-export-compliance/) | 按实际加密实现、系统/第三方库及分发地区判断 `ITSAppUsesNonExemptEncryption` 和需要的材料；不能因为用 Rust、HTTP 或示例写 `false` 就认定已豁免 |
| [完整性与测试](https://developer.apple.com/app-store/review/guidelines/#performance) | 正式商店不接收 demo/beta；使用 TestFlight。给审核者可执行的目录选择、共享和浏览器操作步骤以及可复现测试数据；资料、设备行为、权限描述和实际功能一致 |

## F-Droid：从初期验证源码构建

### 准入和构建边界

[当前 Inclusion Policy](https://f-droid.org/en/docs/Inclusion_Policy/) 要求实际维护、对用户有用、真实公开源码、自由软件许可及依赖；网站包装也需要原生增强，demo 不适合收录。政策还列出至少 **3 个月活跃开发**，或已有用户、作者已维护其他 F-Droid 应用、活动专用应用等例外。旧 FileInfra 曾尝试提交不自动证明本项目满足例外；正式申请时给出真实证据。

当前 MIT 项目许可、独立 Rust 核心、静态本地网页和无商店依赖的方向适合继续。完整依赖与资产许可核对仍未完成，不能用根目录 MIT 代替所有第三方义务。禁止依赖 Firebase/GMS/Crashlytics 或专有广告/追踪 SDK；若未来确实增加渠道功能，依赖保持在独立宿主/编译配置内，并提供完整可构建的自由软件版本。

政策当前允许有条件使用 Rust/Rustup、Node 等预编译自由软件构建工具，优先 Debian 可用工具；这是**可能获准的构建输入**，不是任意 npm 二进制或 `.so` 的通行证。Next/SWC、Tauri CLI、Gradle wrapper、NDK 以及每项原生库都要记录来源、版本、校验值和许可；对 SWC 等预编译工具明确源码构建路径或获准使用的依据，不能用整个 `node_modules` 的 `scanignore` 隐藏问题。

[官方贡献指南](https://gitlab.com/fdroid/fdroiddata/-/blob/master/CONTRIBUTING.md) 明确支持 GitHub 上游；**无需把 inout-next 主仓库迁往 GitLab**。F-Droid 的 recipe/MR 位于其 GitLab fdroiddata，源代码仍可以是 GitHub。

### 第一份 recipe 的验收顺序

1. 完成拟提交版本的真实 Android 功能、存储/权限和产物。当前 app-private 前台服务宿主已有直接 APK；SAF、原生前台 Service 和本机导入仍是后续范围。保留可维护的原生源码和可重复的命令行构建步骤：Android 文本骨架已从 `**/gen/` 中明确放行，两个子级 ignore 保持生成配置、资源、原生库、缓存和私钥排除，不能让 clean checkout 缺少必要 recipe 输入。
2. 固定工具链、SDK/NDK/JDK/Gradle/Rust/Node/npm 和所有 lockfiles，记录获取阶段；在 recipe 中从同一公开提交构建 **Next 导出和 Rust 原生库**，不下载 GitHub 已编译 ZIP/web-export 当作应用源码替代品。无需 Node 运行时或私有密钥/API token。
3. 在 `metadata/io.github.zocs.inoutnext.yml` 的真实构建块中填写提交、原生目录、版本、ABI、Gradle/手动步骤和实际 APK 输出。这个文件名是沿用现有身份的候选；先核对包名可用性及归属。没有真实 Android 源码和输出前，不提交伪造的可运行 recipe。
4. 用固定版本 fdroidserver/构建环境执行 `fdroid readmeta`、`fdroid rewritemeta <appid>`、`fdroid lint <appid>`、`fdroid checkupdates <appid>` 和实际 `fdroid build -v -l <appid>`；记录各自成功/失败阶段。官方 [Quick Start](https://f-droid.org/en/docs/Submitting_to_F-Droid_Quick_Start_Guide/) 的旧环境大小/速度示例不是当前资源保证，容器及工具版本在实际试验时再固定。
5. 用第二个干净环境按同一 recipe 重建；将 APK 和编译输入差异定位到 ZIP 顺序/时间、native ELF、DEX、资源、路径或版本，而不是连续盲目重试远程 CI。只有本地/容器 recipe 跑通后才推进一项 GitLab 验证和 MR，仍不和 GitHub CI 重叠。
6. 正式 release 有不可变源提交、确定版本、与 APK 一致的标签及更新检查。Android `versionCode`/Apple build number 应显式、单调管理；多个 preview 或 ABI 不依赖单纯显示版号推导，否则同版修订可能无法升级。F-Droid 更新检查用静态值/明确 `UpdateCheckData`，不能依赖构建时执行代码算版本。

构建失败应分别定位 metadata、scanner、依赖获取、web、Rust、Gradle、APK 及签名比较阶段。验证需要明确的源版本、recipe、镜像、工具版本和实际命令结果。

### 可复现与签名：优先尝试，不能虚报

[F-Droid FAQ](https://f-droid.org/en/docs/FAQ_-_App_Developers/#are-reproducible-builds-required) 明确可复现构建**并非全部收录的硬性要求**，但鼓励新项目从一开始做。[Reproducible Builds](https://f-droid.org/en/docs/Reproducible_Builds/) 解释了验证上游 APK 并保留上游签名的方式；现代 APK v2/v3 签名需要匹配相应未签名前的完整字节，而不只是抽出的文件内容。

准备时控制源/构建路径、文件顺序、时间、Rust/NDK 工具版本、编译并行/代码生成差异和 Next build ID。只在真实差异需要时调整 `SOURCE_DATE_EPOCH`、路径 remap 或 Rust `codegen-units` 等；不宣称现有桌面 ZIP/export 已达到 APK 可复现。

默认 F-Droid 为应用创建自己的签名；同一个 Android ID、不同签名通常不能相互覆盖升级。推荐先评估由开发者保留应用签名密钥、Play App Signing 使用对应密钥、Play upload key 单独管理，再通过 F-Droid 验证上游签名。密钥选择、Play 加密导入、密钥轮换与跨渠道实际升级要独立验证。若采用 F-Droid 自签名，须在首次分发前明确渠道身份/迁移及用户影响，不通过更换产品名或 keystore alias 解决签名不匹配。

## 已落实的早期准备与尚未通过的关卡

当前落实：本报告、官方来源清单、[隐私数据清单](privacy-data-inventory.md)、需求/架构/模块/品牌/平台/路线图中的商店约束；`.gitignore` 排除 Apple `.p8`/`.p12`/provisioning profile、签名本机配置和商店二进制。生产身份仍是 `io.github.zocs.inoutnext`，Android 直接预览独立使用 `.preview`、显式 `versionCode=100001` 和保留的预览证书；未导入 FileInfra/生产密钥，未完成商店注册。显示改名不自动更改包名、签名或安装身份。

[Android 前台 preview.2](android-preview.md) 已实现私有共享目录、原始网页资源校验、前后台停止/手动重启和能力约束。[CI 43704226914](https://github.com/zocs/inout-next/actions/runs/43704226914) 六个目标串行通过；其 arm64 APK 经独立预览证书签名后为15.39MiB，manifest、单 ABI、全部资源、ELF/native ZIP 16 KiB 对齐和签名检查通过。相同应用源码的 API36 x86_64/4 KiB 模拟器 APK 通过实际 HTTP/浏览器操作与生命周期验证。真实 arm64、16 KiB 运行、双设备 Wi-Fi、API24/WebView、AAB 和商店签名仍须分别验收，不能由对齐检查或模拟器结果替代。

2026-11-15 产品补充：应用显示名为 **inout**，仓库/slug 保留 `inout-next`；[签名与应用身份](signing.md) 说明与 FileInfra 共用证书时的独立包名、signature 权限及渠道影响。原生临时诊断通过五秒内激活名称按钮五次开启，仅执行受限本机调参和只读回环测速，没有遥测或公开 HTTP 控制接口。按 [Apple 审核指南 2.3.1](https://developer.apple.com/app-store/review/guidelines/#accurate-metadata)，候选版应在审核备注中明确说明入口、用途和步骤，不能以隐藏发现方式隐瞒审核功能；数据清单和候选版说明同步包含诊断行为。商店产物、真机与商店审核关卡仍需独立验收。

| Gate | 要交付的可核查结果 | 当前状态 |
| --- | --- | --- |
| S1 身份与渠道 | 所有渠道包名/Bundle ID、归属、版本递增、签名指纹/持有人/备份及迁移记录；密钥别名不随品牌同步 | 稳定桌面身份、Android 预览 ID/构建号/证书已有；生产渠道、商店注册与升级/迁移未验收 |
| S2 原生存储与服务 | SAF/security-scoped 提供者、真实权限、失败/撤销处理、后台/超时/停止与清理，按能力隐藏无法安全实现的操作 | Android app-private 前台适配与停止/重建已验证；SAF/原生前台 Service、iOS/Apple 沙盒待实现 |
| S3 原生产物与兼容 | 实际 target/manifest/ABI/16 KB/IPv6/WebView、签名/沙盒/资源检查以及最低与当前系统设备记录 | 桌面 target CI、Android 直接 APK 包检查与 x86_64/4 KiB 模拟器已有；arm64/16 KiB/最低 OS/双设备与商店产物未验收 |
| S4 隐私与安全 | 最终依赖/数据流、公开隐私政策/联系入口、应用内访问、两家表单、必要 manifest/SDK、传输加密/配对/撤销设计与测试 | 当前数据清单已建立；HTTP 明文、政策发布/原生隐私清单及商店表单未完成 |
| S5 自由软件/源码构建 | 依赖和资产许可/SBOM、干净 checkout 的 web/Rust/Gradle 源码构建、实际 fdroidserver recipe、可复现尝试/差异记录 | 根项目 MIT、锁文件和 Android 文本骨架已有；完整许可清单、F-Droid recipe/构建/可复现未通过 |
| S6 审核资料与测试 | 真实功能描述、分级、联系/支持、必要截图/演示、适用的 closed testing/TestFlight、审核者操作步骤 | 商店截图/演示、适用的 closed testing/TestFlight 和提交材料待准备 |
| S7 发布运维 | 单队列 CI、受保护的发布凭据、签名产物/来源/checksums、商店/直接渠道分开验收与回滚/升级记录 | 桌面preview.1与Android preview.2有各自来源、串行CI、指南/checksums；Android预览签名已核验。商店发布配置与实际升级验收待独立实现 |

正式候选版前重新检查官方政策及控制台当时的提示。SDK/政策日期放在这份分发研究与平台构建配置，避免耦合到 Rust domain/API 模块。各 Gate 有实际结果后才改为通过；框架教程、源码 review 或另一渠道 CI 不自动清除关卡。

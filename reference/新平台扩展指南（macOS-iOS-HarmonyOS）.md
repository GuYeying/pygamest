# 新平台扩展指南（macOS / iOS / HarmonyOS）

> 2026-11-11（八阶段目标·阶段七）。面向要把 pygamest 游戏带到新平台
> 的开发者。架构依据见 `reference/多平台扩展性架构分析与整改（阶段六）.md`
> ——核心结论：**新平台 = 新增适配层，核心实现零改动**。

## 一、接入位总览（四层）

| 层 | 接入位 | 契约 |
|---|---|---|
| 窗口/输入 | `starfish-we/src/platform/<os>/`（mod.rs + translate.rs） | `poll_event()` 非阻塞泵 + 事件翻译 + rwh 裸句柄直通 wgpu；物理像素在模块边界换算 |
| 视频硬解 | `starfish-video/src/<os>.rs` + lib.rs cfg 分支 | `open → async` 门面；输出统一 `DecodedFrame`/`FramePixels`；无后端 = `VideoError::UnsupportedPlatform` 站桩 |
| 音频 | `starfish-audio/src/device` trait 实现 | cpal 胶水为"平台差异唯一收敛点"——新平台实现同一 trait，混音层零改动 |
| 打包 | `pygamest-runtime/scripts/build_<os>.py` + 总控 builders 表登记 | 产物进 `dist/`；加密资产链（bundle.py）与清单形态复用 |

**平台无关层（永远不用动）**：starfish-std / starfish-font /
vello 光栅 / starfish-render 管线 / starfish-pygame 绘制词汇 /
pygamest 绑定 / FRAME_BRIDGE 调度 / starfish-io 路由 / starfish-net。

## 二、macOS（最短路径——适配层已实装）

现状：AppKit 窗口/事件后端（867 行）+ VideoToolbox 视频后端已实装；
wgpu Metal 与 cpal CoreAudio 由依赖自动覆盖。

剩余步骤（仅打包）：

1. **build_macos.py**：cargo build（`aarch64-apple-darwin` /
   `x86_64-apple-darwin` universal2 可选）→ `.app` bundle 组装
   （`Contents/MacOS/<name>` + `Info.plist`：CFBundleIdentifier /
   NSHighResolutionCapable / SSID）→ 加密资产链复用 bundle.py →
   codesign（ad-hoc `codesign -s -` 起步，分发换 Developer ID）。
2. 总控 `scripts/build.py` builders 表登记 `"macos"`（预留插槽已备）。
3. **验收门**：`cargo check --target aarch64-apple-darwin`（历史已过
   skip-taskbar 面）→ 真机/VM 跑 netvideo_test（窗口/输入/视频全链）→
   IME marked-text 深度验收。

## 三、iOS

现状：UIKit 后端（含 view.rs 触摸/文本输入）实装；Metal/CoreAudio/
VideoToolbox 同上自动覆盖。

剩余步骤：

1. **工程模板**：Xcode 项目（或 cargo-mobile 形态）——静态库/框架
   嵌入 + CAMetalLayer 挂接（starfish-we rwh 句柄直通）。
2. **build_ios.py**：`cargo build --target aarch64-apple-ios` →
   xcframework → 资产入 bundle → 真机签名（描述文件）→ .ipa。
3. **音频 Session**：AVAudioSession category 激活配置（cpal 之上
   平台前置——iOS 进入后台/中断策略）。
4. **验收门**：`cargo check --target aarch64-apple-ios` → 真机
   netvideo_test（触摸映射 / 键盘生命周期 / 沙箱 base_dir）。

## 四、HarmonyOS（全栈新增）

工作量最大，按层推进（每层独立可验收）。**第一步已完成**（2026-11-11）：

1. **cfg 定型 ✅ 已实证**：`rustc --print cfg --target
   aarch64-unknown-linux-ohos` → `target_os="linux"` + `target_env=
   "ohos"`。**分流桩已落地**：starfish-we `platform/harmonyos/`
   （WindowInner 刻意不可构造 + 全契约显式 Err 站桩，feature 门控
   镜像 linux）+ video `UnsupportedPlatform` 分流 + linux 依赖门
   （wayland/gstreamer/tray-icon）摘 OHOS——`cargo check --target
   aarch64-unknown-linux-ohos -p starfish-we` ✅（否则 OHOS 会静默
   吞 linux-Wayland/GStreamer 路径）。
2. **音频（真适配第一项）**：starfish-audio cpal 门控摘 OHOS（当前
   OHOS check 卡点 = alsa-sys 构建脚本）+ OHAudio 实现 device trait
   （record/stream 模块同步割）——解锁全树 OHOS check 门。
3. **窗口/输入实装**：`platform/harmonyos/` 桩 →
   NativeXComponent 回调 → 缓冲队列 → `poll_event` 排空（模板 =
   android/mod.rs 反转形态）；表面 = OH_NativeXComponent_GetNativeWindow
   → `RawWindowHandle::Ohos` 新变体（连带 handle.rs 消费方全 match
   更新 + wgpu 映射）。
4. **渲染**：wgpu OHOS 后端缺口——路线 A：wgpu GL 后端 + EGL on
   OHOS（GLES3.x；需 vendor patch，警惕 GLES 实现栈差异——
   base_vertex 先例见 pygame_live2d 修复）；路线 B：wgpu-hal 自研
   OHOS hal（成本高，非必要不走）。
5. **视频**：AVPlayer/AVCodec NDK 后端（`ohos.rs`；当前
   UnsupportedPlatform 站桩已就位保语义）。
6. **打包**：build_ohos.py——hvigor/DevEco 工具链 + module.json5 +
   资产 HAP 打包（对齐 android/build.py 五步链）；总控 builders 表
   预留插槽已备。
7. **验收门**：`cargo check --target aarch64-unknown-linux-ohos`
   （we/video ✅，audio/pygame 待音频割后解锁）→ OHOS 真机/模拟器
   netvideo_test 三模块全绿。

## 五、通用不变式（跨平台纪律）

1. **显式优于隐式**：相机/管线/资源全显式传入（阶段一定约），新
   平台适配层不得引入兜底。
2. **单一职责**：平台差异只进 platform/<os>/（或对应后端模块），
   门面与核心层零平台字符串。
3. **站桩语义**：未实装能力显式 Err（文案含平台名与后续路径），
   不静默。
4. **io 单源**：一切文件访问走 sfx::read 路由，新平台只提供
   base_dir 语义。
5. **回归门**：每接入一层跑一次三平台（Windows/WASM/Android）
   netvideo_test——基线见阶段四（Win 16/16 / WASM assert 过 /
   Android 绿屏+回声侧证）。

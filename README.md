# pygamest

**次世代 pygame —— Python 游戏开发的原生形态。**

[starfish](https://github.com/GuYeying/starfish) 引擎（wgpu GPU 渲染）的
Python 绑定层：内嵌 [RustPython](https://github.com/RustPython/RustPython)
解释器，`import pygame` 直达 **Rust 原生模块**（`#[pymodule]` 树，零 shim
零 ctypes）——pygame 的心智，GPU 的出生面，三平台一套代码。

- **三平台**：Windows 桌面 / Web（wasm32）/ Android（arm64 真机，鸿蒙
  NEXT + 卓易通容器实测）
- **依赖单向红线**：`pygamest → starfish`（引擎本体零 Python 资产）

## 与 pygame 的关系（免侵权声明）

**独立项目，零关联声明**：本项目（pygamest）与 pygame（pygame.org /
pygame-ce）及其社区**无任何隶属、合作或背书关系**；与 SDL 同样无任何
关系。

- **零代码借用**：本项目不包含、不链接、不翻译、不派生 pygame（LGPL）
  或 SDL 的任何源码、二进制与美术资源——实现为 **100% 原生 Rust**：
  Python 解释器 = [RustPython](https://github.com/RustPython/RustPython)
  （MIT，vendored 快照）；渲染 = wgpu；窗口/事件/音频 = starfish 自研
  Rust crate。与 pygame 的技术栈（CPython + C 语言 SDL）**从底层起完全
  不同**。
- **为什么像 pygame**：本项目存在的理由，就是把 pygame 经二十余年沉淀的
  **接口风格与心智模型**带到原生 Rust + GPU 的现代栈上——让 pygame
  开发者的经验直接复用、学习成本趋近于零，同时把 pygame 受 C/SDL 制约
  而难做的部分（GPU 渲染、移动端分发、现代扩展模块）做到更好。接口
  命名与组织惯例属于**思想与风格层面**；本项目的全部表达（代码/文档/
  架构）均为原创。
- **名称使用**：文中出现的 "pygame" 仅为**描述性提及**（说明接口风格的
  渊源与兼容意图），不构成商标性使用；pygame 名称与商标归其各自权利人。
- **许可零污染**：本项目 MIT OR Apache-2.0 双协议，与 pygame 的 LGPL
  **无衍生、无链接、无合并关系**——互不传染，可放心商用。
- **致敬**：pygame 二十余年哺育了无数开发者与独立游戏。若本项目对你
  有用，也请支持原版 [pygame](https://www.pygame.org)。

## 层级结构

```
你的游戏（Python，RustPython 执行）
  ↓ import pygame / import pygame_gui / import pygame_physics / …
pygamest 绑定层（本仓）——#[pymodule] 原生模块树（零 ctypes / 零 shim）
  ↓ 帧桥 / 事件桥 / 资源链（SFXE1 加密面）
starfish 引擎（纯 Rust 跨平台 crate 群）——wgpu GPU 渲染 · starfish-we
  窗口与事件 · starfish-pygame 资源加密 · starfish-font / audio / net…
  ↓
平台后端：Windows（DirectX / DirectComposition 透明合成）·
  Web（wasm32 + rAF 帧拍）· Android（arm64，Java 宿主）
```

## 当前项目结构

```
pygamest（本仓）
├── src/rpy/                 # 绑定层核心
│   ├── pygame/              # 23 原生模块（display/event/font/gfxdraw/…
│   │                        #   每模块一个 .rs，强类型强边界）
│   ├── pygame_gui/          # egui 对接（OOP 门面）
│   ├── pygame_physics/      # Rapier 原生词汇物理
│   ├── pygame_spine/        # Spine 2D 骨骼动画轴
│   ├── pygame_ozz/          # ozz 3D 骨骼动画轴
│   ├── pygame_live2d/       # Live2D/Cubism 轴（独立管线 crate 的薄绑定）
│   ├── bridge.rs            # RustPython ↔ 引擎 事件/帧桥
│   └── dependencies/        # RustPython vendored 快照（MIT）
├── examples/                # 契约锚点探针（发版必跑）
├── docs-zh/ docs-en/        # API 参考（双语文档站）
├── reference/               # 设计稿 / 契约 / 三端构建指南
└── android/ resources/      # Android 壳 / CJK 字体等资源
```

## 与 pygame 相比的特色

**① 更多的标准库拓展**——pygame 没有或需三方拼装的：

| 模块 | 能力 |
|---|---|
| `dialog` | 跨平台模态对话框（选文件 / 消息） |
| `video` | 视频输出 |
| `io` | 异步资源链（`fetch`/`load` + 分发形态解密） |
| `net` | TCP/UDP 轮询面 |
| `permission` | 权限面 |

**② 更方便与更好的跨平台**

- Android 打包**不依赖 Linux**（全 Windows 工具链：cargo-ndk + SDK
  aapt2/apksigner 一条命令出 APK）
- **低成本 Rust 库绑定**：接一个 Rust 库 = 写一个 `#[pymodule]` 模块
  文件，零 ctypes 序列化开销
- **更高的可维护性**：强类型 Rust + 每模块独立 .rs + 契约锚点探针
  （发版必跑，130+ 项断言）

**③ GPU 渲染高性能**：wgpu 全 GPU 管线（blit/draw/文字全 GPU），多
后端自适应（Vulkan / DX12 / Metal / GL）

**④ 更多的拓展模块**（feature 轴按需裁剪）：`pygame_gui`（egui 集成）·
`pygame_physics`（Rapier 2D·3D 原生词汇）· `pygame_spine` ·
`pygame_live2d` · `pygame_ozz`（三动画库——spine/ozz 为 crates.io ·
git 依赖的独立适配 crate，零源码维护）

**⑤ 更高级的渲染操作**：自定义 **WGSL 顶点 + 片元双着色器** +
`raw_render` 直绘会话（自管相机/绑组/管线——内置绘制做不了的逐像素
效果，如程序化波纹圆盘，见 pygamest-dev examples/custom_pipeline_demo.py）

## 快速开始（真实案例）

开发用 [pygamest-dev](https://github.com/GuYeying/pygamest-dev) 的解释器
（`python.exe`，pygame 全内置零 Rust 工具链）。最小游戏
（[pygamest-runtime](https://github.com/GuYeying/pygamest-runtime)
/examples/hello_word）：

```python
import pygame

async def main():
    screen = pygame.display.set_mode((800, 600))
    clock  = pygame.time.Clock()

    while True:
        for e in pygame.event.get():
            if e.type == pygame.event.QUIT:
                return
            if e.type == pygame.event.KEYDOWN and e.key == pygame.key.K_ESCAPE:
                return

        with screen.render() as r:          # GPU 渲染会话
            r.fill((125, 125, 125))

        clock.tick(60)
        pygame.display.flip()
        await pygame.next_frame()           # 帧桥：跨平台统一挂起点
```

与 pygame 心智一致，只有三处"新"：

1. 入口严格 `async def main()`（统一跨平台函数入口——Web rAF 模型的硬约束）；
2. 渲染走 **GPU 渲染会话**（`with screen.render() as r:`——blit/draw 全走 wgpu 管线，with语法块来等价实现wgpu命令打包渲染模式）；
3. 帧尾 `await pygame.next_frame()`（帧桥；将来网络 awaitable 同机制扩展）。

## 异步资源链（跨平台 IO 兼容优先）

```python
data = await pygame.io.fetch("assets/hero.png")      # 底层通用口（分发形态含解密）
img  = await pygame.image.load("assets/hero.png")    # 三线统一：load = fetch + 解码
snd  = await pygame.mixer.load("assets/bleep.wav")
fnt  = await pygame.font.load("cjk", 20)             # CJK 预载链（dev 解释器）

img2 = pygame.image.load_from_bytes(data)            # sync 字节构造（无路径场景）
```

`fetch` 读到 `.py` 自动注册 import（分发形态的模块装载链）；无路径场景
（Web 无文件系统）走 `fetch → *_from_bytes`。资源缓存归开发者，生命周期
自己管。Web 边界：音频须首次手势解锁（`pygame.mixer.needs_gesture()`）。

## 模块面（与 pygame 对位 + 扩展）

| 原生模块 | 覆盖 |
|---|---|
| `display` `event` `time`(Clock) `key` `mouse` | 窗口 / 事件轮询 / 计时 / 输入 |
| `image` `font` `mixer` `draw` `gfxdraw` `transform` `mask` `math` `Rect` `Surface` | 资源 / 文本（含 CJK 预载链）/ 图形（六形状×填充描边/半透明）/ 几何 |
| `gamepad` `touch` `sndarray` `sprite` `video` `permission` `version` | 手柄 / 多点触控 / 数组互操作 / 精灵组 / 视频输出 |
| `io` `net` `dialog` | 跨平台读写 / TCP·UDP 轮询面 / 桌面模态 |
| `pygame_gui` | egui 对接（OOP 门面 `UiBase`：`t.draw_ui(panel)` + `egui.event(e)` 显式喂事件 + 嵌套布局 with 语法） |
| `pygame_physics` | Rapier 原生词汇物理（`PhysicsWorld` / `RigidBodyBuilder`，`t.draw_debug` 调试线） |
| `pygame_spine` `pygame_ozz` `pygame_live2d` | 三动画库（2D 骨骼 / 3D 骨骼 / Cubism——路径进对象出，独立 crate 轴） |
| `pygame.next_frame` `pygame.tasks` `pygame.io.payload_list` | 帧桥 / 微调度（spawn·gather·sleep_frames）/ 分发清单 |

UI 与物理示例（pygamest-dev/examples/egui_test.py、rapier_test.py）：

```python
import pygame_gui as egui
import pygame_physics as rp

with screen.render() as r:
    r.draw_ui(panel)                    # egui HUD 恒叠在游戏内容之上
    r.draw_debug(renderer, sink)        # 物理调试线（同一会话排程）
```

## 开发与分发分工

- **开发** = [pygamest-dev](https://github.com/GuYeying/pygamest-dev)：
  pygame 全内置的解释器（`python.exe`），零 Rust 工具链；游戏示例
  （hello_world / draw_test / font_test / egui_test / rapier_test…）在其
  examples/ 下
- **分发** = [pygamest-runtime](https://github.com/GuYeying/pygamest-runtime)：
  玩家单元（入口脚本编译期加密嵌入 + 哈希名散文件 + 加密清单——
  `hello_word.exe + assets/ + modules/` 镜像源结构，双击即玩；
  Android = Java 宿主 + JNI 嵌入面——卓易通实测通过）

## 本仓锚点探针

`examples/` 保留体系**契约锚点**（发版必跑）：

| 探针 | 说明 |
|---|---|
| `sig_probe` | 签名契约锚点（**无头**，先于真机跑；130+ 项断言，终端看 `[OK]`/`FAIL`） |
| `binding_probe` | M1 绑定面验收（标准 pygame：with 会话渲染 + font + mixer + 手柄/触摸查询） |
| `rp_hello` | M0 嵌入冒烟（RP 创世纪 / 语言特性 / 冻结 stdlib） |

## 前置准备

```bash
# Rust 编译目标（Web + Android；桌面无需额外目标）
rustup target add wasm32-unknown-unknown
rustup target add aarch64-linux-android

# Web 胶水工具（版本必须与 Cargo.lock 严格一致）
cargo install wasm-bindgen-cli

# Android 打包工具
cargo install cargo-ndk
```

Android 另需环境变量（未设时脚本按常见路径探测，如 `D:/SDK/Android`）：
`ANDROID_HOME` / `ANDROID_NDK_ROOT` / `ANDROID_JAR`。

## 文档指引

| 文档 | 内容 |
|---|---|
| `CLAUDE.md` | 项目全景 + 架构 + 坑位（接手先读） |
| `docs-zh/` `docs-en/` | API 参考（双语文档站：核心循环/绘制资源/io 网络/扩展命名空间） |
| `docs/1.0-路线图.md` | 测试覆盖矩阵/文档进度/稳定性准则（1.0 = API 稳定承诺） |
| `reference/三端构建与运行指南.md` | 三端构建一站式详细入口 |
| `reference/pygame绑定层API设计稿.md` | 绑定工程唯一契约 |
| `reference/pygame兼容层模块架构.md` | 模块映射 + Tier 排期 |
| `reference/安卓APK打包lib_name坑与真机诊断手册.md` | 安卓问题先读 |
| `reference/wasm编译与运行指南.md` | wasm 编译详解 |

## 生态仓库矩阵

| 仓库 | 角色 |
|---|---|
| [starfish](https://github.com/GuYeying/starfish) | 引擎本体（纯 Rust 跨平台库群：wgpu 渲染 / 窗口事件 / 资源加密面，零 Python 资产） |
| [pygamest](https://github.com/GuYeying/pygamest) | Python 绑定层（**本仓**） |
| [pygamest-dev](https://github.com/GuYeying/pygamest-dev) | 开发解释器（python.exe 形态，零 Rust 工具链，能力轴 feature 裁剪） |
| [pygamest-runtime](https://github.com/GuYeying/pygamest-runtime) | 游戏运行时（玩家单元：入口加密嵌入 + 散文件清单，三平台打包） |
| [pygame_live2d](https://github.com/GuYeying/pygame_live2d) | Live2D/Cubism 独立运行时（mocari vendored + 独立 wgpu 蒙版管线） |
| [pygame_spine](https://github.com/GuYeying/pygame_spine) | Spine 2D 独立适配（chine 纯 Rust 运行时，crates.io 依赖） |
| [pygame_ozz](https://github.com/GuYeying/pygame_ozz) | ozz 3D 骨骼动画独立适配（ozz-animation-rs git 依赖） |
| [egui-starfish-we](https://github.com/GuYeying/egui-starfish-we) | egui × starfish-we 适配（vendored egui-winit fork） |
| [rapier-starfish-debug](https://github.com/GuYeying/rapier-starfish-debug) | Rapier 物理适配 + 通用 debug 渲染 |
| RustPython | vendored 快照内嵌于本仓 `src/rpy/dependencies/`（MIT，零改动+三处 Android 补丁） |

## 协议

MIT OR Apache-2.0 双协议（见 LICENSE-MIT / LICENSE-APACHE）。

> 本软件按"现状"提供，不含任何形式的担保；使用造成的任何损失由使用者
> 自负——完整担保免责与责任限制条款见 LICENSE 文本。本项目为独立实现，
> 与 pygame（LGPL）无衍生关系——详见[与 pygame 的关系](#与-pygame-的关系免侵权声明)。

# pygamest

**次世代 pygame —— Python 游戏开发的原生形态。**

[starfish](https://github.com/GuYeying/starfish) 引擎（wgpu GPU 渲染）的
Python 绑定层：内嵌 [RustPython](https://github.com/RustPython/RustPython)
解释器，`import pygame` 直达 **Rust 原生模块**（`#[pymodule]` 树，零 shim
零 ctypes）——pygame 的心智，GPU 的出生面，三平台一套代码。

- **三平台**：Windows 桌面 / Web（wasm32）/ Android（arm64 真机，鸿蒙
  NEXT + 卓易通容器实测）
- **依赖单向红线**：`pygamest → starfish`（引擎本体零 Python 资产）

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

1. 入口严格 `async def main()`（跨平台统一——Web rAF 模型的硬约束；
   生成器 `yield` 门已退役）；
2. 绘制进 **GPU 渲染会话**（`with screen.render() as r:`——blit/draw 全走
   wgpu 管线，Surface 即 GPU 出生面）；
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
  `hello_word.exe + assets/ + modules/` 镜像源结构，双击即玩）

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
| `reference/三端构建与运行指南.md` | 三端构建一站式详细入口 |
| `reference/pygame绑定层API设计稿.md` | 绑定工程唯一契约 |
| `reference/pygame兼容层模块架构.md` | 模块映射 + Tier 排期 |
| `reference/安卓APK打包lib_name坑与真机诊断手册.md` | 安卓问题先读 |
| `reference/wasm编译与运行指南.md` | wasm 编译详解 |

## 生态仓库矩阵

| 仓库 | 角色 |
|---|---|
| [starfish](https://github.com/GuYeying/starfish) | 引擎本体（纯 Rust 跨平台库，零 Python 资产） |
| [pygamest](https://github.com/GuYeying/pygamest) | Python 绑定层（本仓） |
| [pygamest-dev](https://github.com/GuYeying/pygamest-dev) | 开发解释器（python.exe 形态，零 Rust 工具链） |
| [pygamest-runtime](https://github.com/GuYeying/pygamest-runtime) | 游戏运行时（玩家单元，入口加密嵌入 + 散文件清单） |
| [egui-starfish-we](https://github.com/GuYeying/egui-starfish-we) | egui × starfish-we 适配（vendored egui-winit fork） |
| [rapier-starfish-debug](https://github.com/GuYeying/rapier-starfish-debug) | Rapier 物理适配 + 通用 debug 渲染 |

## 协议

MIT OR Apache-2.0 双协议（见 LICENSE-MIT / LICENSE-APACHE）。

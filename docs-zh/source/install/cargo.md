# cargo 依赖接入

Rust 项目把 pygamest 当 crate 用——引擎 + 绑定 + RustPython 一体。

## 依赖

```toml
[dependencies]
pygamest = "0.3"
```

能力轴（默认全量）：

```toml
[dependencies]
pygamest = { version = "0.3", default-features = false, features = ["gui", "physics2d"] }
```

| 轴 | 内容 |
|---|---|
| gui | egui 即时 GUI（pygame_gui 模块） |
| physics2d / physics3d | rapier 米制物理（pygame_physics） |
| spine / ozz / live2d | 三动画库 |
| stdlib-full | RustPython 标准库全量（默认构建须 nightly 的根源） |

## 最小宿主

引擎自持窗口/事件循环，宿主只需喂入脚本：

```rust
// 独立引擎入口（NativeActivity / Winit 面由 starfish-we 承担）
// 参考 pygamest-runtime：入口编译期加密嵌入 + 散文件清单形态
```

完整生产宿主见 `pygamest-runtime` 仓（Windows / WASM / Android 三端
打包链 + pack.toml 单一配置源）。

## 版本

| 包 | 说明 |
|---|---|
| `pygamest` | 绑定 + RustPython 嵌入 |
| `starfish-pygame` / `starfish-we` / `starfish-render` / `starfish-gfx` | 引擎族（GitHub git 依赖） |
| `pygamest-runtime` | 生产宿主（打包链） |

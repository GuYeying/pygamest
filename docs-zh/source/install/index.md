# 安装

pygamest 的使用形态分四类——按你的目标选择：

```{toctree}
:maxdepth: 1

windows <windows>
android <android>
wasm <wasm>
cargo <cargo>
```

| 形态 | 适合谁 | 产物 |
|---|---|---|
| **Windows 宿主** | 桌面开发调试（最快上手） | `python.exe` 宿主 + examples 案例目录 |
| **Android** | 手机真机测试 | `pygamestdev_demo.apk`（59+ 案例全内置） |
| **WASM** | 浏览器分发 | 静态站（`python_bg.wasm` + 脚本目录） |
| **cargo 依赖** | Rust 项目集成 | `pygamest` crate（crates.io / git） |

所有形态共享同一套 Python 脚本与资产（三端字面一致），写一次到处跑。

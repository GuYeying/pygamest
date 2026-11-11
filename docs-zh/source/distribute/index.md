# 分发你的游戏

```{toctree}
:maxdepth: 1

windows <windows>
wasm <wasm>
android <android>
```

三端打包链统一由打包脚本族承担（配置 = `pack.toml` 单一配置源）：
游戏脚本 + 资产加密/清单化，分发物零明文零 `.py` 后缀。

| 端 | 脚本 | 产物 |
|---|---|---|
| Windows | `scripts/build_windows.py` | `<game>.exe`（入口嵌入 exe）+ 资源目录（哈希名 .sfx） |
| WASM | `scripts/build_wasm.py` | `<game>-web/`（runtime.wasm + 载荷） |
| Android | `android/build.py` | `<game>.apk`（NativeActivity + assets 展开） |

`scripts/build.py --target windows/wasm/android` 总控分派；
`macos/ios/ohos` 为预留插槽（引擎侧平台后端就位，打包适配层待建）。

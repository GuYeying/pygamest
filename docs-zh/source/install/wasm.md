# WASM

## 构建

```bash
rustup target add wasm32-unknown-unknown
python web/build.py                    # host 一次编译（nightly）
cd web && python -m http.server 8124   # 静态服务（file:// 无效）
```

浏览器打开 `http://127.0.0.1:8124/index.html?script=cube_rotate.py`
——页面亦支持选本地 `.py` 单文件直跑。

## 端差异（设计语义）

| 能力 | Web 形态 |
|---|---|
| 同步 io | 显式 `ValueError`（宿主预载 / 异步资源装载承载） |
| 异步资源 | `await pygame.io.read()` 三端统一 |
| net | TCP = WebSocket 同协议同分帧（4 字节大端长度前缀） |
| video | 站桩 `ValueError`（WebCodecs 管线后续批次） |
| 渲染 | WebGPU 自动优先，WebGL2 兜底 |

## 已验证案例

`io_test`（站桩 + 异步通道 fetch E2E）/ `net_test`（TCP-over-WS 回声）/
`spine_wasm_test` / `ozz_wasm_test` / `anim3_wasm_test`（三动画库同屏）/
`cube_rotate` / `mesh_builder_demo` 等——三端一致断言驱动。

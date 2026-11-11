# WASM 打包

```bash
python scripts/build_wasm.py --game <name>
```

## 产物

```
dist/<game>-web/
  runtime.js + runtime_bg.wasm   ← host（wasm-bindgen 胶水）
  _payload.sfx                   ← 加密清单（脚本 + 资源）
  assets/…                       ← 明文豁免资源（视频等）
```

测试：目录内起静态服务（`python -m http.server 8124`），浏览器打开
`index.html`——壳自动 fetch 清单预载（解密）后运行，页面零交互。

## 端语义

- 同步 io 站桩 `ValueError`；异步资源装载（fetch + 帧轮询）三端统一
- net = WebSocket 同协议同分帧
- 渲染 WebGPU 优先 / WebGL2 兜底

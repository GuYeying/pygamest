## Context

IO 异步化定约（跨平台 IO 兼容优先）：三线资源加载统一为 **`load`（async，路径）+ `load_from_bytes`（sync，字节）** 两接口，接口下沉 starfish-pygame 层；SFIX1 容器退役，脚本走 Python import 钩子（finder 查表缓存——模块源码缓存有界且 CPython 同款，保留）；资源数据不进缓存（fetch 字节直返开发者）。

## 统一接口规范（三线一致）

```
await pygame.image.load(path)            → Surface（fetch+解密+解码）
pygame.image.load_from_bytes(bytes)      → Surface（sync 解码）
await pygame.mixer.load(path)            → Sound
pygame.mixer.load_from_bytes(bytes)      → Sound
await pygame.font.load(path, size)       → Font
pygame.font.load_from_bytes(bytes, size) → Font
await pygame.io.fetch(path)              → bytes（底层通用口；.py 自动注册 import）
pygame.io.payload_list()                 → 清单（wasm；native=目录扫描）
```
旧接口处置：`Sound("path")` / `Font(path)` 构造的**路径形态移除**（保留 bytes 形态构造）；`image.load` 由 sync 变 async（breaking，已拍板）

## 批次1：异步 IO 桥 + io.fetch
- `testhost.rs` FRAME_BRIDGE：`_IoOp`（`__await__` yield `("io_read", path)` → Rust send 处理：native `block_on(io::read)` / web `spawn_local(fetch)` → 填槽 → 恢复返回解密字节）；挂 `pygame.io`：`async def fetch(path)`（.py 自动 register_module，复用 modfind.rs:78 register）；`payload_list`
- io 层：删 wasm PRELOAD 表；`read` 的 wasm fetch 保留（fetch 底层）；wasm `read_sync` 恒 Err（文案指路 fetch+from_bytes）；native `read_sync`/sfx 链不变

## 批次2：三线统一接口
- starfish-pygame：image `load`→async（已有 load_async 改名/补齐）、`load_from_bytes` ✓；mixer 加 `SoundData::from_file_async`（io::read().await→from_bytes）+ mixer 层 `load`/`load_from_bytes`；font 加 `load`/`load_from_bytes`（from_bytes 即现 font_build bytes 分支下沉）
- 绑定层：三线 sync `load_from_bytes` pyfunction；async `load` = FRAME_BRIDGE Python `async def`（await _IoOp → 调 sync load_from_bytes）——RP #[pyfunction] 不支持 async，桥模式（next_frame 同款）
- 旧接口移除：mixer/font 绑定的 path 构造分支、image 双 cfg load 合一

## 批次3：SFIX1 退役
- runtime 三平台：脚本 fetch→自动注册→import；删 trailer/SFIX1/pack.rs/read_self_blob/set_pack_source、dev `bin/runtime.rs`
- build 四脚本：payload 加密散文件（保原名）+ _payload 清单；Android 脚本并入 assets 展开通道（io android 后端已持有 AssetManager）；verify_export.py 适配；桌面单文件 exe → 目录分发

## 批次4：三平台回归
- wasm 浏览器 hello_word 全量 0 错误（按需 fetch + async 三线 + import）
- Windows dist 实跑；Android 重建（真机待验）；Linux 脚本就绪；开发模型文档落 CLAUDE/README

## 关键文件
`testhost.rs`、`starfish-io\src\lib.rs`（删 PRELOAD/保 android 后端）、`starfish-io\src\pack.rs`（删）、`starfish-pygame\src\{image,font,mixer}.rs`、`starfish-audio\src\sound_data.rs`、`pygame-rs\src\rpy\{bridge.rs,pygame\{image,font,mixer}.rs,modfind.rs(复用)}`、`pygame-rs-runtime\src\bin\runtime.rs`、`scripts\build_*.py`、`verify_export.py`

## 验证
每批次桌面+wasm 双跑；最终 wasm 浏览器 hello_word 全量 0 错误；Windows dist 实跑；verify_export 全绿
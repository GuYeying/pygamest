# pygame.codec

编解码层（**stdlite 定约 2026-11-24**）——starfish-codec 直通绑定：
hash（sha256）/ zlib（flate2）/ base64（自实现零三方依赖）。
存档指纹、资源校验、体积压缩、文本化传输四场景一次配齐。

## 快速上手

```python
import pygame

# ── sha256：摘要（资源校验 / 存档指纹）──
data = pygame.io.read_bytes("save.dat")
print(pygame.codec.sha256_bytes(data))    # [185, 77, ...] 32 字节
print(pygame.codec.sha256_hex(data))      # 'b94d27...' 64 字符小写 hex

# ── zlib：体积压缩（存档瘦身）──
packed = pygame.codec.zlib_compress(data)
print(len(data), "->", len(packed))       # 压缩比看数据形态
raw = pygame.codec.zlib_decompress(packed)
assert raw == data

# ── base64：字节 ↔ 文本（网络/配置文件文本化传输）──
b64 = pygame.codec.b64_encode(data)
back = pygame.codec.b64_decode(b64)
assert back == data
```

## 典型用法：带校验的存档

```python
def save_game(path, state):
    blob = pygame.codec.zlib_compress(repr(state).encode())
    checksum = pygame.codec.sha256_hex(blob)
    pygame.io.write_text(path, checksum + "\n" + pygame.codec.b64_encode(blob))

def load_game(path):
    head, b64 = pygame.io.read_text(path).split("\n", 1)
    blob = pygame.codec.b64_decode(b64)
    if pygame.codec.sha256_hex(blob) != head:
        raise ValueError("存档校验失败")
    return pygame.codec.zlib_decompress(blob).decode()
```

## API 一览

| 函数 | 说明 |
|---|---|
| `sha256_bytes(data) -> list[int]` | 字节摘要（32 字节） |
| `sha256_hex(data) -> str` | 64 字符小写 hex 摘要 |
| `zlib_compress(data) -> list[int]` | zlib 压缩（默认压缩级） |
| `zlib_decompress(data) -> list[int]` | zlib 解压（损坏 = 空——以空判损） |
| `b64_encode(data) -> str` | 字节 → base64 文本（标准字母表 + padding） |
| `b64_decode(text) -> list[int]` | base64 文本 → 字节（非法字符/padding = ValueError） |

```{note}
📌 **bytes 收发形态**：入参收 bytes/bytearray/memoryview（`try_into_value`
直取）；出参以 `list[int]`（0-255）返回——与本仓 io 读写口
`read_bytes/write_bytes` 同构，存档链零格式转换。zlib 解压损坏返回空表
（调用方以空判损）——不做异常穿透。
```

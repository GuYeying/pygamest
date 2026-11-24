# pygame.parse

结构化数据解析层（**stdlite 定约 2026-11-24**）——starfish-parse 直通绑定：
json / toml 双格式同构 `serde_json::Value` 值模型，绑定层零分派成本。
**界限定约**：数据序列化格式收（存档/配置/网络载荷）；资产格式不收
（建模/贴图等走各自领域模块）。

## py 值模型

双格式统一收发：dict / list / str / int / float / bool / None——
python 值树 ↔ Rust 值树直转换，无中间字符串编组。其他类型
（bytes/对象等）= ValueError 带类型名。

## 快速上手

```python
import pygame

# ── json：文本 ↔ 值树 ──
cfg = pygame.parse.loads('{"width": 800, "vsync": true, "title": "demo"}')
print(cfg["width"], cfg["vsync"])         # 800 True

text = pygame.parse.dumps({"score": 100, "items": ["sword", "shield"]})
print(text)                               # {"score":100,"items":["sword","shield"]}（紧凑）

pretty = pygame.parse.dump_pretty(cfg)    # 缩进美化（调试/人工编辑）

# ── toml：配置文件形态 ──
settings = pygame.parse.loads_toml('''
[window]
width = 800
height = 600
fullscreen = false
''')
print(settings["window"]["width"])        # 800

toml_text = pygame.parse.dumps_toml({"window": {"width": 800}})
```

## 典型用法：配置加载 + 存档

```python
def load_config(path):
    return pygame.parse.loads_toml(pygame.io.read_text(path))

def save_game(path, state):
    pygame.io.write_text(path, pygame.parse.dumps(state))

def load_game(path):
    return pygame.parse.loads(pygame.io.read_text(path))
```

## API 一览

| 函数 | 说明 |
|---|---|
| `loads(text) -> 值树` | json 文本 → py 值（解析失败 ValueError） |
| `dumps(obj) -> str` | py 值 → json 文本（紧凑） |
| `dump_pretty(obj) -> str` | py 值 → json 文本（缩进美化） |
| `loads_toml(text) -> 值树` | toml 文本 → py 值 |
| `dumps_toml(obj) -> str` | py 值 → toml 文本 |

```{note}
📌 **float 精度界**：json 不支持 NaN/inf——写非有限 float = ValueError
（toml 同）。dict 键须为 str。**超大 int 降级**：超出 i64 的 int 静默转
f64（精度损失——存档关键值勿依赖 2^53 以上整数）。
```

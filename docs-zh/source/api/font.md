# pygame.font

字体与文字渲染模块。加载 TTF 字体，把文字渲染为可绘制的 Surface。

文字表面经 `pygame.gfxdraw.draw_surface()` 放置到画布上（中心锚，
`transform=` 摆位；不写 `transform` 则居中、用 `dst_rect=` 则轴对齐）；
也可用 `pygame.font.draw_text()` 一步完成渲染与放置。

```python
font = await pygame.font.load("zhao.ttf", 24)
surf = font.render("得分 1200", True, (255, 220, 0))
pygame.gfxdraw.draw_surface(t, surf,
                            transform=pygame.math.mat4_translation(0, 200, 0))
```

注意：按名字查找字体未命中时，会使用内嵌默认字体继续运行（默认字体
只含英文字形，中文将显示为方块）。设置环境变量
`STARFISH_STRICT_ASSETS=1` 可改为直接抛出异常。

## <span class="c-func">函数</span>

#### <span class="c-func">load(path, size) -> Font</span> <span class="tag async">async</span>

按路径加载字体（**await 形态**；全平台同语义）。

```python
async def main():
    font = await pygame.font.load("zhao.ttf", 24)
```

- :rtype: Font

#### <span class="c-func">load_from_bytes(bytes, size) -> Font</span>

从 TTF 字节构造字体。

- :param bytes: TTF 原始字节
- :type bytes: bytes
- :param size: 字号（像素）
- :type size: int
- :rtype: Font

#### <span class="c-func">SysFont(name, size) -> Font</span>

按名字查找系统字体。当前实现统一使用内嵌默认字体（占位形态）。

- :rtype: Font

#### <span class="c-func">draw_text(f, font, text, model, color)</span>

渲染文字并按显式矩阵放置——一步完成。常用于三维场景中的文字牌
（透视相机下自动跟随）或需要仿射变换的二维文字。文字以**中心锚**
放置（局部原点为文字面中心）。

```python
# 3D 世界中的玩家 ID 牌（透视相机下自动跟随）
m = (pygame.math.mat4_translation(0, 1.5, -3)
     * pygame.math.mat4_scale(0.005, 0.005, 1))
pygame.font.draw_text(f, font, "ID: player_1", m, (120, 255, 160))
```

- :param f: 渲染会话（`with screen.render() as f` 的 `f`）
- :param font: 字体对象
- :type font: Font
- :param text: 渲染文本
- :type text: str
- :param model: 模型矩阵（Mat4 经 `pygame.math.mat4_*` 组合）
- :type model: Mat4
- :param color: 文字颜色
- :type color: tuple[int, int, int] | tuple[int, int, int, int]

## <span class="c-class">类</span>

### <span class="c-class">Font —— 字体</span>

内含字形缓存（单线程）。

#### <span class="c-func">Font(font, size)</span>

构造字体。

```python
font = pygame.font.Font(None, 32)          # 内嵌默认字体
font = pygame.font.Font("zhao.ttf", 24)    # 路径（Web 需宿主预载）
font = pygame.font.Font(ttf_bytes, 24)     # bytes
```

- :param font: `None`（默认字体）/ 名字 / 路径 / `bytes` 四形态
- :type font: str | bytes | None
- :param size: 字号
- :type size: int

##### <span class="c-func">size(text) -> tuple[int, int]</span>

排版度量：`(宽, 高)`——宽 = 字形推进和（向上取整），高 = 行高；
换行符按单行语义跳过。

- :rtype: tuple[int, int]

##### <span class="c-func">render(text, antialias=True, color=(255, 255, 255)) -> Surface</span>

文字表面——返回独立 `Surface`（中心锚，局部原点 = 面中心），经
`pygame.gfxdraw.draw_surface(f, surf, transform=...)` 放置（`transform`
不收时按原点居中直绘）。

```python
surf = font.render("得分 1200", True, (255, 220, 0))
pygame.gfxdraw.draw_surface(f, surf,
                            transform=pygame.math.mat4_translation(0, 200, 0))
```

- :param text: 渲染文本
- :type text: str
- :param antialias: 抗锯齿（默认开）
- :type antialias: bool
- :param color: 文字颜色
- :type color: tuple[int, int, int] | tuple[int, int, int, int]
- :returns: 文字表面（中心锚——局部原点 = 面）
- :rtype: Surface

##### <span class="c-attr">line_height</span>

行高（像素）。

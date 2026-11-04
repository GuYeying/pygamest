# pygame.font

字体与文字渲染——TTF 解码 + 文字表面一等化（`render()` 产物可 blit /
可 transform）。

```{note}
[差异] **缺资源不崩溃**：按名字查字体未命中（未预载且文件不存在）时
落内嵌默认字体继续（纯英文字形——中文将显示豆腐块 = 可视化告警，
日志同步记因）；`STARFISH_STRICT_ASSETS=1` 恢复真异常。
```

## <span class="c-func">函数</span>

#### <span class="c-func">load(path, size) -> Font</span> <span class="tag async">async</span>

按路径加载字体（**await 形态**；全平台同语义）。

```python
async def main():
    font = await pygame.font.load("zhao.ttf", 24)
```

- :rtype: Font

#### <span class="c-func">load_from_bytes(bytes, size) -> Font</span>

字节构造字体（三线同规范）。

- :param bytes: TTF 原始字节
- :param size: 字号（像素）
- :rtype: Font

#### <span class="c-func">SysFont(name, size) -> Font</span>

系统字体查找形态。**[差异]** 查找未实现——一律用内嵌默认字体
（pygame 常用形态的占位语义）。

- :rtype: Font

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
- :param size: 字号

##### <span class="c-func">size(text) -> tuple[int, int]</span>

排版度量：`(宽, 高)`——宽 = 字形推进和（向上取整），高 = 行高；
换行符按单行语义跳过。

- :rtype: tuple[int, int]

##### <span class="c-func">render(text, antialias=True, color=(255, 255, 255)) -> Surface</span>

文字表面——返回独立 `Surface`，可 blit / 可 transform。

```python
surf = font.render("得分 1200", True, (255, 220, 0))
```

- :param text: 渲染文本
- :param antialias: 抗锯齿（默认开）
- :param color: 文字颜色
- :returns: 文字表面
- :rtype: Surface

##### <span class="c-attr">line_height</span>

行高（像素）。

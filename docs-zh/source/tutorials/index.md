# 快速上手

```{toctree}
:maxdepth: 1

hello <hello>
core-concepts <core-concepts>
gfx-3d <gfx-3d>
```

三步跑起第一个程序（Windows 宿主）：

```bash
cd pygamest-dev/examples
..\target\release\python.exe hello_world.py
```

`hello_world.py` 全文：

```python
import pygame


async def main():
    screen = pygame.display.set_mode((800, 600))
    while True:
        for e in pygame.event.get():
            if e.type == pygame.event.QUIT:
                return
        with screen.render() as t:
            t.fill((30, 30, 46))
        pygame.display.flip()
        await pygame.next_frame()
```

四个要素：`set_mode` 开窗 → `with screen.render()` 会话（绘制词汇按
管线分域：图元 [pygame.gfxdraw](../api/gfxdraw.md)、文字
[pygame.font](../api/font.md)）→ `flip()` 落屏 → `await
pygame.next_frame()` 帧拍。

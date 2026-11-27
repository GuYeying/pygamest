# 第一个程序：逐行解析

`hello_world.py` 全文（四要素）：

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
        await pygame.tasks.yield_now()
```

## 四要素

1. **`async def main()`**——入口恒为异步（跨端统一：Web rAF 模型的
   硬要求；三端同一脚本的前提）。
2. **`set_mode`**——开窗并返回 Screen（逻辑尺寸；letterbox 由引擎
   相机内部处理）。
3. **`with screen.render() as t:`**——本帧主渲染通道。会话对象只表达
   "画到哪"；绘制词汇按管线分域：图元 → `pygame.gfxdraw`（renderpass
   作首参），文字 → `pygame.font`。
4. **`flip()` + `await pygame.tasks.yield_now()`**——落屏 + 帧拍（事件泵
   与渲染节拍的统一入口）。

## 加一点内容

```python
        with screen.render() as t:
            t.fill((30, 30, 46))
            pygame.gfxdraw.draw_batch(                      # 中心系原点画圆
                t, pygame.gfxdraw.gen_circle(80),
                color=(120, 220, 120, 255))
            hud = pygame.font.Font(None, 20).render("hello", True, (255, 255, 255))
            pygame.gfxdraw.draw_surface(
                t, hud, transform=pygame.math.mat4_translation(-380, 270, 0))
```

坐标系 = **中心原点、y 向上**（与 3D 透视、物理米制系三方同构）。
颜色是**逐次绘制的参数**（`color=(r, g, b, a)`）——没有 `set_color`
状态机了。

**坐标域要先声明**：进入循环前调一次
`screen.set_camera(pygame.render.game_camera_ortho(800, 600, 800, 600))`。
不设相机 = 裸 NDC 直通（`[-1, 1]` 单位方框），上面 `gen_circle(80)`
会整屏铺满。

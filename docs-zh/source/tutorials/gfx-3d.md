# gfxdraw 3D 词汇

gfxdraw 除 2D 图元外直通 **shape3d 五件套 + 平面多边形 3D**——配合
`render_depth` 深度会话与透视相机，无需自建管线即可摆 3D 场景。

## 最小场景

```python
import pygame

M = pygame.math


async def main():
    screen = pygame.display.set_mode((800, 600))
    cam = pygame.render.camera_perspective(55.0, 800 / 600, 0.1, 100.0)
    cam = cam.with_view(M.mat4_translation(0.0, 1.6, -7.0))

    while True:
        with screen.render_depth() as rt:
            rt.fill((10, 11, 18))
            pygame.gfxdraw.set_camera(rt, cam)

            pygame.gfxdraw.set_color(rt, (70, 74, 92))
            pygame.gfxdraw.plane(rt, [0, -1, 0], [1, 0, 0], [0, 0, 1], [8, 8])
            pygame.gfxdraw.set_color(rt, (60, 120, 200))
            pygame.gfxdraw.cube(rt, [-0.6, -0.6, -0.6], [0.6, 0.6, 0.6])
            pygame.gfxdraw.set_color(rt, (200, 90, 60))
            pygame.gfxdraw.sphere(rt, [-2.6, 0.0, 0.5], 0.8)

            pygame.gfxdraw.clear_camera(rt)   # 摘透视 → 回目标尺寸正交
            # ...2D HUD（正交像素坐标直用）...

        pygame.display.flip()
        await pygame.next_frame()
```

## 词汇一览

| 函数 | 形状 |
|---|---|
| `cube(f, min, max)` | 立方体（对角点） |
| `sphere(f, center, radius)` | 球（经纬分段） |
| `plane(f, center, right, up, size)` | 平面（基向量定朝向） |
| `cylinder(f, base_center, radius, height)` | 圆柱（+y 挤出） |
| `cone(f, base_center, radius, height)` | 圆锥（+y 顶点） |
| `polygon3d(f, points)` | 平面多边形 3D（flat [x,y,z,...]——Newell 平面基三角化） |
| `capsule(f, ax, ay, bx, by, radius)` | 2D 胶囊（凸包填充） |
| `polyline(f, points, closed)` | 折线（1px） |
| `regular_polygon(f, cx, cy, radius, sides)` | 正多边形（填充） |

要点：角度制参数（`quat_from_axis_angle` / 相机 fov 同款度数约定）；
颜色走 [one-shot set_color 状态机](core-concepts.md)；完整案例
`gfx3d_demo.py`。

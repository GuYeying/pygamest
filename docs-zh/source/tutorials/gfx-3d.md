# gfxdraw 3D 几何

3D 与 2D 走**同一条通道**：`gen_*` 生成 `VertexBuf` → `draw_batch` 提交。
配合 `render_depth` 深度会话与透视相机，无需自建管线即可摆 3D 场景
（旧的 `cube` / `sphere` / `plane` / `cylinder` / `cone` / `polygon3d`
直调口已全部删除）。

## 最小场景

```python
import pygame

M = pygame.math


async def main():
    screen = pygame.display.set_mode((800, 600))
    # 透视相机（GameCamera：逻辑/物理尺寸 + fov 度数制），机位 = 视图矩阵
    cam = pygame.render.game_camera_perspective(
        55.0, 800, 600, 800, 600, 0.1, 100.0)
    cam = cam.with_view(M.mat4_translation(0.0, 1.6, -7.0))

    # 几何只生成一次（VertexBuf 可复用，逐帧只提交）
    floor_buf = pygame.gfxdraw.gen_plane(
        [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [8.0, 8.0])
    cube_buf = pygame.gfxdraw.gen_box([-0.6, -0.6, -0.6], [0.6, 0.6, 0.6])
    sphere_buf = pygame.gfxdraw.gen_sphere(0.8)
    t_sphere = M.mat4_translation(-2.6, 0.0, 0.5)

    while True:
        with screen.render_depth() as rt:
            rt.fill((10, 11, 18))
            pygame.gfxdraw.set_camera(rt, cam)

            pygame.gfxdraw.draw_batch(rt, floor_buf, color=(70, 74, 92, 255))
            pygame.gfxdraw.draw_batch(rt, cube_buf, color=(60, 120, 200, 255),
                                      cull="back")
            pygame.gfxdraw.draw_batch(rt, sphere_buf, transform=t_sphere,
                                      color=(200, 90, 60, 255), cull="back")

            pygame.gfxdraw.clear_camera(rt)   # 摘透视 → 回目标尺寸像素正交
            # ...2D HUD（中心系像素坐标直用）...

        pygame.display.flip()
        await pygame.tasks.yield_now()
```

## 生成器一览

| 生成器 | 形状 |
|---|---|
| `gen_box(min, max)` / `gen_cube(size)` | 轴对齐盒（对角点）/ 边长立方体（中心原点） |
| `gen_sphere(radius, su=16, sv=12)` | 球（经纬分段，中心原点） |
| `gen_plane(center, right, up, size)` | 平面（基向量定朝向，`size=[w, h]`） |
| `gen_cylinder(radius, height, segments=24)` | 圆柱（原点 = 柱高中点，沿 +y 挤出） |
| `gen_cone(radius, height, segments=24)` | 圆锥（原点 = 柱高中点，+y 尖端） |
| `gen_polygon3d(points)` | 平面多边形（扁平 `[x,y,z,...]`——Newell 平面基三角化） |

摆位全靠 `draw_batch(transform=Mat4)`（球/柱/锥没有 `center` 参数——
用 `mat4_translation` 补位）。闭合体统一 **CCW 外向绕序**，`cull="back"`
剔背向面（内容层按 Ccw = 正面裁决）。

要点：角度制参数（`quat_from_axis_angle` 收度数、相机 fov 同款约定）；
颜色/线框/混合/剔除全部是 [逐次绘制参数](core-concepts.md)；完整案例
`gfx3d_demo.py`。

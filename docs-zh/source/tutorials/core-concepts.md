# 核心概念

## 会话 = 目标表达

`with screen.render() as t:` 开启本帧主渲染通道——**会话对象只表达
"画到哪"**，不承载绘制词汇：图元/贴图/状态机 →
[pygame.gfxdraw](../api/gfxdraw.md)（renderpass 作首参）；文字 →
pygame.font。会话自有的低层口：`fill` / `quad` / `vertices` /
`blit_area` / `raw_render` / `raw_compute` / `draw_ui`。

## 颜色 / 线框（逐次参数）

没有颜色状态机——每个绘制调用自带 `color=(r, g, b, a)`，不写就用会话
默认色。形状一律 `gen_*` 生成 → `draw_batch` 提交：

```python
pygame.gfxdraw.draw_batch(
    t, pygame.gfxdraw.gen_circle(22), color=(255, 200, 0, 255),
    transform=pygame.math.mat4_translation(150, 30, 0))
pygame.gfxdraw.draw_batch(
    t, pygame.gfxdraw.gen_box([-1, -1, -1], [1, 1, 1]), color=(90, 120, 220, 255),
    cull="back")
```

`wireframe=True` / `blend="additive"` 同一个位置给——都是一次性的。

## 相机三层级

| 层级 | 入口 | 说明 |
|---|---|---|
| 全局显式 | `screen.set_camera(GameCamera)` | 全局唯一相机；物理面对账（跨设备可移植） |
| 段级 | `pygame.gfxdraw.set_camera(f, GameCamera)` | 会话内分段；`clear_camera` 清除 |

两层都收 `GameCamera`（letterbox 内聚——MVP / 坐标映射 / 裁剪同一
事实源）。**不设相机 = 裸 NDC 直通**（`[-1, 1]`），像素坐标要显式
`game_camera_ortho(lw, lh, lw, lh)`。

## 异步资源三线

文件/网络/字体三类资源统一异步形态（引擎 io 通道 + 帧轮询，主循环
不卡）：

```python
img = await pygame.image.load("assets/hero.png")
snd = await pygame.mixer.load("assets/boom.wav")
fnt = await pygame.font.load("cjk", 17)
```

同步口 `load_from_bytes` 同步形态三端可用（Web 端同步 io 显式
`ValueError`——宿主预载承载）。

## 深度会话（3D 内容）

`with screen.render_depth() as rt:` ——绘制走深度测试管线（深度缓冲
随 base 帧清 1.0）。配合透视相机 + [gfxdraw 3D
生成器](gfx-3d.md)（`gen_box` / `gen_sphere` / `gen_plane` /
`gen_cylinder` / `gen_cone` / `gen_polygon3d`，一律 `draw_batch` 提交）。

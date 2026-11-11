# 核心概念

## 会话 = 目标表达

`with screen.render() as t:` 开启本帧主渲染通道——**会话对象只表达
"画到哪"**，不承载绘制词汇：图元/贴图/状态机 →
[pygame.gfxdraw](../api/gfxdraw.md)（renderpass 作首参）；文字 →
pygame.font。会话自有的低层口：`fill` / `quad` / `vertices` /
`raw_render` / `raw_compute` / `draw_ui`。

## 颜色 / 线框状态机（one-shot）

图元函数不收颜色参数——`pygame.gfxdraw.set_color(f, color)` 单独设置，
**只对下一个图元生效**，随后自动重置回默认紫色（忘 set 一眼可辨）。
逐形状配对是正解用法：

```python
pygame.gfxdraw.set_color(t, (255, 200, 0))
pygame.gfxdraw.filled_circle(t, 150, 30, 22)
pygame.gfxdraw.set_color(t, (90, 120, 220))
pygame.gfxdraw.cube(t, [-1, -1, -1], [1, 1, 1])
```

## 相机三层级

| 层级 | 入口 | 说明 |
|---|---|---|
| 全局 Auto | `pygame.render.init(dim=3, fov=...)` → `set_mode` | wish 驱动，letterbox 内聚 |
| 全局显式 | `screen.set_camera(GameCamera)` | 覆盖 Auto；物理面对账（跨设备可移植） |
| 段级 | `pygame.gfxdraw.set_camera(f, Camera)` | 会话内分段；`clear_camera` 清除 |

`screen.set_camera` 收 `GameCamera`（letterbox 内聚——MVP / 坐标映射 /
裁剪同一事实源）；段级收 `Camera`（像素空间投影）。

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
词汇](gfx-3d.md)（cube/sphere/plane/cylinder/cone/polygon3d）。

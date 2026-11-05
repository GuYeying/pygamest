# pygame.gfxdraw

SDL_gfx 风格图元 + **渲染统一 draw 面**（相机/变换状态机 + 贴图通道）——
所有绘制经渲染会话（renderpass 作首参），CPU 烘矩阵（集成管线自动
letterbox）。

```{note}
[差异] 首参 `f` 是**渲染会话**（`with screen.render() as f` 的 `f`，
或 Surface 会话）。尾参颜色统一为**单个颜色值** `(r, g, b)` /
`(r, g, b, a)` / `pygame.Color`（非真 pygame 的三个独立 int）。

[坐标系] **中心原点、y 向上**——`(0, 0)` = 屏幕/画布中心，`+y` 向上
（与 3D 透视 / 物理米制系三方同构）。

[旧口退役] 会话对象上的 `r.rect` / `r.circle` / `r.line` 等绘制便捷
方法与 `blit` 家族已全部剔除——图元走本模块，贴图走 `draw_surface` 系。
```

## <span class="c-func">状态机（相机 / 变换）</span>

renderpass 作首参——与 SDL_gfx 签名同构；render 会话保持纯通道。

#### <span class="c-func">set_camera(f, camera)</span>

会话相机切换（后续 gfx 绘制进新段；仅本会话生效）。

- :param f: 渲染会话（`with screen.render() as f` 的 `f`）
- :param camera: 相机（`pygame.render.camera_ortho` / `camera_perspective` 产物）
- :type camera: Camera

#### <span class="c-func">reset_camera(f)</span>

回 Auto 兜底相机（按目标尺寸现算）。

#### <span class="c-func">set_transform(f, model)</span>

会话变换——后续绘制的顶点按此矩阵 **CPU 烘**（2D 仿射 / 3D 模型矩阵
统一入口；集成管线 pack 时自动乘 letterbox，矩阵全权）。

**one-shot**：变换只作用于紧随的**下一次绘制**，消费后自动回恒等
（物体级变换零泄漏——忘 set 不会复用上一物体的矩阵）。camera 保持
持续（视角级语义——一个会话一个视角，多视角靠 `set_camera` 分段）。

- :param f: 渲染会话
- :param model: 模型矩阵（Mat4 经 `pygame.math.mat4_*` 组合）
- :type model: Mat4

```python
# 旋转 30° 围绕物体中心（中心锚——局部原点 = 物体中心）
m = (pygame.math.mat4_translation(100, 50, 0)
     .mul_mat4(pygame.math.mat4_from_quat(
         pygame.math.quat_from_axis_angle(0, 0, 1, 30))))
pygame.gfxdraw.set_transform(f, m)
pygame.gfxdraw.draw_surface(f, img)
```

#### <span class="c-func">reset_transform(f)</span>

回恒等变换。

## <span class="c-func">贴图通道（draw_surface 系）</span>

#### <span class="c-func">draw_surface(f, src)</span>

画一张离屏面——**中心锚**（局部原点 = 物体中心）：位置/旋转/缩放/
镜像全由 `set_transform` 的模型矩阵承载。

- :param f: 渲染会话
- :param src: 源面（`pygame.Surface`）

#### <span class="c-func">draw_surface_centered(f, src, center)</span>

中心锚画面（quad 中心对位 `center`；仍按会话 transform 复合变换）。

- :param center: `(x, y)` 中心系坐标
- :type center: tuple[int, int]

#### <span class="c-func">draw_surface_uv(f, src, uv, tint=None)</span>

子区域画面（area 语义继任）——src 的 uv 子矩形贴为等像素尺寸 quad。

- :param uv: `[u0, v0, u1, v1]` 归一 0..1
- :type uv: list[float]

#### <span class="c-func">draw_surface_tf(f, src, mat, tint=None)</span>

显式矩阵画面——mat 全权（不复合会话 transform）。

- :param mat: 模型矩阵
- :type mat: Mat4

## <span class="c-func">点 / 线</span>

#### <span class="c-func">pixel(f, x, y, color)</span>

画单个像素。

#### <span class="c-func">hline(f, x1, x2, y, color)</span>

水平线。

#### <span class="c-func">vline(f, x, y1, y2, color)</span>

垂直线。

#### <span class="c-func">line(f, x1, y1, x2, y2, color)</span>

线段（宽度 1）。

#### <span class="c-func">aaline(f, x1, y1, x2, y2, color)</span>

= `line` 兼容别名。

## <span class="c-func">圆 / 椭圆</span>

#### <span class="c-func">circle(f, x, y, radius, color)</span>

1px 描边圆（圆心 + 半径；负半径钳 0）。

#### <span class="c-func">aacircle(f, x, y, radius, color)</span>

= `circle` 兼容别名。

#### <span class="c-func">filled_circle(f, x, y, radius, color)</span>

填充圆。**注意**：半径为 **i32 整数世界单位**——透视空间里做不了
小物体（1 单位分辨率），3D 小物体用 `draw_surface`。

#### <span class="c-func">ellipse(f, x, y, rx, ry, color)</span>

1px 描边椭圆（中心 + 半径轴）。

#### <span class="c-func">aaellipse(f, x, y, rx, ry, color)</span>

= `ellipse` 兼容别名。

#### <span class="c-func">filled_ellipse(f, x, y, rx, ry, color)</span>

填充椭圆。

## <span class="c-func">弧</span>

#### <span class="c-func">arc(f, x, y, radius, start_angle, stop_angle, color)</span>

1px 描边圆弧。**[差异]** 角度**度数制**（SDL_gfx 惯例）；v1 仅圆弧
（无椭圆弧 / 填充弧）。

## <span class="c-func">三角 / 多边形</span>

#### <span class="c-func">trigon(f, x1, y1, x2, y2, x3, y3, color)</span>

描边三角。

#### <span class="c-func">aatrigon(f, x1, y1, x2, y2, x3, y3, color)</span>

= `trigon` 兼容别名。

#### <span class="c-func">filled_trigon(f, x1, y1, x2, y2, x3, y3, color)</span>

填充三角。

#### <span class="c-func">polygon(f, points, color)</span>

1px 描边多边形——`points = [(x, y), ...]`。

#### <span class="c-func">aapolygon(f, points, color)</span>

= `polygon` 兼容别名。

#### <span class="c-func">filled_polygon(f, points, color)</span>

填充多边形。

## <span class="c-func">矩形</span>

`rect` 形态统一 `(left, top, w, h)` 序列。

#### <span class="c-func">rectangle(f, rect, color)</span>

1px 描边矩形。

#### <span class="c-func">box(f, rect, color)</span>

填充矩形。

#### <span class="c-func">rounded_rectangle(f, rect, radius, color)</span>

**[垫底]** 圆角 v1 未建——`radius` 忽略，降级描边直角矩形。

#### <span class="c-func">rounded_box(f, rect, radius, color)</span>

**[垫底]** 同上，降级填充矩形。

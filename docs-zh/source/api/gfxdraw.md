# pygame.gfxdraw

SDL_gfx 风格图元 + **渲染统一 draw 面**——图元函数**只收 renderpass + 几何参数**；颜色/线框经独立状态机单独设置（one-shot，draw 后自动重置）。

```{note}
[差异] 首参 `f` 是**渲染会话**（`with screen.render() as f` 的 `f`，
或 Surface 会话）。

[坐标系] **中心原点、y 向上**——`(0, 0)` = 屏幕/画布中心，`+y` 向上
（与 3D 透视 / 物理米制系三方同构）。

[颜色状态机] 图元函数**不收颜色参数**——颜色经 `set_color(f, color)`
单独设置（one-shot）；不设 = **默认紫色**（调试可辨——忘了设色一眼看出）；draw 后自动重置回紫色。

[线框状态机] `set_wireframe(f)` 开启三角网格边线渲染（one-shot）——
呈现原始三角化的**边**（圆的扇形边、耳切对角线都直接可见），
"是什么形式就是什么形式"。

[旧口退役] 会话对象上的 `r.rect` / `r.circle` 等绘制便捷方法与 `blit`
家族已全部剔除——图元走本模块，贴图走 `draw_surface` 系。
```

## <span class="c-func">状态机</span>

#### <span class="c-func">set_color(f, color)</span>

**one-shot 绘制色**——下一次图元 draw 用此色，随后自动重置回默认紫色。

```python
pygame.gfxdraw.set_color(f, (255, 0, 0))
pygame.gfxdraw.filled_circle(f, 0, 0, 50)     # → 红色
pygame.gfxdraw.filled_circle(f, 100, 0, 50)   # → 紫色（默认）
```

- :param f: 渲染会话
- :param color: `(r, g, b)` / `(r, g, b, a)` 纯色
- :type color: tuple[int, int, int] | tuple[int, int, int, int]

#### <span class="c-func">set_wireframe(f, enabled=True)</span>

**one-shot 线框模式**——下一次图元 draw 渲染为三角网格边线（三角形 →
三条边），随后自动重置回填充面。

```python
pygame.gfxdraw.set_wireframe(f)
pygame.gfxdraw.filled_circle(f, 0, 0, 50)  # → 网格线框（三角边可见）
pygame.gfxdraw.filled_circle(f, 100, 0, 50)  # → 填充面（默认）
```

- :param f: 渲染会话
- :param enabled: `True`（默认）/ `False`
- :type enabled: bool

#### <span class="c-func">set_camera(f, camera)</span>

会话相机切换（后续 gfx 绘制进新段；仅本会话生效）。

- :param f: 渲染会话
- :param camera: 相机（`pygame.render.camera_ortho` / `camera_perspective` 产物）
- :type camera: Camera

#### <span class="c-func">reset_camera(f)</span>

回 Auto 兜底相机。

#### <span class="c-func">set_transform(f, model)</span>

会话变换（one-shot——下一次绘制的顶点按此矩阵 CPU 烘）。

- :param model: 模型矩阵（Mat4）
- :type model: Mat4

#### <span class="c-func">reset_transform(f)</span>

回恒等变换。

## <span class="c-func">贴图通道（draw_surface 系）</span>

#### <span class="c-func">draw_surface(f, src)</span>

画一张离屏面——**中心锚**（局部原点 = 物体中心）。位置/旋转/缩放/
镜像全由 `set_transform` 的模型矩阵承载。

#### <span class="c-func">draw_surface_centered(f, src, center)</span>

中心锚画面（quad 中心对位 `center`）。

#### <span class="c-func">draw_surface_uv(f, src, uv, tint=None)</span>

子区域画面。

#### <span class="c-func">draw_surface_tf(f, src, mat, tint=None)</span>

显式矩阵画面（mat 全权，不复合会话 transform）。

## <span class="c-func">图元（纯几何参数，无色参）</span>

### 点 / 线

#### <span class="c-func">pixel(f, x, y)</span>
#### <span class="c-func">hline(f, x1, x2, y)</span>
#### <span class="c-func">vline(f, x, y1, y2)</span>
#### <span class="c-func">line(f, x1, y1, x2, y2)</span>
#### <span class="c-func">aaline(f, x1, y1, x2, y2)</span>

= `line` 兼容别名。

### 圆 / 椭圆

#### <span class="c-func">circle(f, x, y, radius)</span>
#### <span class="c-func">aacircle(f, x, y, radius)</span>

= `circle` 兼容别名。

#### <span class="c-func">filled_circle(f, x, y, radius)</span>

**注意**：半径为 i32 整数世界单位——透视空间小物体用 `draw_surface`。

#### <span class="c-func">ellipse(f, x, y, rx, ry)</span>
#### <span class="c-func">aaellipse(f, x, y, rx, ry)</span>
#### <span class="c-func">filled_ellipse(f, x, y, rx, ry)</span>

### 弧

#### <span class="c-func">arc(f, x, y, radius, start_angle, stop_angle)</span>

度数制（SDL_gfx 惯例）；v1 仅圆弧。

### 三角 / 多边形

#### <span class="c-func">trigon(f, x1, y1, x2, y2, x3, y3)</span>
#### <span class="c-func">aatrigon(f, x1, y1, x2, y2, x3, y3)</span>
#### <span class="c-func">filled_trigon(f, x1, y1, x2, y2, x3, y3)</span>
#### <span class="c-func">polygon(f, points)</span>
#### <span class="c-func">aapolygon(f, points)</span>
#### <span class="c-func">filled_polygon(f, points)</span>

### 矩形

#### <span class="c-func">rectangle(f, rect)</span>
#### <span class="c-func">box(f, rect)</span>
#### <span class="c-func">rounded_rectangle(f, rect, radius)</span>

**[垫底]** 圆角 v1 未建——降级直角。

#### <span class="c-func">rounded_box(f, rect, radius)</span>

**[垫底]** 同上。

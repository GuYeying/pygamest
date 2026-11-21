# pygame.gfxdraw

SDL_gfx 风格图元 + **渲染统一 draw 面**——图元函数**只收 renderpass + 几何参数**；颜色/线框经独立状态机单独设置（one-shot，draw 后自动重置）。

另有 **push 统一口**（见下文 [push 统一口](#push-统一口)）——手写顶点数据
的通用出口：`VertexBuf` 顶点容器 + `set_topology` 拓扑状态 + `push`
入批，数据全程在 Rust 侧流转。

```{note}
首参 `f` 是**渲染会话**（`with screen.render() as f` 的 `f`，
或 Surface 会话）。

[坐标系] **中心原点、y 向上**——`(0, 0)` = 屏幕/画布中心，`+y` 向上
（与 3D 透视 / 物理米制系三方同构）。

[颜色状态机] 图元**不收颜色参数**——颜色经 `set_color(f, color)` 单独
设置。**双语义分立**：push 流 = sticky（色/拓扑/变换跨 push 持续）；
不设 = **默认紫色**（调试可辨）。

[图元函数族退役] SDL_gfx 风格图元函数（`filled_circle` / `box` /
`line` / `rectangle` 等直接绘制口）已全部剔除——形状统一走
`gen_*` 生成器 + `push` 统一口（数据面收敛，见下节）。

[线框状态机] `set_wireframe(f)` 开启三角网格边线渲染（one-shot）——
呈现原始三角化的**边**（圆的扇形边、耳切对角线都直接可见），
"是什么形式就是什么形式"。

[旧口退役] 会话对象上的 `r.rect` / `r.circle` 等绘制便捷方法与 `blit`
家族已全部剔除——图元走本模块，贴图走 `draw_surface` 系。
```

## <span class="c-func">push 统一口</span>

手写顶点数据的统一出口——与图元函数族（SDL_gfx 风格）互补。数据
形态为**位置流**（每点 x,y,z 三个分量），颜色、uv、变换全部在入批
时按当前状态烘焙——数据全程 Rust 侧流转，Python 侧无逐顶点 tuple
摆渡。

```python
with screen.render() as t:
    tri = pygame.gfxdraw.gen_circle(60)          # 生成器：Rust 生成位置流
    pygame.gfxdraw.set_topology(t, "triangles")  # sticky——跨 push 持续
    pygame.gfxdraw.set_color(t, (230, 90, 60))
    pygame.gfxdraw.set_transform(t, pygame.math.mat4_translation(-200, 0, 0))
    pygame.gfxdraw.push(t, tri)                  # 生成器句柄
    pygame.gfxdraw.push(t, [80, 0, -80, 0, 0, 100])  # 扁平 list 同色同拓扑

    pygame.gfxdraw.set_topology(t, "lines")      # 切拓扑
    pygame.gfxdraw.push(t, seg_bytes)            # bytes 直拷通道
```

语义与图元函数族的分界：**push 流是 sticky**——`set_color` /
`set_topology` / `set_transform` 设置后跨 push 持续，直到再次设置；
图元函数族保持 one-shot（draw 后重置，pygame 兼容语义）不变。

拓扑语义：顶点按**点**分组——`triangles` = 3 点一三角、`lines` =
2 点一线段；每点恒为 x,y,z 三分量（2D 场景 z 传 0）。索引越界自动
跳过对应组（不 panic——push 收用户任意数据）。

#### <span class="c-func">VertexBuf()</span>

顶点位置流容器。`add_point(x, y)`（z=0）/ `add_point3(x, y, z)` 单点
追加；`add_points(flat)` 扁平一维数组批量追加（一次调用，Rust 内
解包）；`extend(data)` bytes 直拷（f32 LE 内存块，长度须为 12 的
倍数——`struct.pack` / `array` / numpy 产出零解包）；`set_indices(
indices)` 设置索引；`clear()` 清空复用容器。

```python
buf = pygame.gfxdraw.VertexBuf()
buf.add_points([x1, y1, 0, x2, y2, 0, x3, y3, 0])   # 3 个点
buf.set_indices([0, 1, 2])
pygame.gfxdraw.push(t, buf)
```

#### <span class="c-func">push(f, data, indices=None)</span>

顶点位置流入批。`data` 三通道：`VertexBuf`（含自带索引）/ bytes
（f32 LE）/ list（扁平 `[x,y,z,...]` 数字流或 `[(x,y),...]` 点列表）。
`indices` 参数可覆盖 VertexBuf 自带索引；不设索引时按拓扑逐组解释。

#### <span class="c-func">set_topology(f, mode)</span>

push 流拓扑状态（sticky）。`mode` 取 `"triangles"`（默认）、`"lines"`
或 `"default"`（回默认）。

#### 生成器族（gen_*）

Rust 内生成位置流，返回 `VertexBuf`——性能与生成全程不落地 Python
一致。2D 形状产出为**原点中心局部坐标**（`gen_line`/`gen_polygon`
系例外——输入点即局部坐标），位置由 `set_transform` 赋予：

| 生成器 | 产出 |
|---|---|
| `gen_circle(radius, segments=48)` | 填充圆（扇形三角化） |
| `gen_circle_outline(radius, segments=64)` | 圆周线（1px） |
| `gen_rect(w, h)` / `gen_rect_outline(w, h)` | 矩形 / 周界线 |
| `gen_ellipse(rx, ry, segments=48)` | 填充椭圆 |
| `gen_arc(radius, start_deg, stop_deg, segments=48)` | 圆弧线（度数制） |
| `gen_line(x1, y1, x2, y2)` | 线段 |
| `gen_polygon(points)` / `gen_polygon_outline(points)` | 多边形填充 / 周界线 |
| `gen_cube(size)` / `gen_box(min, max)` | 立方体 / 轴对齐盒（3D） |
| `gen_sphere(radius, su=16, sv=12)` | 球体（3D） |
| `gen_plane(center, right, up, size)` | 任意朝向面片（3D） |
| `gen_cylinder(radius, height, segments=24)` / `gen_cone(radius, height, segments=24)` | 圆柱 / 圆锥（3D，中心 = 柱高中点） |
| `gen_polygon3d(points)` | 3D 多边形板（Newell 稳健三角化） |

## <span class="c-func">状态机</span>

#### <span class="c-func">set_color(f, color)</span>

**绘制色状态**——后续 push / 图元绘制用此色。push 流 = **sticky**
（跨 push 持续直到再次设置）；不设 = **默认紫色**（调试可辨——忘了
设色一眼看出）。

```python
pygame.gfxdraw.set_color(f, (255, 0, 0))
pygame.gfxdraw.set_transform(f, pygame.math.mat4_translation(0, 0, 0))
pygame.gfxdraw.push(f, pygame.gfxdraw.gen_circle(50))      # → 红色
pygame.gfxdraw.set_transform(f, pygame.math.mat4_translation(100, 0, 0))
pygame.gfxdraw.push(f, pygame.gfxdraw.gen_circle(50))      # → 仍红色（sticky）
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
- :param camera: 相机（`pygame.render.game_camera_ortho` / `game_camera_perspective` 产物）
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

## <span class="c-func">动画与外接管线插座</span>

`mesh(f, surface, positions, uvs, indices, color)`：带贴图三角网格
通用口（外部数据流消费）。`draw_spine` / `draw_ozz_quads` /
`draw_ozz_bones`：动画运行时插座（spine / ozz 适配库自持管线）。

### 相机状态（clear_camera）

#### <span class="c-func">clear_camera(f)</span>

清除会话相机覆盖（回按目标尺寸正交）——2026-11-12 由 `reset_camera`
改名：隐式兜底已退役，"clear" 才是实义。段级 `set_camera` 的配对清除口。

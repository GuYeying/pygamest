# pygame.gfxdraw

SDL_gfx 风格图元 + **渲染统一 draw 面**——`gen_*` 生成器产出顶点数据，
`draw_batch` 一口入批：**参数自包含**（transform/color/wireframe kwargs，
`None` = 沿用会话当前值；one-shot 语义统一——本次是本次、下次是下次）。

```{note}
首参 `f` 是**渲染会话**（`with screen.render() as f` 的 `f`，
或 Surface 会话）。

[坐标系] **中心原点、y 向上**——`(0, 0)` = 屏幕/画布中心，`+y` 向上
（与 3D 透视 / 物理米制系三方同构）。

[拓扑随数据] 顶点流的图元解释**随数据走**：填充族生成器烤入
`triangles`、线族烤入 `lines`——无需手工声明；自定义流经
`buf.topology` 属性覆写（"triangles"/"lines"）。

[图元函数族退役] SDL_gfx 风格图元函数（`filled_circle` / `box` /
`line` / `rectangle` 等直接绘制口）已全部剔除——形状统一走
`gen_*` 生成器 + `draw_batch` 统一口（数据面收敛，见下节）。

[旧口退役] 会话对象上的 `r.rect` / `r.circle` 等绘制便捷方法与 `blit`
家族已全部剔除——图元走本模块，贴图走 `draw_surface` 系。

[混合/剔除] 图元内容层**全局口**：`pygame.render.set_blend_mode("alpha"
/ "opaque" / "premultiplied")` + `set_cull_mode("none" / "back" /
"front")`——gfxdraw 几何与 draw_surface 系全部消费该状态（详见
[pygame.render](render.md#set-blend-mode-mode)）。per-call 粒度未做
（batch 分仓无 blend 维度）。
```

## <span class="c-func">draw_batch 统一口</span>

顶点批绘制统一口——**参数自包含**（2026-11-25 接口破坏定约：替代
sticky 三口 + push 组合，FFI N+1→1；2026-11-27 one-shot 统一——
transform/color/wireframe 本次是本次、下次是下次，各自独立）。

```python
with screen.render() as t:
    # 拓扑随数据：outline 生成器烤入 Lines、填充生成器烤入 Triangles
    pygame.gfxdraw.draw_batch(
        t, pygame.gfxdraw.gen_circle_outline(60),
        transform=pygame.math.mat4_translation(-200, 0, 0),
        color=(230, 90, 60, 255),
    )
    pygame.gfxdraw.draw_batch(
        t, pygame.gfxdraw.gen_circle(60),
        transform=pygame.math.mat4_translation(200, 0, 0),
        color=(90, 160, 255, 255),
        wireframe=True,                      # one-shot 仅本批
    )
    pygame.gfxdraw.draw_batch(t, seg_bytes)  # bytes 直拷通道（默认色/变换）
```

- `transform=None`：沿用会话当前变换（`set_transform` 设置的持久值）
- `color=None`：沿用会话当前绘制色
- `wireframe=None`：沿用会话当前线框开关

数据形态为**位置流**（每点 x,y,z 三分量），颜色、uv、索引全部在入批
时烘焙——数据全程 Rust 侧流转，Python 侧无逐顶点 tuple 摆渡。

#### <span class="c-func">VertexBuf()</span>

顶点位置流容器。`add_point(x, y)`（z=0）/ `add_point3(x, y, z)` 单点
追加；`add_points(flat)` 扁平一维数组批量追加（一次调用，Rust 内
解包）；`extend(data)` bytes 直拷（f32 LE 内存块，长度须为 12 的
倍数——`struct.pack` / `array` / numpy 产出零解包）；`set_indices(
indices)` 设置索引；`clear()` 清空复用容器。

**`topology` 属性**（可读写）：`"triangles"`（默认）/ `"lines"`——
自定义流的图元解释覆写口。

```python
buf = pygame.gfxdraw.VertexBuf()
buf.add_points([x1, y1, 0, x2, y2, 0, x3, y3, 0])   # 3 个点
buf.set_indices([0, 1, 2])
buf.topology = "lines"                               # 覆写解释（可选）
pygame.gfxdraw.draw_batch(t, buf)
```

#### <span class="c-func">draw_batch(f, data, transform=None, color=None, wireframe=None)</span>

顶点批绘制。`data` 三通道：`VertexBuf`（自带拓扑与索引）/ bytes
（f32 LE）/ list（扁平 `[x,y,z,...]` 数字流或 `[(x,y),...]` 点列表，
按 Triangles 解释）。`indices` 语义：VertexBuf 自带索引优先。

#### 生成器族（gen_*）

Rust 内生成位置流，返回 `VertexBuf`——性能与生成全程不落地 Python
一致。2D 形状产出为**原点中心局部坐标**（`gen_line`/`gen_polygon`
系例外——输入点即局部坐标）。**拓扑烤入**：填充族 → `triangles`、
`*_outline`/`line`/`arc` → `lines`：

| 生成器 | 产出 | 烤入拓扑 |
|---|---|---|
| `gen_circle(radius, segments=48)` | 填充圆（扇形三角化） | triangles |
| `gen_circle_outline(radius, segments=64)` | 圆周线（1px） | lines |
| `gen_rect(w, h)` / `gen_rect_outline(w, h)` | 矩形 / 周界线 | triangles / lines |
| `gen_ellipse(rx, ry, segments=48)` | 填充椭圆 | triangles |
| `gen_arc(radius, start_deg, stop_deg, segments=48)` | 圆弧线（度数制） | lines |
| `gen_line(x1, y1, x2, y2)` | 线段 | lines |
| `gen_polygon(points)` / `gen_polygon_outline(points)` | 多边形填充 / 周界线 | triangles / lines |
| `gen_cube(size)` / `gen_box(min, max)` | 立方体 / 轴对齐盒（3D） | triangles |
| `gen_sphere(radius, su=16, sv=12)` | 球体（3D） | triangles |
| `gen_plane(center, right, up, size)` | 任意朝向面片（3D） | triangles |
| `gen_cylinder(radius, height, segments=24)` / `gen_cone(radius, height, segments=24)` | 圆柱 / 圆锥（3D，中心 = 柱高中点） | triangles |
| `gen_polygon3d(points)` | 3D 多边形板（Newell 稳健三角化） | triangles |

## <span class="c-func">会话变换（blit / spine / ozz 桥的持久变换）</span>

#### <span class="c-func">set_transform(f, model)</span>

会话持久变换——`draw_surface` 系 / `draw_spine` / `draw_ozz_*` 桥的
顶点按此矩阵 CPU 烘焙（**这些桥不收 transform 参数，走持久状态**；
`draw_batch` 不读此状态——它的 transform 走 kwargs）。

- :param model: 模型矩阵（Mat4）
- :type model: Mat4

#### <span class="c-func">reset_transform(f)</span>

回恒等变换。

## <span class="c-func">贴图通道（draw_surface 系）</span>

#### <span class="c-func">draw_surface(f, src, transform=None)</span>

画一张离屏面——**中心锚**（局部原点 = 物体中心）。位置/旋转/缩放/
镜像经 `transform=` kwarg 自带（Mat4——None = 无变换直绘；内部走
tf 通道）。**会话持久变换（set_transform）已不作用于本口**。

#### <span class="c-func">draw_surface_uv(f, src, uv, tint=None)</span>

子区域画面。

#### <span class="c-func">draw_surface_tf(f, src, mat, tint=None)</span>

显式矩阵画面（mat 全权，不复合会话 transform）。

## <span class="c-func">动画与外接管线插座</span>

`mesh(f, surface, positions, uvs, indices, color)`：带贴图三角网格
通用口（外部数据流消费）。`draw_spine` / `draw_ozz_quads` /
`draw_ozz_bones`：动画运行时插座（spine / ozz 适配库自持管线）。

### 相机状态（clear_camera）

#### <span class="c-func">set_camera(f, camera)</span>

会话相机切换（后续 gfx 绘制进新段；仅本会话生效）。

- :param camera: 相机（`pygame.render.game_camera_ortho` / `game_camera_perspective` 产物）
- :type camera: Camera

#### <span class="c-func">reset_camera(f)</span>

回 Auto 兜底相机。

#### <span class="c-func">clear_camera(f)</span>

清除会话相机覆盖（回按目标尺寸正交）——2026-11-12 由 `reset_camera`
改名：隐式兜底已退役，"clear" 才是实义。段级 `set_camera` 的配对清除口。

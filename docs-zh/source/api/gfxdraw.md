# pygame.gfxdraw

形状生成器（`gen_*`）产出顶点数据 → `draw_batch` 一口入批——2D/3D
几何共用这一条通道：**参数自包含**（`transform` / `color` / `wireframe` /
`blend` kwargs，one-shot 语义统一——本次是本次、下次是下次）。

```{note}
首参 `f` 是**渲染会话**（`with screen.render() as f` 的 `f`，
或 Surface 会话）。

[坐标域] **像素坐标要先声明相机**：`screen.set_camera(
pygame.render.game_camera_ortho(w, h, w, h))`，或会话段级
`pygame.gfxdraw.set_camera(f, cam)` / `clear_camera(f)`（回落像素正交）。
未声明相机的 **Screen 会话 = NDC 直通**（裸管线域，`[-1, 1]` 单位方框
——直接喂像素坐标会整屏铺满或飞出画面）；**Surface 会话恒像素正交**
（逻辑 == 物理，无需相机）。

[坐标系] 声明正交相机后：**中心原点、y 向上、+x 向右**（`translate(0,
+100)` 落在画面**上**半、`-100` 落下半——2026-11-27 帧缓冲目视实测）。

[拓扑随数据] 顶点流的图元解释**随数据走**：填充族生成器烤入
`triangles`、线族烤入 `lines`——无需手工声明；自定义流经
`buf.topology` 属性覆写（"triangles"/"lines"）。

[剔除随数据] **内外约定与 VertexBuf 强绑定**（2026-11-28 定约）：
闭实体生成器（`gen_cube` / `gen_box` / `gen_sphere` / `gen_cylinder` /
`gen_cone`）烤入 `buf.cull = "back"`（绕序一致外向，内视全剔实证）；
开薄面 / 2D 族恒 `"none"`（单面板天然双面，像素域剔除无意义）；
线拓扑无面忽略剔除。`buf.cull` 属性可读写（"none"/"back"）覆写约定；
`draw_batch` 的 `cull` kwarg **已退役**——传了即 `TypeError`。

[图元函数族退役] SDL_gfx 风格图元函数（`filled_circle` / `box` /
`line` / `rectangle` / `set_color` 等直接绘制口）已全部剔除——形状
统一走 `gen_*` 生成器 + `draw_batch` 统一口（数据面收敛，见下节）。

[旧口退役] 会话上的**图元便捷口**（`r.rect` / `r.circle` / `r.line`…）
与整张贴图 `blit` 已剔除——图元走本模块，贴图走 `draw_surface`。会话
仍留低层逃逸舱 `fill` / `quad` / `vertices` / `blit_area`。

[混合/剔除] `blend` **逐次参数**（2026-11-27 per-call 参数化）：
`draw_batch(..., blend=...)` / `draw_surface(..., blend=...)`。取值
`alpha`（默认）/ `opaque`（别名 `none` / `noblend`）/ `premultiplied` /
`additive` / `multiply`。`cull` **随数据**（2026-11-28——`draw_batch`
无此 kwarg，经 `buf.cull`；`draw_surface` 仍逐次收 `cull=`，取值
`none`（默认）/ `back`，`front` 仅供自建管线）。模块级
`pygame.render.set_blend_mode` / `set_cull_mode` **已移除**——调用即
`RuntimeError`。

[3D 单一通道] 3D 直调口（`cube` / `sphere` / `plane` / `cylinder` /
`cone` / `polygon3d`）**已全部删除**——3D 一律 `gen_*` → `draw_batch`
（闭实体自动烤入 `buf.cull="back"`），配合 `screen.render_depth()`
深度会话 + 透视相机。
```

## <span class="c-func">draw_batch 统一口</span>

顶点批绘制统一口——**参数自包含**（2026-11-25 接口破坏定约：替代
sticky 三口 + push 组合，FFI N+1→1；2026-11-27 one-shot 统一——
transform/color/wireframe 本次是本次、下次是下次，各自独立）。

```python
screen = pygame.display.set_mode((800, 600))
# 像素域要显式相机（Screen 会话无相机 = NDC 直通）
screen.set_camera(pygame.render.game_camera_ortho(800, 600, 800, 600))

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
        blend="additive",                    # 混合逐次给
    )
    seg = pygame.gfxdraw.VertexBuf()
    seg.extend(seg_bytes)                # f32 LE 直拷（每点 12 字节）
    seg.topology = "lines"
    pygame.gfxdraw.draw_batch(t, seg)     # 默认色/无变换
```

- `transform=None`：不设模型矩阵（走会话当前值，通常恒等——旧代码靠
  `set_transform` 的写法已无现役消费方，3D 一律显式传矩阵）
- `color=None`：会话当前色，**默认引擎紫 `(128, 0, 128)`**——彩色场景
  务必逐个给色（否则一屏同色分不清结构）
- `wireframe=None`：沿用会话当前线框开关
- `blend=None`：`alpha`（取值见页首 note）
- 剔除**不收 kwarg**——随 `buf.cull`（见页首[剔除随数据]）

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

**`cull` 属性**（可读写）：`"none"`（默认）/ `"back"`——内外约定
覆写口。闭实体 `gen_*` 烤入 `"back"`，自定义 3D 外向绕序流想启用
背面剔除就设它；`"front"` 与其他值拒绝（`ValueError`）。线拓扑忽略
剔除（无面可剔）。

```python
buf = pygame.gfxdraw.VertexBuf()
buf.add_points([x1, y1, 0, x2, y2, 0, x3, y3, 0])   # 3 个点
buf.set_indices([0, 1, 2])
buf.topology = "lines"                               # 覆写解释（可选）
buf.cull = "back"                                    # 覆写内外约定（可选）
pygame.gfxdraw.draw_batch(t, buf)
```

#### <span class="c-func">draw_batch(f, data, transform=None, color=None, wireframe=None, blend=None)</span>

顶点批绘制。`data` 只收 `VertexBuf`（拓扑/索引/剔除约定随数据）——
外部数据先 `VertexBuf().extend(bytes)` / `add_points(flat)` 灌进去再
提交。`blend` 是**本次绘制的管线状态**（不落回会话，见页首 note）；
剔除走 `data.cull`（无 kwarg——传了即 `TypeError`）。

#### 生成器族（gen_*）

Rust 内生成位置流，返回 `VertexBuf`——性能与生成全程不落地 Python
一致。2D 形状产出为**原点中心局部坐标**（`gen_line`/`gen_polygon`
系例外——输入点即局部坐标）。**拓扑烤入**：填充族 → `triangles`、
`*_outline`/`line`/`arc` → `lines`。**剔除烤入**：闭实体 →
`back`、开薄面 / 2D 族 → `none`：

| 生成器 | 产出 | 烤入拓扑 | 烤入剔除 |
|---|---|---|---|
| `gen_circle(radius, segments=48)` | 填充圆（扇形三角化） | triangles | none |
| `gen_circle_outline(radius, segments=64)` | 圆周线（1px） | lines | —（无面） |
| `gen_rect(w, h)` / `gen_rect_outline(w, h)` | 矩形 / 周界线 | triangles / lines | none / — |
| `gen_ellipse(rx, ry, segments=48)` | 填充椭圆 | triangles | none |
| `gen_arc(radius, start_deg, stop_deg, segments=48)` | 圆弧线（度数制） | lines | —（无面） |
| `gen_line(x1, y1, x2, y2)` | 线段 | lines | —（无面） |
| `gen_polygon(points)` / `gen_polygon_outline(points)` | 多边形填充 / 周界线 | triangles / lines | none / — |
| `gen_cube(size)` / `gen_box(min, max)` | 立方体 / 轴对齐盒（3D） | triangles | **back** |
| `gen_sphere(radius, su=16, sv=12)` | 球体（3D，**中心原点**） | triangles | **back** |
| `gen_plane(center, right, up, size)` | 任意朝向面片（3D） | triangles | none（单面板双面） |
| `gen_cylinder(radius, height, segments=24)` / `gen_cone(radius, height, segments=24)` | 圆柱 / 圆锥（3D，**原点 = 柱高中点**，沿 +y 挤出） | triangles | **back** |
| `gen_polygon3d(points)` | 3D 多边形板（Newell 稳健三角化） | triangles | none（单面板双面） |

**3D 族约定**（配合 `screen.render_depth()` + 透视相机，一律显式
`transform=` 摆位）：

- `gen_sphere` / `gen_cylinder` / `gen_cone` 以**自身原点为心**（柱/锥
  = 柱高中点），没有 `center` 参数——摆位靠 `transform=mat4_translation(...)`；
- 闭合体统一 **CCW 外向绕序**，烤入的 `back` 剔的是背向面（内容层
  按 Ccw = 正面裁决；内视全剔已探针实证）。单面板（`gen_plane` /
  `gen_polygon3d`）恒 `none` 双面可见——需要单面就 `buf.cull = "back"`；
- **已知边界**：负行列数变换（镜像 `mat4_scale(-1,1,1)` 等）翻转绕序，
  烤入的 back 会剔掉本应可见的面（静默隐形）——det<0 自动翻转剔除
  属后续批次，现版镜像场景请 `buf.cull = "none"` 或复制缓冲覆写；
- 旧 3D 直调口（`cube(f, min, max)` / `sphere(f, center, r)` /
  `cylinder(f, base_center, r, h)` 等带绝对坐标的便捷口）**已删除**，
  文档/案例里再出现即过期写法。

## <span class="c-func">会话变换（仅 mesh 口残留）</span>

#### <span class="c-func">set_transform(f, model)</span>

会话持久变换。**现役只剩一个消费方**：`mesh` 通用网格口（不收
`transform=`，顶点按此矩阵烘）。`draw_batch` / `draw_surface` /
`draw_spine` / `draw_ozz_quads` / `draw_ozz_bones` 全部改收 per-call
`transform=`，不读此状态——新代码不要再依赖它（候补退役）。

- :param model: 模型矩阵（Mat4）
- :type model: Mat4

#### <span class="c-func">reset_transform(f)</span>

回恒等变换。

## <span class="c-func">贴图通道（draw_surface）</span>

#### <span class="c-func">draw_surface(f, src, transform=None, src_rect=None, dst_rect=None, tint=None, blend=None, cull=None)</span>

画一张离屏面（单口——原 `draw_surface` / `draw_surface_uv` /
`draw_surface_tf` 三口已合并，后两个名字**不存在**）。

- **位置两选一**：`transform`（Mat4，**中心锚**——局部原点 = 面中心，
  可旋转/缩放/镜像）**或** `dst_rect=[x, y, w, h]`（轴对齐拉伸到该矩形；
  y 向上，`[x, y]` 是矩形左下角）。两者同给抛 `ValueError`；全不给 = 中心
  锚原尺寸直绘。
- `src_rect=[u0, v0, u1, v1]`：源面**归一化 UV** 子区域（不是像素）——
  精灵表/九宫格切片用它。
- `tint=(r, g, b, a)`：整体染色（乘算）；`None` = 原色直出。
- `blend`：同 `draw_batch`（逐次管线状态）。`cull`：同样逐次收参
  （`none`/`back`——引擎生成的 quad 不经 VertexBuf，无数据可烤约定），
  默认 `none`。

```python
pygame.gfxdraw.draw_surface(t, sprites, src_rect=(0.5, 0.0, 1.0, 0.5),
                            dst_rect=(100, 80, 64, 64))
pygame.gfxdraw.draw_surface(t, sheet,
                            transform=pygame.math.mat4_translation(0, 0, 0)
                            @ rot, tint=(255, 120, 120, 255))
```

## <span class="c-func">动画与外接管线插座</span>

| 口 | 签名要点 |
|---|---|
| `mesh` | `mesh(f, surface, positions, uvs, indices, color=None)`——带贴图三角网格通用口（外部数据流消费；**唯一仍读 `set_transform` 的桥**） |
| `draw_spine` | `draw_spine(f, skeleton, pages, transform=None)`——spine 命令流 → 网格上屏 |
| `draw_ozz_quads` | `draw_ozz_quads(f, player, scale=1.0, cx=0.0, cy=0.0, size=14.0, transform=None)`——ozz 关节盒体 |
| `draw_ozz_bones` | `draw_ozz_bones(f, player, scale=1.0, cx=0.0, cy=0.0, transform=None)`——骨架线（调试形态） |

### 相机状态（clear_camera）

#### <span class="c-func">set_camera(f, camera)</span>

会话相机切换（后续 gfx 绘制进新段；仅本会话生效）。不调 = 目标默认域
（Screen 会话 = NDC 直通，Surface = 像素正交）。

- :param camera: 相机（`pygame.render.game_camera_ortho` / `game_camera_perspective` 产物）
- :type camera: GameCamera

#### <span class="c-func">clear_camera(f)</span>

清除会话相机覆盖（回按目标尺寸正交）——2026-11-12 由 `reset_camera`
改名（旧名已不存在）：隐式兜底已退役，"clear" 才是实义。段级
`set_camera` 的配对清除口。

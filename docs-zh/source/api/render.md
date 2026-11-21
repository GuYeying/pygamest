# pygame.render

底层渲染模块——用 WGSL 编写自定义渲染管线，配合 `raw_render()` 直绘
会话与计算会话使用。

渲染会话（`with screen.render() as f:` 的会话对象）是**纯通道**——
本身不承载绘制方法。绘制接口按内容分模块：图形图元与贴图用
[pygame.gfxdraw](gfxdraw.md)，文字用 [pygame.font](font.md)，自定义
管线命令用本模块的直绘会话。

## 坐标系与相机

- **中心原点、y 轴向上**——正交相机的视野为
  `(-w/2, -h/2) .. (w/2, h/2)`，与 3D 透视相机、物理米制系三方同构；
  鼠标 / 触摸 / 事件坐标由引擎统一换算到中心系
- 一切图元的位置 = `present_fit × 相机.mvp()`——相机构造用
  [game_camera_ortho / game_camera_perspective](#gamcamera-正交与透视相机)
- 未显式 `screen.set_camera()` 时走 NDC 恒等域——自定义管线顶点直接
  按 NDC 坐标写（错位显眼，便于诊断）
- pygame 图元 API（[pygame.gfxdraw](gfxdraw.md)）恒像素域，与相机
  解耦——HUD / UI 不随游戏相机漂移
- 自建管线（`raw_render`）**引擎不做相机兜底**——相机 buffer 由脚本
  全权管理（同帧多个 raw 会话须各持独立 buffer）

## 示例

```python
import pygame

ac = pygame.render.access()
cam_buf = ac.buffer(64)
vert_shader = ac.shader(WGSL_VERT)
mesh = ac.mesh(["float32x3"], tri_verts)

with screen.raw_render(depth=True) as p:
    p.set_pipeline(tri_pipe)
    p.set_bind_group(0, tri_bind)
    p.draw_mesh(tri_mesh)
```

wgpu 裸对象（Device / Queue / Buffer / TextureView）不出 Python 界——
一切经本模块句柄类安全收参。

## <span class="c-func">函数</span>

#### <span class="c-func">game_camera_ortho(lw, lh, pw, ph) -> GameCamera</span>

2D 正交 GameCamera（**letterbox 内聚**——逻辑画布 → 物理视口的等比
适配在相机内部结算，MVP/坐标映射/裁剪同一事实源）。

- :param lw: 逻辑画布宽（set_mode 语义尺寸）
- :param lh: 逻辑画布高
- :param pw: 物理视口宽（窗口实际像素）
- :param ph: 物理视口高
- :rtype: GameCamera

#### <span class="c-func">game_camera_perspective(fov_deg, lw, lh, pw, ph, near, far) -> GameCamera</span>

3D 透视 GameCamera（fov 度数制；纵横比按**逻辑画布**；letterbox 内聚
同上）。

- :rtype: GameCamera

#### <span class="c-func">present_fit() -> Mat4</span>

返回**画布 → 物理视口**的 letterbox 拟合矩阵（NDC 域）。桌面与全屏
下为恒等矩阵；Android 等窗口化平台上，会把自绘内容收进居中画布。

与相机矩阵的组合公式：`present_fit().mul_mat4(cam.mvp())`——相机
负责世界 → 画布，本矩阵负责画布 → 物理视口。

- :rtype: Mat4

#### <span class="c-func">init(dim=None, fov=None, near=None, far=None, cull=None, blend=None)</span>

渲染管线愿望（`set_mode` **前**设置；部分覆盖——未传保留现值，后设
优先）。`dim`: 2 / 3（Gpu 装配消费——**不再喂隐式相机**，域分层 v3
后 3D 须显式 `screen.set_camera(game_camera_perspective(...))`）；
`cull`: `"none"`（默认）/ `"back"` / `"front"`（管线编译期定型，
下次 set_mode 生效）；`blend`: `"alpha"`（默认）/ `"opaque"` /
`"premultiplied"`。

#### <span class="c-func">access(offscreen=False) -> ResourceAccess</span>

底层资源入口（自建管线建造工厂；须 `set_mode` 后，否则 RuntimeError）。

主屏 access 的默认颜色格式 = 交换链格式（平台相关：Web / macOS
Bgra8，Windows Rgba8）；`offscreen=True` 离屏版恒 Rgba8。

- :rtype: ResourceAccess

#### <span class="c-func">screen_size() -> tuple[int, int]</span>

主屏物理像素。

- :rtype: tuple[int, int]

#### <span class="c-func">screen_format() -> str</span>

主屏颜色格式名（wgpu 枚举字符串形态）。

- :rtype: str

#### <span class="c-func">screen_samples() -> int</span>

MSAA 样本数。

- :rtype: int

#### <span class="c-func">screen_attachment() -> ColorAttachment | None</span>

当前帧颜色附件（快照句柄；**须在帧内调用——present 后失效**）。

- :rtype: ColorAttachment | None

#### <span class="c-func">write_camera_mvp(buf, cam_mvp=None)</span>

把相机矩阵（列主 16 float）写入 Buffer——自建管线的相机传递口
（`group0/b0` 64B mat4 布局）。两种形态：

- **双参** `write_camera_mvp(buf, cam.mvp())` —— 传入自管相机矩阵，
  引擎自动左乘呈现拟合（letterbox）收进画布，脚本无需关心窗口缩放
- **单参** `write_camera_mvp(buf)` —— 写入全局相机 MVP
  （`present_fit × 屏上相机`），与段级绘制 / debug / egui 消费同一
  组合

#### <span class="c-func">screen_frame_state() -> str</span>

帧生命周期状态（诊断——定位探针时序与跳帧）。

- :rtype: str

#### <span class="c-func">set_fb_dump(on)</span>

帧缓冲取证 dump 开关（诊断；Android 落 dev_log）。

#### <span class="c-func">screen_probe() -> list</span>

帧缓冲网格回读（**诊断专用**；flip 前调用有效）——
`[(x, y, r, g, b, a), ...]`。

- :rtype: list

## <span class="c-class">类</span>

### <span class="c-class">Camera —— 相机</span>

直呼不可实例化——经 `camera_ortho` / `camera_perspective` 构造。

##### <span class="c-func">with_view(view) -> Camera</span>

替换视图矩阵（3D 位姿：`Mat4::look_at_rh` 等；2D 恒 Identity）。返回新
Camera。

##### <span class="c-func">mvp() -> Mat4</span>

MVP（projection × view——唯一上传量；调试 / 教学口）。

### <span class="c-class">GameCamera —— letterbox 内聚相机</span>

**显式传入，杜绝兜底**（2026-11 定约）——`screen.set_camera(cam)` 的
实参类型；直呼不可实例化，经 `game_camera_ortho` /
`game_camera_perspective` 构造。内部三合一：投影（ortho / perspective）
× 视图 × letterbox 适配，`mvp()` 为唯一上传量（NDC 域拟合 × projection
× view），同时承载逻辑 ↔ 物理坐标映射与裁剪区换算。设置后
`refresh_camera`（set_mode / resize 自动重建）不再覆盖——显式优先于
自动；物理视口变化走 `.resized(pw, ph)` 重算 letterbox。

**[物理面对账]**（2026-11-11 手机实证定约）构造入参的 pw/ph 只是
设计意图（桌面恰好等于窗口尺寸）——`screen.set_camera` 时引擎以
**当前物理表面**重算 letterbox（投影/视图/逻辑面不动）。跨设备
（手机物理面 ≠ 桌面 800×600）不重对账 = fit_ndc 恒等映射 = 内容拉伸
满屏、letterbox 失效；对账后脚本零设备知识即可移植。

```python
cam = pygame.render.game_camera_perspective(58.0, 800, 600, 800, 600, 0.1, 300.0)
screen.set_camera(cam)
```

##### <span class="c-func">with_view(view) -> GameCamera</span>

替换视图矩阵（3D 位姿 `Mat4`；2D 恒 Identity）。返回新 GameCamera。

##### <span class="c-func">mvp() -> Mat4</span>

letterbox 内聚 MVP（唯一上传量）。

##### <span class="c-func">physical_to_logical(x, y) -> tuple[float, float]</span>

物理点 → 逻辑点（事件坐标进脚本坐标系）。

##### <span class="c-func">resized(pw, ph) -> GameCamera</span>

物理视口尺寸变化（重算 letterbox；Camera 是 Copy——返回新对象）。

### <span class="c-class">ResourceAccess —— 资源工厂</span>

`access()` 产物。

##### <span class="c-func">shader(wgsl, label=None) -> Shader</span>

WGSL 源码编译。

##### <span class="c-func">buffer(size, label=None, usage="uniform") -> Buffer</span>

raw 缓冲（字节自管——相机定约 64B）。`usage`:
`"uniform"`（默认）/ `"indirect"` / `"storage"`。

##### <span class="c-func">mesh(layout, vertices, indices=None, index_bytes=None, index_format=None, floats=None, vertices_bytes=None, label=None) -> Mesh</span>

顶点网格。`layout` 布局预设：`pos3`(3) / `pos3uv2`(5) / `pos3color4`(7)
/ `pos3uv2color4`(9) / `corner2`(2) / `inst_pos2`(2)（GPU 粒子
instanced 通路）；`vertices` 扁平数值列表（按布局 stride 校验）；
`vertices_bytes` 原始字节（任意格式混用——struct.pack 形态）；
`vertices=None` 须同时给 `floats=N`（零填充容量预留）；`indices`
u16 列表（0..=65535）；**`index_bytes` 原始字节索引 + `index_format`
"u16"/"u32"**（高级口——大 mesh >65535 顶点 / LE 字节直喂，越界校
验调用方自担；两参数互斥——indices 列表口恒 u16）。

##### <span class="c-func">update_mesh(mesh, vertices, indices=None)</span>

动态几何——顶点（/索引）就地更新，免每帧重建。**容量以建时为准**
（超出 = wgpu 越界校验错；更大容量请重建）。indices 列表口恒 u16。

##### <span class="c-func">update_mesh_index_bytes(mesh, data, format="u16")</span>

**原始字节索引更新**（LE 直喂——u32 大 mesh 通路）。格式须与 mesh
建时 index_format 一致（不匹配抛 ValueError）；无索引 mesh 抛
ValueError（index 建立须走 mesh() 的 indices/index_bytes 口）；超
容量截断告警。

##### <span class="c-func">mesh.index_format() -> str | None</span>

`"u16"` / `"u32"`；无索引 = None。配套查询：`index_count()` /
`has_index_buffer()` / `index_buffer_size()`（字节容量）/ 
`vertex_buffer_size()`（copy/update 守卫基准）。

##### <span class="c-func">bind_group(bindings, label=None) -> BindGroup</span>

绑定组——`bindings = [(slot, kind, obj), ...]`；`kind`:
`"uniform"`（Buffer）/ `"texture"`（Surface 或 Texture）/
`"texture_data"`（`rgba32f` Texture——VS texelFetch 通路）/
`"sampler"`（Sampler 或 None = 默认线性 clamp）/ `"storage"` /
`"storage_rw"`（storage Buffer）。

##### <span class="c-func">pipeline(shader, mesh, bind_groups, topo="triangles", cull="none", depth_mode=None, depth_write=None, blend=None, wireframe=False, alpha_to_coverage=False, vertex_entry=None, fragment_entry=None, label=None) -> Pipeline</span>

渲染管线（按本 access 默认颜色格式建——不传可选参数 = 中性基座：
无深度 / 无剔除 / alpha 混合 / vs_main + fs_main）。

| 参数 | 可选值 | 语义 |
|---|---|---|
| topo | triangles / lines | 三角形 / 线段拓扑 |
| cull | none / back / front | 面剔除 |
| depth_mode | disabled / standard / reverse | 深度模式（standard = 近0远1；reverse = 近1远0——无限远投影/z-fighting 减少）|
| depth_write | bool | 深度写入开关（透明物体 / 天空盒：读深度但不写入）|
| blend | alpha / opaque / additive / multiply / none | 混合模式（additive = 发光/粒子；multiply = 阴影叠加）|
| wireframe | bool | GPU 线框（需 `Features::POLYGON_MODE_LINE`）|
| alpha_to_coverage | bool | MSAA A2C（植被 / alpha 裁切）|
| vertex_entry | str | 顶点着色器入口名（默认 vs_main——同 shader 多入口复用）|
| fragment_entry | str | 片元着色器入口名（默认 fs_main）|

**`depth_mode` 须配 `raw_render(depth=True)`**（深度目标严格配对）。

### 透明对象处理（四个可控层级）

| 层级 | 口 | 语义 |
|---|---|---|
| 管线级 | `pipeline(blend="alpha")` | 片元 alpha（WGSL 返回 `vec4(r,g,b,a)` 第四分量）参与混合；`"additive"` = 发光/粒子叠加；`"multiply"` = 阴影 |
| Surface 级 | `Surface.set_alpha(0..255)` | 面级整体透明度（`draw_surface` 时生效）；图片 alpha 通道（PNG 带透明内容 blit 自然透） |
| 会话序 | **画家序** | alpha 混合无深度排序——会话内绘制顺序 = 叠加顺序；最佳实践 = **先画 opaque 再画 transparent**，透明管线建议 `depth_write=False`（透明像素写深度会遮挡后续对象） |
| 窗口级 | `TRANSPARENT` 旗标 + `display.set_opacity(0..1)` | 创建期逐像素透出桌面（DX12 推导自动承载）+ 整窗均匀 alpha |

样张：`tilemap_layers_test.py` fx 层（`blend="alpha"` + WGSL `alpha 0.62` 水波 +
`depth_mode="disabled"`——透明管线标准形态）。

##### <span class="c-func">compute_pipeline(shader, bind_groups, entry="cs_main", label=None) -> ComputePipeline</span>

计算管线。

##### <span class="c-func">write_buffer(buf, data)</span>

数值列表（LE f32 bytes）或 Mat4 直写（自管相机正道：
`ac.write_buffer(cam_buf, cam.mvp())`）。

##### <span class="c-func">write_buffer_u32(buf, ints)</span>

u32 列表写入（indirect 绘制参数等整数语义数据）。

##### <span class="c-func">sampler(mag="linear", min="linear", mipmap="linear", address="clamp", label=None) -> Sampler</span>

采样器。filter = linear / nearest；address = clamp / repeat / mirror /
border。**compare 语义 v2 再放**。

##### <span class="c-func">texture(size, rgba=None, kind="sampled", label=None) -> Texture</span>

GPU 纹理。`rgba` = RGBA8 字节列表（w×h×4，None = 零填充）；`kind`:
`"sampled"` / `"rgba32f"`（空数据纹理——GPU 侧写入）。

### <span class="c-class">Mesh —— 网格句柄</span>

##### <span class="c-func">vertex_count() -> int / index_count() -> int</span>

顶点数 / 索引数（无索引 = 0）。

##### <span class="c-func">has_index_buffer() -> bool</span>

是否带索引。

### <span class="c-class">Texture —— 纹理句柄</span>

##### <span class="c-func">size() -> tuple[int, int]</span>

像素尺寸。

### <span class="c-class">透明句柄</span>

`Shader` / `Buffer` / `BindGroup` / `Pipeline` / `ComputePipeline` /
`Sampler` / `ColorAttachment`——无方法的透明句柄。

### <span class="c-class">RawPass —— 直绘会话</span>

`with screen.raw_render(depth=False) as p:`（主屏版——**目标级扁平
形态**，2026-11-12 从会话摘除；离屏 Surface 版 =
`with surface.raw_render() as p:`）。命令记录制——`__exit__` 排程到
目标提交时点回放，**恒在游戏内容之上**；深度恒清 1.0。帧纪律 =
**flip 前任意时点**（present 统一消费叠加队列，登记序）——主 2D
词汇仍走 `with screen.render()` 会话（gfxdraw/font + fill/quad/blit）。

**[管线缓存]**（2026-11 定约）`set_pipeline` / `set_bind_group` 在当前
with 块内缓存生效——同一 `Pipeline`（Arc 指针）/同 slot 同 `BindGroup`
指针的重复设置被跳过（零冗余命令）；管线变更自动清 bind 缓存。设置
一次后块内持续生效，无需每帧重设。

**[交叉顺序定约]**（2026-11-12）raw 内容经叠加通道在 **present 时
回放，恒在本帧全部 gfxdraw 内容之上**——gfxdraw → raw → gfxdraw
的脚本序在呈现时**必然乱序**（后画的 gfxdraw 仍被压在 raw 之下）。
可靠的层级手段只有：① gfx×gfx 之间按登记序；② raw×raw 之间按登记
序（"盖在 3D 之上的 2D" = 再开一段无深度 raw 画图元）。另：同帧
`depth=True` 的深度附件**只供第一条 raw 通道**（第二条起为 None）——
3D 内容同帧只放一段带深度 raw。

##### <span class="c-func">set_pipeline(pipeline)</span>

绑定渲染管线（同块内同管线重复设置 = 缓存命中跳过）。

##### <span class="c-func">set_bind_group(index, group)</span>

绑定组（slot 从 0 起；同 slot 同组重复设置 = 缓存命中跳过）。

##### <span class="c-func">set_mesh(mesh) / draw_mesh(mesh)</span>

绑定网格 / 全量绘制便利形态。

##### <span class="c-func">draw(first, end) / draw_indexed(first, end)</span>

顶点范围 / 索引范围绘制。

##### <span class="c-func">set_scissor(x, y, w, h) / set_viewport(x, y, w, h, min_depth, max_depth)</span>

像素裁剪 / 视口。

##### <span class="c-func">draw_indirect(buf, offset) / draw_indexed_indirect(buf, offset)</span>

间接绘制（buffer 须 `usage="indirect"` + u32 参数写入）。

##### <span class="c-func">set_stencil_reference(value) / set_blend_constant(r, g, b, a)</span>

模板动态参考值 / 混合动态常量。

##### <span class="c-func">push_debug_group(label) / pop_debug_group() / insert_debug_marker(label)</span>

调试分组 / 标注（RenderDoc / Nsight 捕获用）。

### <span class="c-class">RawCompute —— 计算会话</span>

`with screen.raw_compute() as p:`（目标级扁平形态，同 raw_render
2026-11-12 摘除；离屏 Surface 版 = `surface.raw_compute()`）——
排程到 present 收口，独立 compute pass。

##### <span class="c-func">set_pipeline(pipeline)</span>

绑定计算管线。

##### <span class="c-func">set_bind_group(index, group)</span>

绑定组。绑定位按登记序自动排列，`index` 参数实际被忽略。

##### <span class="c-func">dispatch(x, y=1, z=1)</span>

工作组派发。

##### <span class="c-func">copy_buffer(src, dst, size)</span>

storage → uniform GPU 闭环。

##### <span class="c-func">copy_mesh_vertices(mesh, src, size)</span>

storage → mesh 顶点缓冲直搬（GPU 粒子 instanced 通路）。

##### <span class="c-func">copy_mesh_indices(mesh, src, size)</span>

storage → mesh **索引缓冲**直搬（动态索引通路——粒子重生序/实例筛
选 GPU 侧维护）。无索引 mesh（建时未给 indices/index_bytes）回放期
跳过 + 告警；超容量截断告警。

##### <span class="c-func">copy_buffer_to_texture(src, tex, bytes_per_row, w, h)</span>

storage → 采样纹理（**bytes_per_row 须 256 对齐**）。

### <span class="c-class">GpuSettings —— GPU 装配配置</span>

`pygame.display.set_mode` 的 `gpu_settings=` kwargs 承载（**从平台
默认起手**应用字段——Android 默认 GL 等特判保留；**仅首次 GPU 装配
消费**——宿主先行装配形态走 display.md 的 env 通道）。

```python
gpu = pygame.render.GpuSettings(memory_hints="memory_usage",
                                power_preference="low_power")
screen = pygame.display.set_mode((800, 600), gpu_settings=gpu)
```

- `backends`：`"all"` / `"vulkan"` / `"dx12"` / `"metal"` / `"gl"`
  （逗号组合）——默认平台最优（Android = GL）
- `power_preference`：`"high_performance"`（默认）/ `"low_power"` /
  `"none"`
- `memory_hints`：`"performance"`（默认——分配器保池）/ `"memory_usage"`
  （省内存——频繁回收，**空项目 WS 实测 -47%**）
- `dx12_swapchain_kind`：`"hwnd"`（默认）/ `"visual"`（逐像素透明
  专属）

### <span class="c-class">SurfaceSettings —— 表面/交换链配置</span>

`set_mode` 的 `surface_settings=` kwargs 承载（仅首次装配消费）。

- `present_mode`：`"fifo"`（默认，三端通吃）/ `"fifo_relaxed"` /
  `"mail_box"` / `"immediate"` / `"auto_no_vsync"`
- `desired_maximum_frame_latency`：交换链帧数（默认 2；`1..=8`——
  1 更省内存）
- `sample_count`：MSAA（默认 1 = 关；渲染进表面的管线须同
  sample_count 建）
- `alpha_mode`：`"pre_multiplied"` / `"post_multiplied"` / `"opaque"`
  / `"auto"`（默认 None = 引擎定——透明窗口走 TRANSPARENT 旗标自动
  PreMultiplied，勿手动覆盖）
- `color_format` / `depth_format`：`"rgba8unorm"` / `"bgra8unorm"` /
  `"rgba8unorm_srgb"` / `"bgra8unorm_srgb"` / `"depth24plus"` /
  `"depth32float"` / `"depth24plus_stencil8"`（None = 引擎定）

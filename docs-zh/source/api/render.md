# pygame.render

底层渲染口——全局相机、自建 WGSL 管线资源工厂、`raw_render()` /
`raw_compute()` 直绘与计算会话。**进阶专用**：常规游戏绘制走
`with screen.render() as r:`（会话方法见 Screen / Surface）。

```{note}
[相机定约] 相机**全局唯一**——Screen / Surface 无相机字段，全部绘制
按全局相机状态解 MVP。`Auto` = 按目标尺寸隐式（resize 天然跟随）；
`Manual` = `set_default_camera()` override，一处设置全体生效。

[严格精细控制] **引擎不做相机兜底**——自管相机 buffer 由用户全权管理
（同帧多 raw 会话须各持独立 buffer，共享会被后写覆盖）。

[wgpu 红线] wgpu 裸对象（Device / Queue / Buffer / TextureView）**零出
Python 界**——一切经本模块句柄类安全收参。
```

## <span class="c-func">函数</span>

#### <span class="c-func">camera_ortho(width, height) -> Camera</span>

2D 正交相机——像素坐标直入（`(0,0)` 左上 / `(w,h)` 右下，y 向下）。

- :rtype: Camera

#### <span class="c-func">camera_perspective(fov_degrees, aspect, near, far) -> Camera</span>

3D 透视相机（fov 度数制；右手系，看向 `-Z`；位姿接 `.with_view()`）。

- :rtype: Camera

#### <span class="c-func">set_default_camera(camera)</span>

全局相机 override（后续全部渲染生效）。

#### <span class="c-func">camera_mvp() -> Mat4</span>

当前全局 MVP（Auto 态按主屏尺寸现算）。

- :rtype: Mat4

#### <span class="c-func">present_fit() -> Mat4</span>

呈现拟合矩阵——自管相机的 letterbox 对齐口（自绘管线左乘它即与主
绘制同一映射）。桌面 / 全屏恒等；Android 窗口化把自绘内容收进居中
画布。

- :rtype: Mat4

#### <span class="c-func">camera_state() -> str</span>

相机状态：`"auto2d"` / `"auto3d"` / `"manual"`。

- :rtype: str

#### <span class="c-func">init(dim=None, fov=None, near=None, far=None, cull=None, blend=None)</span>

渲染管线愿望（`set_mode` **前**设置；部分覆盖——未传保留现值，后设
优先）。`dim`: 2 / 3；`fov` / `near` / `far`：Auto3d 相机参数；
`cull`: `"none"`（默认——镜像矩阵绕序天然安全）/ `"back"` / `"front"`
（管线编译期定型，下次 set_mode 生效）；`blend`:
`"alpha"`（默认）/ `"opaque"` / `"premultiplied"`。

#### <span class="c-func">access(offscreen=False) -> ResourceAccess</span>

底层资源入口（自建管线建造工厂；须 `set_mode` 后，否则 RuntimeError）。

**[差异]** 主屏版默认颜色格式 = 交换链格式（平台漂移：Web/macOS
Bgra8、Windows Rgba8）；`offscreen=True` 离屏版恒 Rgba8（管线与目标
格式严格配对）。

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

矩阵（列主 16 float）写入 Buffer（`group0/b0 = 64B mat4` 定约）。单参
= 全局相机 MVP（帧内快照）；双参 = 自管相机矩阵——引擎自动左乘呈现
拟合收进画布（脚本零矩阵知识）。

#### <span class="c-func">screen_frame_state() -> str</span>

帧生命周期状态（诊断——定位探针时序与跳帧）。

- :rtype: str

#### <span class="c-func">set_fb_dump(on)</span>

帧缓冲取证 dump 开关（诊断；Android 落 dev_log）。

#### <span class="c-func">screen_probe() -> list</span>

帧缓冲网格回读（**诊断专用**；flip 前调用有效）——
`[(x, y, r, g, b, a), ...]`，同时落日志行（Android = dev_log.txt
可取证）。

- :rtype: list

## <span class="c-class">类</span>

### <span class="c-class">Camera —— 相机</span>

直呼不可实例化——经 `camera_ortho` / `camera_perspective` 构造。

##### <span class="c-func">with_view(view) -> Camera</span>

替换视图矩阵（3D 位姿：look_at 等 Mat4；2D 恒 Identity）。返回新
Camera。

##### <span class="c-func">mvp() -> Mat4</span>

MVP（projection × view——唯一上传量；调试 / 教学口）。

### <span class="c-class">ResourceAccess —— 资源工厂</span>

`access()` 产物。

##### <span class="c-func">shader(wgsl, label=None) -> Shader</span>

WGSL 源码编译。

##### <span class="c-func">buffer(size, label=None, usage="uniform") -> Buffer</span>

raw 缓冲（字节自管——相机定约 64B）。`usage`:
`"uniform"`（默认）/ `"indirect"` / `"storage"`。

##### <span class="c-func">mesh(layout, vertices, indices=None, floats=None, label=None) -> Mesh</span>

顶点网格。`layout` 布局预设：`pos3`(3) / `pos3uv2`(5) / `pos3color4`(7)
/ `pos3uv2color4`(9) / `corner2`(2) / `inst_pos2`(2)（GPU 粒子
instanced 通路——pos 顶点属性直进 VS）；`vertices` 扁平数值列表（按
布局 stride 校验）；`vertices=None` 须同时给 `floats=N`（零填充容量
预留——百万级 Python 列表不可行）；`indices` u16 列表（0..=65535）。

##### <span class="c-func">update_mesh(mesh, vertices, indices=None)</span>

动态几何——顶点（/索引）就地更新，免每帧重建。**容量以建时为准**
（超出 = wgpu 越界校验错；更大容量请重建）。

##### <span class="c-func">bind_group(bindings, label=None) -> BindGroup</span>

绑定组——`bindings = [(slot, kind, obj), ...]`；`kind`:
`"uniform"`（Buffer）/ `"texture"`（Surface 或 Texture）/
`"texture_data"`（`rgba32f` Texture——VS texelFetch 通路）/
`"sampler"`（Sampler 或 None = 默认线性 clamp）/ `"storage"` /
`"storage_rw"`（storage Buffer）。

##### <span class="c-func">pipeline(shader, mesh, bind_groups, flavor="2d", topo="triangles", cull="none", depth=False, label=None) -> Pipeline</span>

渲染管线（按本 access 默认颜色格式建）。`flavor`: `"2d"` / `"3d"`；
`topo`: `triangles` / `lines`；`cull`: none / back / front；
**`depth=True` 须配 `raw_render(depth=True)`**（深度目标严格配对）。

##### <span class="c-func">compute_pipeline(shader, bind_groups, entry="cs_main", label=None) -> ComputePipeline</span>

计算管线。

##### <span class="c-func">write_buffer(buf, data)</span>

数值列表（LE f32 bytes）或 Mat4 直写（自管相机正道：
`ac.write_buffer(cam_buf, cam.mvp())`）。

##### <span class="c-func">write_buffer_u32(buf, ints)</span>

u32 列表写入（indirect 绘制参数等整数语义数据）。

##### <span class="c-func">sampler(mag="linear", min="linear", mipmap="linear", address="clamp", label=None) -> Sampler</span>

采样器。filter = linear / nearest；address = clamp / repeat / mirror /
border（同时应用 u/v/w）。**compare 语义 v2 再放**。

##### <span class="c-func">texture(size, rgba=None, kind="sampled", label=None) -> Texture</span>

GPU 纹理。`rgba` = RGBA8 字节列表（w×h×4，None = 零填充）；`kind`:
`"sampled"` / `"rgba32f"`（空数据纹理——GPU 侧写入，`"texture_data"`
绑定 + texelFetch 读）。

### <span class="c-class">Mesh —— 网格句柄</span>

##### <span class="c-func">vertex_count() -> int / index_count() -> int</span>

顶点数 / 索引数（无索引 = 0）。

##### <span class="c-func">has_index_buffer() -> bool</span>

是否带索引。

### <span class="c-class">Texture —— 纹理句柄</span>

##### <span class="c-func">size() -> tuple[int, int]</span>

像素尺寸。

### <span class="c-class">Shader —— 着色器句柄</span>

WGSL 着色器（`shader()` 产物；无方法——wgpu 裸对象不出 Python 界）。

### <span class="c-class">Buffer —— 缓冲句柄</span>

raw 缓冲（`buffer()` 产物；无方法）。

### <span class="c-class">BindGroup —— 绑定组句柄</span>

`bind_group()` 产物（无方法）。

### <span class="c-class">Pipeline —— 渲染管线句柄</span>

`pipeline()` 产物（无方法）。

### <span class="c-class">ComputePipeline —— 计算管线句柄</span>

`compute_pipeline()` 产物（无方法）。

### <span class="c-class">Sampler —— 采样器句柄</span>

`sampler()` 产物（无方法）。

### <span class="c-class">ColorAttachment —— 帧颜色附件</span>

`screen_attachment()` 产物——主屏当前帧颜色附件快照（查询 / 未来
自绘逃生口；不透明句柄）。

### <span class="c-class">RawPass —— 直绘会话</span>

`with t.raw_render(depth=False) as p:`（`t` = Screen / Surface；主屏版
须在 `with screen.render()` 块内，否则 TypeError）。命令记录制——
`__exit__` 排程到目标提交时点回放，**恒在游戏内容之上**；深度恒清
1.0（客人叠加定约）。

##### <span class="c-func">set_pipeline(pipeline)</span>

绑定渲染管线。

##### <span class="c-func">set_bind_group(index, group)</span>

绑定组（slot 从 0 起）。

##### <span class="c-func">set_mesh(mesh) / draw_mesh(mesh)</span>

绑定网格 / 全量绘制便利形态（自动 set_mesh + 按自身计数）。

##### <span class="c-func">draw(first, end) / draw_indexed(first, end)</span>

顶点范围 / 索引范围绘制（须 set_mesh）。

##### <span class="c-func">set_scissor(x, y, w, h) / set_viewport(x, y, w, h, min_depth, max_depth)</span>

像素裁剪 / 视口。

##### <span class="c-func">draw_indirect(buf, offset) / draw_indexed_indirect(buf, offset)</span>

间接绘制（buffer 须 `usage="indirect"` + u32 参数写入）。

##### <span class="c-func">set_stencil_reference(value) / set_blend_constant(r, g, b, a)</span>

模板动态参考值 / 混合动态常量。

##### <span class="c-func">push_debug_group(label) / pop_debug_group() / insert_debug_marker(label)</span>

调试分组 / 标注（RenderDoc / Nsight 捕获用）。

### <span class="c-class">RawCompute —— 计算会话</span>

`with t.raw_compute() as p:`（须在 `with screen.render()` 块内）——
排程到 present 收口，独立 compute pass，恒在渲染内容之后提交。

##### <span class="c-func">set_pipeline(pipeline)</span>

绑定计算管线。

##### <span class="c-func">set_bind_group(index, group)</span>

绑定组。**[差异]** 绑定位按登记序自动排（set_pipeline 后从 0 递增），
`index` 实际被忽略。

##### <span class="c-func">dispatch(x, y=1, z=1)</span>

工作组派发。

##### <span class="c-func">copy_buffer(src, dst, size)</span>

storage → uniform GPU 闭环（顶点段 SSBO 设备相关问题的规避路径）。

##### <span class="c-func">copy_mesh_vertices(mesh, src, size)</span>

storage → mesh 顶点缓冲直搬（GPU 粒子 instanced 通路）。尺寸守卫：
超目标容量 = 截断 + 告警（永不崩溃定约）。

##### <span class="c-func">copy_buffer_to_texture(src, tex, bytes_per_row, w, h)</span>

storage → 采样纹理（**bytes_per_row 须 256 对齐**）。

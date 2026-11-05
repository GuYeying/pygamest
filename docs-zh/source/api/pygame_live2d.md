# pygame_live2d

Live2D/Cubism 绑定——上游 [mocari](https://github.com/Eatgrapes/Mocari)
v0.4.0（MIT，纯 Rust Cubism 兼容运行时；上游仓已删，vendored 溯源见
pygame_live2d 仓 README）。**独立顶层 Python 模块**
（`import pygame_live2d` 直达）。

```{note}
[接口风格] 上游词汇原生化——canvas / drawable / parameter（Live2D
惯例即文档，不 pygame 化）。

[坐标系] Live2D 原生画布域（y 向下、原点画布左上）——与引擎中心系
换算脚本侧自理。

[双形态] `Live2DModel` = 静态数据面（default pose 的 meshes/textures）；
`Live2DRuntime` = 参数驱动面（每帧 set_parameter_normalized →
update_meshes → mesh 数据）。

[渲染接入] mesh(i) 数据流（顶点/uv/索引/纹理索引/不透明度）+ 纹理
rgba——引擎带贴图 mesh draw 面落地后接 GPU（与 pygame_spine
render_commands 同边界，v1 数据出口）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">load_model(path) -> Live2DModel</span>

`.model3.json` 入口（moc3/纹理相对路径按其目录解析）。缺文件/坏内容
= ValueError（不崩溃——严格模式同理，Live2D 面无静默兜底定约）。

#### <span class="c-func">load_runtime(path) -> Live2DRuntime</span>

参数驱动形态加载。

## <span class="c-class">类</span>

### <span class="c-class">Live2DModel —— 静态数据面</span>

##### <span class="c-func">canvas_size() -> (w, h)</span>

模型画布尺寸（Live2D 原生单位）。

##### <span class="c-func">mesh_count() -> int</span>

drawable 网格数。

##### <span class="c-func">mesh(i) -> (texture_index, opacity, positions, uvs, indices)</span>

网格数据流（positions/uvs 扁平 f32；indices u16）。

##### <span class="c-func">texture_count() -> int</span>

##### <span class="c-func">texture_info(i) -> (w, h)</span>

##### <span class="c-func">texture_rgba(i) -> bytes</span>

纹理原始 RGBA 字节（大对象——调试/Surface 构建取用）。

### <span class="c-class">Live2DRuntime —— 参数驱动面</span>

##### <span class="c-func">canvas_size() -> (w, h)</span>

##### <span class="c-func">parameter_ids() -> list[str]</span>

可驱动参数 id 表。

##### <span class="c-func">set_parameter_normalized(id, v) -> bool</span>

归一参数驱动（0..1；id 不存在返回 False）。

##### <span class="c-func">update_meshes()</span>

参数 → 网格重建（每帧参数更新后调用）。

##### <span class="c-func">mesh_count() -> int / mesh(i)</span>

数据面同 `Live2DModel.mesh`（读重建产物）。

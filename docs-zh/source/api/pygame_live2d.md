# pygame_live2d

```{warning}
⚠️ **使用前必读（两项前置条件）**

1. **Live2D IP/法律合规**：Live2D 为 Cubism Inc.（Live2D Inc.）的注册
   商标与受版权保护的技术——**模型资产须经正版 Live2D Editor 导出**
   （持有有效 Live2D 许可证）；分发模型资产须遵守 Live2D 各资产固有
   的使用条款。运行时适配层（mocari，MIT）本身不含 Live2D 专有代码。
2. **编译默认不含本库**：`pygame_live2d` 为 feature 门控的可选模块——
   构建时须显式激活 `features = ["live2d"]`（stable 可用），否则
   `import pygame_live2d` 即 ImportError。
```

Live2D/Cubism 绑定——上游 [mocari](https://github.com/Eatgrapes/Mocari)
v0.4.0（MIT，纯 Rust Cubism 兼容运行时；上游仓已删，vendored 溯源见
pygame_live2d 仓 README 与其 docs/API.md）。**独立顶层 Python 模块**
（`import pygame_live2d` 直达）。**归档形态**：实现/渲染管线/文档随
独立库 pygame_live2d 维护，本仓仅存薄绑定。

```{note}
[接口风格] 上游词汇原生化——canvas / drawable / parameter（Live2D
惯例即文档，不 pygame 化）。

[坐标系] 归一化顶点域（y 向上）；脚本侧用 `rt.draw` 的 x/y/scale 定位。

[sprite 工作流] 用户创建 Surface → `rt.draw(surface, x, y, scale)`
全管线自理（**蒙版纹理方案 GPU 化**——masked 双纹理，眼睛等蒙版
drawable 完整支持）→ 产物 = 全透明底人物图 →
`pygame.gfxdraw.draw_surface` 上屏。

[双形态] `Live2DModel` = 静态数据面；`Live2DRuntime` = 参数驱动面 +
渲染会话。
```

## <span class="c-func">函数</span>

#### <span class="c-func">load_model(path) -> Live2DModel</span>

`.model3.json` 入口（moc3/纹理相对路径按其目录解析）。缺文件/坏内容
= ValueError（不崩溃——严格模式同理，Live2D 面无静默兜底定约）。

#### <span class="c-func">load_runtime(path) -> Live2DRuntime</span>

参数驱动形态加载。

#### <span class="c-func">load_motion(path) -> Live2DMotion</span>

`.motion3.json` 加载（缺文件/坏内容 = ValueError）。

#### <span class="c-func">motion_player(motion, looping=None) -> Live2DMotionPlayer</span>

播放器构造（looping 缺省跟随 motion 自带 Loop 元数据）。

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

### <span class="c-class">Live2DRuntime —— 参数驱动面 + 渲染会话</span>

##### <span class="c-func">canvas_size() -> (w, h)</span>

##### <span class="c-func">parameter_ids() -> list[str]</span>

可驱动参数 id 表。

##### <span class="c-func">set_parameter_normalized(id, v) -> bool</span>

归一参数驱动（0..1；id 不存在返回 False）。

##### <span class="c-func">update_meshes()</span>

参数 → 网格重建（每帧参数更新后调用）。

##### <span class="c-func">mesh_count() -> int / mesh(i)</span>

数据面同 `Live2DModel.mesh`（读重建产物）。

##### <span class="c-func">draw(surface, x=0.0, y=0.0, scale=0.9)</span>

**全管线自理渲染**——蒙版/masked/整模型三段落 `surface`（产物=全透明
底人物图）。坐标定约（与 pygamest 中心系同构，全 0-1 相对产物面）：
`x`/`y` = 人物内容中心位置（0=面中心，±1=±半幅，y 向上）；
`scale` = 人物内容高/面高（>1 溢出裁边=特写）。

- :param surface: 用户创建的产物面（每次 draw 传入——同一 runtime
  可画任意尺寸/多个面）
- ⚠️ 每帧调用序：`update_meshes()` 后 `draw(...)`；产物面上屏走
  `pygame.gfxdraw.draw_surface`

### <span class="c-class">Live2DMotion</span>

##### <span class="c-func">duration() -> float</span>

motion 时长（秒）。

### <span class="c-class">Live2DMotionPlayer</span>

##### <span class="c-func">tick(dt) / apply(model) / is_finished() / restart() / time() / set_weight(w)</span>

时间轴推进 / 参数写入 / 播完查询 / 回零 / 时间查询 / 混合权重。

## <span class="c-func">渲染集成 —— sprite 工作流</span>

```python
buf = pygame.Surface((800, 600))            # 产物面（尺寸=清晰度预算）
rt = pygame_live2d.load_runtime(path, buf)  # 注入
# 每帧：
rt.update_meshes()
rt.draw(buf, scale=0.9)                     # 全管线自理
with screen.render() as t:
    pygame.gfxdraw.draw_surface(t, buf)     # 上屏/摆放/多层复用
```

资产：Hiyori 官方样例模型已 vendor 至 pygame_live2d 仓
`assets/models/Hiyori/`（Live2D Free Material License，经 neocari
分发副本）。demo：`pygame-rs-dev/examples/live2d_render_demo.py`。

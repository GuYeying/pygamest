# pygame_ozz

```{warning}
⚠️ **使用前必读**：本模块**编译默认不含**——feature `ozz` 门控且需
**nightly rustc**（上游 std::simd；wasm 需 simd128 target-feature）。
构建：`cargo +nightly build --features ozz`，否则
`import pygame_ozz` 即 ImportError。
```

3D 骨骼动画绑定——上游
[ozz-animation-rs](https://github.com/SlimeYummy/ozz-animation-rs)
（MPL-2.0，ozz-animation 的 Rust 移植），经 pygame_ozz 兄弟 crate 的
`Player` 会话（采样→局部→模型矩阵三段 job 薄封装）。**独立顶层 Python
模块**（`import pygame_ozz` 直达）。

```{note}
[工具链] 上游使用 std::simd——**需 nightly rustc 构建**。feature `ozz`
门控（不在 default——stable 默认构建不含本模块）；宿主侧
`cargo +nightly build --features ozz` 激活后 import 可用。

[数据流] `demo_player()` / `Player(skeleton_bytes, animation_bytes)`
→ `sample(ratio)` → `model_matrices()`（每关节 16 f32 列主矩阵流
——引擎蒙皮 mesh draw 面落地后接 GPU；v1 数据出口）。

[测试链] `demo_player()` 内嵌 ozz playback 测试资产（零资产直构）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">demo_player() -> OzzPlayer</span>

playback 测试资产直构（零资产测试链）。

## <span class="c-class">类</span>

### <span class="c-class">OzzPlayer —— 采样会话</span>

#### <span class="c-func">Player(skeleton_bytes, animation_bytes)</span>

从 ozz archive 字节构建（.ozz 二进制——ozz offline 工具导出）。

##### <span class="c-func">num_joints() -> int</span>

关节数。

##### <span class="c-func">joint_names() -> list[str]</span>

关节名表（骨序）。

##### <span class="c-func">duration() -> float</span>

动画时长（秒）。

##### <span class="c-func">sample(ratio)</span>

采样（0..1 归一动画时间）→ 模型矩阵重建。

##### <span class="c-func">model_matrices() -> list[float]</span>

模型矩阵流（每关节 16 f32 列主——上一次 sample 的缓存）。

## <span class="c-func">渲染集成 —— gfxdraw.draw_ozz_bones</span>

#### <span class="c-func">pygame.gfxdraw.draw_ozz_bones(f, player, scale=1.0, cx=0.0, cy=0.0)</span>

ozz 骨架骨骼线渲染——父子关节世界位 xy × scale + 中心偏移（Auto2d
中心系直入；ozz y 向上与引擎同构零翻转）。蒙皮 mesh 面落地前的骨骼
可视化形态；sample(ratio) 每帧先行。

- :param f: 渲染会话
- :param player: OzzPlayer（sample 后调用）
- 另有 `joint_positions_xy()` / `joint_parents()` 数据口（自投影 /
  自绘用）。demo：`pygame-rs-dev/examples/ozz_render_demo.py`

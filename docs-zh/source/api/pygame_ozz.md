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

采样动画（`ratio` 为 0..1 的归一化动画时间），重建全部关节的模型
矩阵——每帧绘制前调用一次：

```python
player.sample(t / player.duration())
```

##### <span class="c-func">model_matrices() -> list[float]</span>

返回模型矩阵流（每关节 16 个 f32，列主序——上一次 `sample()` 的
缓存）。

##### <span class="c-func">joint_positions_xy() -> list[float]</span>

返回全部关节的世界坐标（x, y 交替平铺）——自投影或自绘用。

##### <span class="c-func">joint_parents() -> list[int]</span>

返回每个关节的父关节序号（根关节为 -1）——骨架拓扑遍历用。

## 渲染集成

`pygame.gfxdraw.draw_ozz_bones()` 直接绘制骨架骨骼线（父子关节连线
× 缩放 + 中心偏移；ozz 的 y 向上与引擎同构，零翻转）——蒙皮网格
落地前的可视化形态：

```python
while True:
    player.sample((pygame.time.get_ticks() % dur_ms) / dur_ms)
    with screen.render() as f:
        pygame.gfxdraw.draw_ozz_bones(f, player, scale=80.0, cx=0, cy=0)
    await pygame.tasks.yield_now()
```

数据口 `joint_positions_xy()` / `joint_parents()` 支持自投影与自绘。
完整演示见 `pygame-rs-dev/examples/ozz_render_demo.py`。

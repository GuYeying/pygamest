# pygame_spine

Spine 2D 骨骼动画绑定——上游 [chine](https://crates.io/crates/chine)
（纯 Rust Spine 运行时），适配层 = `pygame_spine` 兄弟 crate（阶段 4；
rapier-starfish-debug 同范式）。**独立顶层 Python 模块**
（`import pygame_spine` 直达）。

```{note}
[接口风格] 上游词汇原生化——SkeletonData / Skeleton / AnimationState
三件套（Spine 惯例即文档，不 pygame 化）。

[坐标系] Spine 原生 y 向上——与引擎中心系同构，零换算直出。

[渲染接入] `render_commands()` = RenderCommand 数据流（page/顶点/uv/
三角索引/tint）——引擎带贴图三角网格 draw 面落地后即接 GPU 管线；
v1 为数据出口（脚本可经 raw_render 自绘或先行消费数据）。

[测试链] `demo_skeleton()` 程序化三骨链（root→arm→hand）——零资产
即可跑通绑定全链。
```

## <span class="c-func">函数</span>

#### <span class="c-func">load_json(text) -> SkeletonData</span>

Spine JSON 导入。

#### <span class="c-func">load_binary(bytes) -> SkeletonData</span>

Spine 二进制导入（.skel）。

#### <span class="c-func">demo_skeleton() -> SkeletonData</span>

程序化三骨链（无资产依赖——绑定/测试链即装即跑）。

## <span class="c-class">类</span>

### <span class="c-class">SkeletonData —— 资产共享面</span>

##### <span class="c-func">find_animation(name) -> Animation | None</span>

按名检索动画（产物直入 `AnimationState.set_animation`）。

##### <span class="c-func">bone_count() -> int / bone_name(i) -> str</span>

骨骼数 / 骨骼名。

### <span class="c-class">Skeleton —— 实例姿态面</span>

#### <span class="c-func">Skeleton(data)</span>

从共享数据实例化（setup pose 起始）。

##### <span class="c-func">set_skin(name)</span>

激活皮肤。

##### <span class="c-func">update_world_transform()</span>

世界变换解算（pose 变更后、查询/渲染前调用）。

##### <span class="c-func">bone_world(i) -> (x, y, a, b, c, d)</span>

骨骼世界变换（2×3 仿射 + 世界原点）。

##### <span class="c-func">set_bone_rotation(i, deg)</span>

局部旋转（度）。

##### <span class="c-func">set_position(x, y) / set_scale(sx, sy)</span>

骨架整体平移 / 缩放（负值翻转）。

##### <span class="c-func">render_commands() -> list</span>

渲染命令流——每条 `(page, r, g, b, a, positions, uvs, triangles)`。

### <span class="c-class">Animation / AnimationState —— 播放状态机</span>

##### <span class="c-func">Animation.duration() -> float / name() -> str</span>

时长（秒）/ 名称。

#### <span class="c-func">AnimationState()</span>

状态机实例。

##### <span class="c-func">set_animation(anim, looping=True)</span>

track 0 设当前动画。

##### <span class="c-func">update(dt)</span>

推进时间轴（帧循环调用；dt = 秒）。

##### <span class="c-func">apply(skeleton)</span>

姿态写进骨架（写后调 `update_world_transform()` 再查询/渲染）。

## <span class="c-func">渲染集成 —— gfxdraw.draw_spine</span>

#### <span class="c-func">pygame.gfxdraw.draw_spine(f, skeleton, pages)</span>

spine 骨架渲染——`render()` 命令流内部直推会话 batch（渲染中心在
pygame；uv 翻 v，positions 为 spine 世界坐标直入 Auto2d 中心系）。

- :param f: 渲染会话（`with screen.render() as f`）
- :param skeleton: 已 update_world_transform 的骨架
- :param pages: 图集页 Surface 表（`pages[cmd.page]` 索引；demo 的
  程序化 region = page0 单页）
- 程序化数据源：`pygame_spine.demo_textured_skeleton()`（三骨链 +
  body/hand 两 region 方块 + default skin——零资产渲染链）。
  demo：`pygame-rs-dev/examples/spine_render_demo.py`

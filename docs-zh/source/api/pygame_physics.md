# pygame_physics

rapier 米制物理——刚体 / 碰撞体 / 固定步长积分 / 射线查询 / 碰撞事件，
附 GPU 调试线渲染套件。**独立顶层模块**（`import pygame_physics`）。
接口定约"第三方库等价物，不 pygame 化"——rapier 0.36 词汇，builder
配置全走**构造 kwargs**（原生链式 builder 在这里是
`RigidBodyBuilder("dynamic", translation=...)`）。

```{note}
[双维度] 单模块双维度：2D 类**无后缀**（`PhysicsWorld` 等，随
`physics2d` feature）、3D 类带 **3D 后缀**（`PhysicsWorld3D` 等，随
`physics3d` feature）——可双开，同名类分属不同后缀互不冲突。

[坐标契约] 物理世界 = **米制、y 向上、原点在视口中心**——与 pygame
像素坐标（y 向下、左上原点）不同系，调试线独立成景。

[帧序] event 泵 → `step_frame(dt)` → `collect` → 游戏会话绘制 →
`t.draw_debug(renderer, sink)`（叠加通道）→ `flip`——调试线必须在
render 会话之后、flip 之前（画家序；放会话内会被游戏 batch 覆盖）。

[rapier 行为] 默认把静止体画暗（亮度 ×0.2）——"物体不动就变暗"是
rapier 睡眠状态编码，非 bug。
```

## <span class="c-func">函数</span>

#### <span class="c-func">collect(world, draw, sink)</span>

2D 物理状态 → 线段流（每帧渲染前调用；sink 内部先清后收，零分配
复用）。

- :param world: 物理世界
- :type world: PhysicsWorld
- :param draw: 渲染内容位集
- :type draw: DebugDraw
- :param sink: 线段收集器
- :type sink: LineSink

#### <span class="c-func">collect3d(world, draw, sink)</span>

3D 版（`PhysicsWorld3D` / `DebugDraw3D` / `LineSink3D`）。

## <span class="c-class">2D 类面</span>

### <span class="c-class">PhysicsWorld —— 物理世界</span>

#### <span class="c-func">PhysicsWorld(gravity=(0, -9.81), dt=1/60)</span>

建世界（米/秒²，y 向上为正；`dt` 固定步长秒）。

##### <span class="c-func">step_frame(real_dt) -> int</span>

真实帧距累积积分（内部固定步长；上限节流防死亡螺旋）——返回实际
执行步数，0 = 余量不足一步。碰撞事件仅此路径收集。

##### <span class="c-func">step() / set_dt(dt) / sim_time() -> float</span>

手动固定步进一次 / 改步长 / 已仿真时长（与真实时间解耦）。

##### <span class="c-func">insert(body_builder, collider_builder) -> tuple[int, int]</span>

插入刚体 + 碰撞体——返回 `(body_handle, collider_handle)`。**handle
是不透明 int**（黑盒，回传给查询方法即可）。builder **一次性消费**
（二次 insert 抛 ValueError，重建再插）。

##### <span class="c-func">读面</span>

`translation(h) -> (x, y)`（米）/ `rotation(h) -> float`（弧度）/
`linvel(h) -> (vx, vy)` / `is_sleeping(h) -> bool`（休眠态）。

##### <span class="c-func">写入面</span>

`set_translation(h, pos)`（瞬移）/ `set_rotation(h, angle)` /
`set_linvel(h, v)` / `set_angvel(h, w)` / `apply_impulse(h, imp)` /
`wake_up(h)`——全部**自动唤醒**；`set_gravity(g)`（下一帧生效）；
`remove_body(h) -> bool`（连带全部碰撞体）。

##### <span class="c-func">drain_events() -> list</span>

排空自上帧以来的碰撞事件——`[(kind, h1, h2), ...]`，`kind` =
`"started"` / `"stopped"`。**仅 `events=True` 的碰撞体产生事件**
（传感器触发器同此通道）；每帧调用一次，通道缓冲按步累积。

##### <span class="c-func">cast_ray(origin, dir, max_toi=None) -> tuple | None</span>

射线查询——命中返回 `(collider_handle, 距离)`。`dir` 非单位向量按其
模长定射线长；`max_toi` 为其倍数上限（None = 缺省 100.0）。
**⚠️ 时序契约：insert 后须至少 step 一次方可查询**（broad_phase
时序——实测立即查询返回 None）。

##### <span class="c-func">intersection_pair(h1, h2) -> bool</span>

两碰撞体当前是否相交。

### <span class="c-class">RigidBodyBuilder —— 刚体 builder</span>

#### <span class="c-func">RigidBodyBuilder(kind, translation=None, rotation=None, linvel=None, angvel=None, can_sleep=True)</span>

`kind`: `"dynamic"` / `"fixed"` / `"kinematic"`；`rotation` 弧度；
单位：米 / 米每秒。

### <span class="c-class">ColliderBuilder —— 碰撞体 builder</span>

#### <span class="c-func">ColliderBuilder(kind, hx=None, hy=None, radius=None, translation=None, friction=None, restitution=None, density=None, sensor=False, events=False)</span>

`kind`: `"cuboid"`（须给 hx/hy 半尺寸，米）/ `"ball"`（须给 radius）
——**[差异] v1 仅此两种**。`sensor` = 传感器（只碰不撞）；`events` =
产生碰撞事件（缺省 false 省开销）。

### <span class="c-class">调试渲染三件套</span>

#### <span class="c-func">DebugDraw(collider_shapes=None, contacts=None, joints=None, aabbs=None, axes=None)</span>

渲染内容位集（缺省 = 形状 + 接触开）。
`set_mode(...)` 运行期改位（只更新给出的位）。

#### <span class="c-func">LineSink()</span>

线段收集器——`line_count() -> int` / `is_empty() -> bool` /
`clear()`（保 capacity）。

#### <span class="c-func">DebugLinesRenderer(ppm=50.0)</span>

调试线绘制器（ortho 相机；GPU 惰性装配——首次 draw 须已 set_mode）。
`set_camera(ppm)` 改缩放（下次 draw 生效）；`follow_global_camera()`
切全局相机跟随——**[差异]** 2D 调试线世界坐标为米，Auto2d 全局相机
按像素正交，米坐标直入会过小——跟随时请配 Manual 相机。

#### <span class="c-func">t.draw_debug(renderer, sink)</span>（会话方法）

叠加绘制——t = `with screen.render()`（或 `surf.render()`）的会话
对象；排入游戏 batch 之后的叠加通道（Screen = present 收口，
Surface = flush 收口 / 画中画物理动画）。每帧一次。

## <span class="c-class">3D 类面</span>

与 2D 同款语义，差异四条：相机 = look-at 透视；调试线通道**必须带
深度附件**；rotation 收**欧拉角向量** `(rx, ry, rz)` 弧度、查询返回
**四元数** `(x, y, z, w)`；坐标同为米制 y 向上原点居中。

### <span class="c-class">PhysicsWorld3D —— 物理世界</span>

#### <span class="c-func">PhysicsWorld3D(gravity=(0, -9.81, 0), dt=1/60)</span>

方法面与 2D 同构（`step_frame` / `insert` / 读面 / 写入面 /
`drain_events` / `cast_ray` / `intersection_pair`），维度换 3：
`translation(h) -> (x, y, z)`、`rotation(h) -> (x, y, z, w)` 四元数、
`set_rotation(h, (rx, ry, rz))` 欧拉角、`apply_impulse(h, (ix, iy, iz))`。

### <span class="c-class">RigidBodyBuilder3D / ColliderBuilder3D</span>

`RigidBodyBuilder3D(kind, translation=None, rotation=None, linvel=None,
angvel=None, can_sleep=True)`——`rotation` 收**欧拉角向量**。
`ColliderBuilder3D(kind, hx=None, hy=None, hz=None, radius=None, ...)`
——cuboid 须给 hx/hy/hz 半尺寸；**v1 仅 cuboid / ball**。

### <span class="c-class">DebugDraw3D / LineSink3D / DebugLinesRenderer3D</span>

`DebugDraw3D(...)` 内容位集构造期定格。`LineSink3D()` 同 2D 三方法。
`DebugLinesRenderer3D(eye=(0, 6, 14), target=(0, 0, 0), fov=1.0)`——
look-at 透视相机（fov = 垂直视场角**弧度**，1.0 ≈ 57°）；
`set_camera_lookat(eye, target, fov)` 改机位（宽高比自动跟随）；
`follow_global_camera()` 切全局跟随（再调 set_camera_lookat 解除）。

#### <span class="c-func">t.draw_debug_3d(renderer, sink)</span>（会话方法）

3D 叠加绘制——**带深度通道**（Depth24Plus 硬契约）。Surface 变体
须以 `depth_buffer=True` 创建离屏面（无深度面报错并指路）。

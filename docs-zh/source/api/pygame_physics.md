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

[handle] `insert()` 返回的 handle 是**不透明 int**（黑盒，回传给查询
方法即可，勿自行解包）；无效 handle 一律 ValueError。
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

3D 版——`PhysicsWorld3D` / `DebugDraw3D` / `LineSink3D`。

## <span class="c-class">2D 类面</span>

### <span class="c-class">PhysicsWorld —— 物理世界</span>

#### <span class="c-func">PhysicsWorld(gravity=(0, -9.81), dt=1/60)</span>

建世界。`gravity` 米/秒²（y 向上为正）；`dt` 固定步长秒。

##### <span class="c-func">step_frame(real_dt) -> int</span>

真实帧距累积积分（内部固定步长；上限节流防死亡螺旋）——返回实际
执行步数，0 = 余量不足一步。碰撞事件仅此路径收集。

- :type real_dt: float
- :rtype: int

##### <span class="c-func">step()</span>

固定步长积分一次（不经事件收集）。

##### <span class="c-func">set_dt(dt)</span>

设固定步长（秒）。

- :type dt: float

##### <span class="c-func">sim_time() -> float</span>

已仿真时长（秒；与真实时间解耦）。

- :rtype: float

##### <span class="c-func">insert(body_builder, collider_builder) -> tuple[int, int]</span>

插入刚体 + 碰撞体——返回 `(body_handle, collider_handle)`。
builder **一次性消费**（二次 insert 抛 ValueError，重建再插）。

##### <span class="c-func">translation(h) -> tuple[float, float]</span>

刚体平移（米，世界系）。

##### <span class="c-func">rotation(h) -> float</span>

刚体姿态（弧度）。

##### <span class="c-func">linvel(h) -> tuple[float, float]</span>

线速度（米/秒）。

##### <span class="c-func">is_sleeping(h) -> bool</span>

静止体休眠态。

##### <span class="c-func">set_translation(h, pos)</span>

瞬移（自动唤醒）。

- :type pos: tuple[float, float]

##### <span class="c-func">set_rotation(h, angle)</span>

设姿态（弧度；自动唤醒）。

##### <span class="c-func">set_linvel(h, vel)</span>

设线速度（自动唤醒）。

- :type vel: tuple[float, float]

##### <span class="c-func">set_angvel(h, w)</span>

设角速度（自动唤醒）。

##### <span class="c-func">apply_impulse(h, impulse)</span>

施加冲量（自动唤醒）。

- :type impulse: tuple[float, float]

##### <span class="c-func">wake_up(h)</span>

唤醒休眠体。

##### <span class="c-func">set_gravity(g)</span>

改重力（下一帧生效）。

- :type g: tuple[float, float]

##### <span class="c-func">remove_body(h) -> bool</span>

移除刚体（连带其全部碰撞体）；返回是否存在并已移除。

##### <span class="c-func">create_joint(kind, body1, body2, ...) -> int</span>

脉冲关节创建——返回关节句柄（`remove_joint` 消费）。
**v1 kind = `"spring"`**（弹簧约束）：`rest_length` 自然长（米）、
`stiffness` 刚度、`damping` 阻尼（三者必填）；`anchor1` / `anchor2` =
各自体的局部系锚点（缺省质心）；`contacts_enabled` 两体间碰撞保留
（缺省 False——弹簧互连体互不碰撞）。

- ⚠️ **panic 注意**：`rest_length ≤ 0` 会触发上游断言 panic——绑定层
  已防线（ValueError），但请勿绕过绑定直接构造
- ⚠️ **debug**：`DebugDraw(set_mode(joints=True))` 后 rapier 自绘关
  节线（impulse joint 专属配色——与碰撞线框区分；弹簧不做花哨渲染）
- :param kind: `"spring"`（其余关节族待接口扫描批次）
- :param body1/body2: `insert` 产物刚体句柄

##### <span class="c-func">remove_joint(h) -> bool</span>

移除脉冲关节；返回是否存在并已移除。

##### <span class="c-func">drain_events() -> list[tuple[str, int, int]]</span>

排空自上帧以来的碰撞事件——`[(kind, h1, h2), ...]`，`kind` =
`"started"` / `"stopped"`。**仅 `events=True` 的碰撞体产生事件**
（传感器触发器同此通道）；每帧调用一次，通道缓冲按步累积。

##### <span class="c-func">drain_force_events() -> list[tuple[int, int, float, float]]</span>

排空冲击力事件——`[(h1, h2, 总力幅值, 最大接触力幅值), ...]`。
**仅 `force_events=True` 的碰撞体产生**（`force_threshold` 设门槛，
总力幅值超过才发——落体伤害判定/击打反馈的判定源）。

##### <span class="c-func">set_collider_groups(handle, memberships, filter)</span>

运行时改碰撞分组（句柄取自 `drain_events` / `drain_force_events`
或 `insert` 产物——换阵营/临时免伤的运行时开关）。

##### <span class="c-func">cast_ray(origin, dir, max_toi=None) -> tuple[int, float] | None</span>

射线查询——命中返回 `(collider_handle, 距离)`。`dir` 非单位向量按其
模长定射线长；`max_toi` 为其倍数上限（None = 缺省 100.0）。
**⚠️ 时序契约：insert 后须至少 step 一次方可查询**（broad_phase
时序——实测立即查询返回 None）。

##### <span class="c-func">intersection_pair(h1, h2) -> bool</span>

两碰撞体当前是否相交。

### <span class="c-class">RigidBodyBuilder —— 刚体 builder</span>

#### <span class="c-func">RigidBodyBuilder(kind, translation=None, rotation=None, linvel=None, angvel=None, can_sleep=True, linear_damping=None, angular_damping=None, ccd_enabled=None, lock_rotations=None)</span>

- :param kind: 刚体类型——`"dynamic"` / `"fixed"` / `"kinematic"`
- :type kind: str
- :param translation: 初始平移（米）
- :type translation: tuple[float, float] | None
- :param rotation: 初始姿态（弧度）
- :type rotation: float | None
- :param linvel: 初始线速度（米/秒）
- :type linvel: tuple[float, float] | None
- :param angvel: 初始角速度（弧度/秒）
- :type angvel: float | None
- :param can_sleep: 允许休眠（缺省 True）
- :type can_sleep: bool
- :param linear_damping: 线性阻尼（缺省 0；数值越大停得越快）
- :type linear_damping: float | None
- :param angular_damping: 角阻尼
- :type angular_damping: float | None
- :param ccd_enabled: 连续碰撞检测（高速穿墙防线）
- :type ccd_enabled: bool | None
- :param lock_rotations: 锁定旋转（不倒翁/角色胶囊）
- :type lock_rotations: bool | None

### <span class="c-class">ColliderBuilder —— 碰撞体 builder</span>

#### <span class="c-func">ColliderBuilder(kind, hx=None, hy=None, radius=None, translation=None, friction=None, restitution=None, density=None, sensor=False, events=False, groups_memberships=None, groups_filter=None, force_events=False, force_threshold=None)</span>

- :param kind: 形状——`"cuboid"`（须给 hx/hy 半尺寸，米）/
  `"ball"`（须给 radius）/ `"capsule"`（须给 half_height + radius，
  纵轴 y）/ `"segment"`（须给 ax ay bx by 端点）/ `"triangle"`
  （须给 x1 y1 x2 y2 x3 y3 三顶点）
- :type kind: str
- :param translation: 相对刚体原点的偏移（米）
- :type translation: tuple[float, float] | None
- :param sensor: 传感器（只碰不撞，缺省 False）
- :type sensor: bool
- :param events: 产生碰撞事件（`drain_events` 只报开启者——触发器 /
  碰撞检测入口，缺省 False 省开销）
- :type events: bool
- :param groups_memberships: 碰撞分组——本体所属组位集（与
  `groups_filter` 同给生效；bit i = 组 i：本体只与 filter 含
  本体所属组的物体碰撞——阵营互斥/区域门禁）
- :type groups_memberships: int | None
- :param groups_filter: 可碰撞组位集
- :type groups_filter: int | None
- :param force_events: 产生冲击力事件（`drain_force_events` 产出）
- :type force_events: bool
- :param force_threshold: 力事件门槛（总力幅值超过才发）
- :type force_threshold: float | None

```{note}
[3D 对应面] `PhysicsWorld3D` / `RigidBodyBuilder3D` / `ColliderBuilder3D`
同款新增：`capsule` / `cylinder` / `cone`（half_height + radius，纵轴
y）/ `trimesh`（vertices 扁平 xyz + indices 扁平三角索引——体素地形/
任意静态网格）；`drain_force_events` / `set_collider_groups` 同名同约。
3D `rotation` 为欧拉角向量 (rx, ry, rz) 弧度；`lock_rotations` =
姿态四元数冻结。
```

### <span class="c-class">DebugDraw —— 调试渲染控制面</span>

#### <span class="c-func">DebugDraw(collider_shapes=None, contacts=None, joints=None, aabbs=None, axes=None)</span>

渲染内容位集（缺省 = 形状 + 接触开）。

##### <span class="c-func">set_mode(collider_shapes=, contacts=, joints=, aabbs=, axes=)</span>

运行期改渲染内容位（只更新给出的位，未给的保持不变）。

### <span class="c-class">LineSink —— 线段收集器</span>

#### <span class="c-func">LineSink()</span>

`collect()` 的落点（每帧复用零分配）。

##### <span class="c-func">line_count() -> int</span>

当前线段数（collect 后有效）。

##### <span class="c-func">is_empty() -> bool</span>

是否无线段。

##### <span class="c-func">clear()</span>

清空缓冲（保 capacity）。

### <span class="c-class">DebugLinesRenderer —— 调试线绘制器</span>

#### <span class="c-func">DebugLinesRenderer(ppm=50.0)</span>

ortho 相机（像素/米缩放）；GPU 惰性装配——首次 draw 须已 set_mode。

##### <span class="c-func">set_camera(ppm)</span>

改 ortho 缩放（下次 draw 生效）——同时解除全局相机跟随。

- :type ppm: float

##### <span class="c-func">follow_global_camera(enable=True)</span>

切全局相机跟随——draw 前同步引擎全局 MVP。**[差异]** 2D 调试线世界
坐标为米，Auto2d 全局相机按像素正交，米坐标直入会过小——跟随时请配
Manual 相机。

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

### <span class="c-class">RigidBodyBuilder3D —— 刚体 builder</span>

#### <span class="c-func">RigidBodyBuilder3D(kind, translation=None, rotation=None, linvel=None, angvel=None, can_sleep=True)</span>

同 2D 形态——`rotation` 收**欧拉角向量** `(rx, ry, rz)` 弧度；
`linvel` / `angvel` 三分量。

### <span class="c-class">ColliderBuilder3D —— 碰撞体 builder</span>

#### <span class="c-func">ColliderBuilder3D(kind, hx=None, hy=None, hz=None, radius=None, translation=None, friction=None, restitution=None, density=None, sensor=False, events=False)</span>

cuboid 须给 hx/hy/hz 半尺寸（米）；**v1 仅 cuboid / ball**。

### <span class="c-class">DebugDraw3D —— 调试渲染控制面</span>

#### <span class="c-func">DebugDraw3D(collider_shapes=None, contacts=None, joints=None, aabbs=None, axes=None)</span>

内容位集构造期定格（缺省 = 形状 + 接触开）。

### <span class="c-class">LineSink3D —— 线段收集器</span>

#### <span class="c-func">LineSink3D()</span>

方法同 2D：`line_count()` / `is_empty()` / `clear()`。

### <span class="c-class">DebugLinesRenderer3D —— 调试线绘制器</span>

#### <span class="c-func">DebugLinesRenderer3D(eye=(0, 6, 14), target=(0, 0, 0), fov=1.0)</span>

look-at 透视相机（`fov` = 垂直视场角**弧度**，1.0 ≈ 57°）。

##### <span class="c-func">set_camera_lookat(eye, target, fov)</span>

改机位（下次 draw 生效；视口宽高比自动跟随）——同时解除全局跟随。

##### <span class="c-func">follow_global_camera(enable=True)</span>

切全局相机跟随（再调 `set_camera_lookat` 解除）。

#### <span class="c-func">t.draw_debug_3d(renderer, sink)</span>（会话方法）

3D 叠加绘制——**带深度通道**（Depth24Plus 硬契约）。Surface 变体
须以 `depth_buffer=True` 创建离屏面（无深度面报错并指路）。

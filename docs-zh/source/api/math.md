# pygame.math

数学工具——标量插值 / 钳制、`Vector2` / `Vector3` / `Quaternion` /
`Mat4`（glam 直通）。数值参数收 int 或 float；内部全线 f32。

```{note}
[差异] 向量乘法全对称——`vec * vec` 为**元素级**（glam 语义；真 pygame
乘法仅标量）。零向量归一化 = 零向量（pygame 语义）。四类均不可哈希。
```

## <span class="c-func">标量函数</span>

#### <span class="c-func">lerp(a, b, t) -> float</span>

线性插值（`t=0` → `a`，`t=1` → `b`）。

#### <span class="c-func">clamp(x, min, max) -> float</span>

钳制到 `[min, max]`。

#### <span class="c-func">inv_lerp(a, b, v) -> float</span>

逆线性插值（求 `v` 在 `[a, b]` 中的 `t`）。

#### <span class="c-func">remap(in_a, in_b, out_a, out_b, v) -> float</span>

区间重映射（先 `inv_lerp` 再 `lerp`）。

#### <span class="c-func">smoothstep(e0, e1, x) -> float</span>

平滑阶跃插值。

#### <span class="c-func">smootherstep(e0, e1, x) -> float</span>

更高阶平滑阶跃插值。

## <span class="c-func">3D 构造函数</span>

度数制入口（弧度制走 glam 语义层）。

#### <span class="c-func">quat_identity() -> Quaternion</span>

单位四元数。

#### <span class="c-func">quat_from_axis_angle(ax, ay, az, deg) -> Quaternion</span>

轴角 → 四元数（轴无需预归一；角**度数制**）。

#### <span class="c-func">quat_from_euler(pitch, yaw, roll) -> Quaternion</span>

欧拉角 → 四元数（度数制；XYZ 序）。

#### <span class="c-func">mat4_identity() -> Mat4</span>

4x4 单位矩阵。

#### <span class="c-func">mat4_from_quat(q) -> Mat4</span>

四元数 → 旋转矩阵。

#### <span class="c-func">mat4_translation(x, y, z) -> Mat4</span>

平移矩阵。

#### <span class="c-func">mat4_scale(x, y, z) -> Mat4</span>

缩放矩阵。

#### <span class="c-func">frustum_from_mvp(m) -> Frustum</span>

VP（或 MVP）→ 视锥（Gribb-Hartmann 六平面提取）。**CPU 侧剔除决策
的判定源**——CPU 烘批架构下"查一下再决定 push 不 push"零额外开销：

```python
cam = pygame.render.camera_perspective(45.0, w / h, 0.1, 100.0)
fr = pygame.math.frustum_from_mvp(cam.mvp())     # 每相机建一次（矩阵变则重建）
for obj in objects:
    if not fr.intersects_sphere(obj.x, obj.y, obj.z, obj.bound_r):
        continue                                  # 视锥外 → 不进 batch
    pygame.gfxdraw.set_transform(t, obj.model())
    pygame.gfxdraw.draw_surface(t, obj.surface)
```

**letterbox**：裸 VP（未乘 `present_fit()`）给出**包含**可见区域的保守
超集视锥——剔除安全方向（不误剔）；要精确画布子矩形传
`present_fit().mul_mat4(cam.mvp())`。

- :param m: VP / MVP 矩阵（`Camera.mvp()` 直入）
- :type m: Mat4
- :rtype: Frustum

## <span class="c-class">类</span>

### <span class="c-class">Vector2 —— 二维向量</span>

#### <span class="c-func">Vector2(x, y)</span>

构造二维向量。

##### <span class="c-attr">x / y</span>

分量（读写）。

##### <span class="c-func">dot(other) -> float</span>

点积。

##### <span class="c-func">length() -> float</span>

欧氏长度。

##### <span class="c-func">normalize() -> Vector2</span>

归一化（返回**新**向量；零向量归一化 = 零向量）。

##### <span class="c-magic">运算符</span>

`+` / `-` 仅向量；`*` 全对称（vec×vec 元素级 / vec×标量 / 标量×vec）；
`/` 元素级与标量除（标量除零抛 `ZeroDivisionError`）；`+=` / `-=` 就地
变更（对象恒等保持）；`-x` 取反；`==` 仅 Vector2 逐分量。不支持
`rotate` / `angle_to` 等（增强批次预留）。

### <span class="c-class">Vector3 —— 三维向量</span>

#### <span class="c-func">Vector3(x, y, z)</span>

构造三维向量。

##### <span class="c-attr">x / y / z</span>

分量（读写）。

##### <span class="c-func">dot(other) -> float / cross(other) -> Vector3</span>

点积 / 叉积（右手系）。

##### <span class="c-func">length() -> float / length_squared() -> float</span>

长度 / 长度平方。

##### <span class="c-func">normalize() -> Vector3</span>

归一化（返回新向量；零向量 = 零向量）。

##### <span class="c-func">distance_to(other) -> float</span>

到 other 的直线距离。

##### <span class="c-func">lerp(other, t) -> Vector3</span>

线性插值（`t=0` 本体 → `1` other）。

##### <span class="c-func">scale_to_length(target)</span>

**就地**缩放自身到指定长度（pygame 语义：改自身）。

##### <span class="c-magic">运算符</span>

同 `Vector2` 约定（`+`/`-` 仅向量、`*` 全对称、`/` 标量除零抛错、
`+=`/`-=` 就地、`-x`、`==` 仅同型逐分量）。

### <span class="c-class">Quaternion —— 四元数</span>

推荐经模块函数构造（度数制）；类直呼 = `xyzw` 四参。

#### <span class="c-func">Quaternion(x, y, z, w)</span>

四分量直构。

##### <span class="c-attr">x / y / z / w</span>

分量（**只读**）。

##### <span class="c-func">length() -> float</span>

四元数长度。

##### <span class="c-func">normalize() -> Quaternion</span>

归一化（返回新四元数；零长度**原样返回**——glam 归零会产生 NaN）。

##### <span class="c-func">dot(other) -> float / slerp(other, t) -> Quaternion</span>

点积 / 球面插值（`t=0` 本体 → `1` other）。

##### <span class="c-magic">运算符</span>

`*` 全对称（quat×quat 组合 / quat×标量 缩放）；`-x`；`==` 仅同型。

### <span class="c-class">Mat4 —— 4x4 矩阵</span>

**直呼不可实例化**——经 `mat4_*` 模块函数获得。

##### <span class="c-func">mul_mat4(other) -> Mat4</span>

矩阵乘 `self * other`（变换叠加顺序同 glam）。

##### <span class="c-func">transform_point3(v) -> Vector3</span>

变换三维点（透视除法前的位置变换）。

##### <span class="c-magic">运算符</span>

`*` 全对称（mat×mat 组合 / mat×标量）；`-x`。`==` 为身份比较（未实现
分量比较）。

### <span class="c-class">Frustum —— 视锥（剔除）</span>

六平面视锥——**保守语义**：只把"确定在外"判负，跨界/相切一律放行
（渲染剔除的安全方向：漏剔多画一帧，误剔物体凭空消失）。构造经
`frustum_from_mvp`（直呼不可实例化）；入参 int/float 兼收。

##### <span class="c-func">contains_point(x, y, z) -> bool</span>

点在视锥内（含边界）——最廉价的成员测试（细点云/粒子）。

##### <span class="c-func">intersects_sphere(x, y, z, r) -> bool</span>

包围球与视锥相交（跨界球放行）——**首选判定形**（物体预算包围球的
常规形态；r ≥ 0）。

##### <span class="c-func">intersects_aabb(minx, miny, minz, maxx, maxy, maxz) -> bool</span>

轴对齐包围盒与视锥相交（p-vertex 精确判外——任一平面的正顶点在外
则整盒在外，斜跨盒不误剔）。

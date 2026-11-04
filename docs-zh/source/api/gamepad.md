# pygame.gamepad

手柄输入——标准布局按钮 / 轴查询（每帧轮询态）。命名定约：替代
joystick + controller 两名，按钮语义对齐 SDL 标准布局；Web Gamepad
API 同序。

## <span class="c-const">常量</span>

按钮（标准布局索引 0..16）：

| 常量 | 语义 |
|---|---|
| BTN_SOUTH / BTN_EAST / BTN_WEST / BTN_NORTH | A（下侧）/ B（右侧）/ X（左侧）/ Y（上侧）|
| BTN_LB / BTN_RB | 左 / 右肩键 |
| BTN_LT / BTN_RT | 左 / 右扳机 |
| BTN_SELECT / BTN_START | 选择 / 开始 |
| BTN_LTHUMB / BTN_RTHUMB | 左 / 右摇杆按下 |
| BTN_DPAD_UP / DOWN / LEFT / RIGHT | 十字键四向 |

轴（摇杆 `-1.0..=1.0`；扳机 `0..=1.0`）：

| 常量 | 语义 |
|---|---|
| AXIS_LX / AXIS_LY | 左摇杆 X / Y |
| AXIS_RX / AXIS_RY | 右摇杆 X / Y |
| AXIS_LT / AXIS_RT | 左 / 右扳机行程（0..=1.0）|

## <span class="c-func">函数</span>

#### <span class="c-func">get_count() -> int</span>

已连接手柄数。

- :rtype: int

#### <span class="c-func">connected() -> list[int]</span>

已连接手柄编号列表（连接顺序；平台内自增）。

- :rtype: list[int]

#### <span class="c-func">primary() -> int | None</span>

第一个连接的手柄编号（单手柄便捷入口；无 = None）。

- :rtype: int | None

#### <span class="c-func">is_connected(id) -> bool</span>

指定手柄是否已连接。

- :rtype: bool

#### <span class="c-func">get_pressed(id, btn) -> bool</span>

按键按下态（未连接 = False；无效按钮索引抛 `ValueError`）。

- :rtype: bool

#### <span class="c-func">just_pressed(id, btn) -> bool</span>

本帧新按下（上帧未按下；未连接 = False）。

```python
if pygame.gamepad.just_pressed(pad, pygame.gamepad.BTN_SOUTH):
    jump()
```

- :rtype: bool

#### <span class="c-func">get_axis(id, axis) -> float</span>

轴值（摇杆 `-1.0..=1.0`；扳机 `0..=1.0`；未连接 = 0.0）。

- :rtype: float

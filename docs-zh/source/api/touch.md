# pygame.touch

触摸输入查询——多指快照与指位查询（移动端主用；桌面恒空）。

## <span class="c-func">函数</span>

#### <span class="c-func">get_count() -> int</span>

当前按住的手指数（0 = 无触摸；桌面恒 0）。

- :rtype: int

#### <span class="c-func">get_fingers() -> list[dict]</span>

活动手指快照，按 `id` 升序——`[{"id", "x", "y", "phase"}, ...]`。

**[差异]** `phase` 仅 `"started"` / `"moved"` 两态——抬起的手指即出表
（Ended / Cancelled 不驻留）。

- :rtype: list[dict]

#### <span class="c-func">get_pos(finger_id) -> tuple[int, int] | None</span>

指定手指的位置（**逻辑中心系**——原点屏幕中心、y 向上）；无该手指 = `None`。

- :param finger_id: 手指 id
- :type finger_id: int
- :rtype: tuple[int, int] | None

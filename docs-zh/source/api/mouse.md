# pygame.mouse

鼠标查询面——位置、按键态、相对位移、光标可见性。与 `pygame.event`
配对：事件推（MOUSEMOTION / MOUSEBUTTONDOWN），这里查（轮询即时态）。

## <span class="c-func">函数</span>

#### <span class="c-func">get_pos() -> tuple[int, int]</span>

光标位置（客户区物理像素，y 向下；事件泵前为 `(0, 0)`）。

- :rtype: tuple[int, int]

#### <span class="c-func">get_pressed() -> tuple[bool, bool, bool]</span>

左 / 中 / 右三键按下态（pygame 惯例序）。

```python
left, mid, right = pygame.mouse.get_pressed()
```

- :rtype: tuple[bool, bool, bool]

#### <span class="c-func">get_rel() -> tuple[int, int]</span>

相对位移（自上次 `get_rel()` 调用起；连续调用逐段累加）——FPS 相机
视角用。

- :rtype: tuple[int, int]

#### <span class="c-func">set_visible(visible)</span>

光标可见态切换（移动端无光标语义，收下无副作用）。

#### <span class="c-func">get_visible() -> bool</span>

光标可见态查询。

- :rtype: bool

#### <span class="c-func">get_focused() -> bool</span>

窗口鼠标聚焦态（与键盘聚焦同源）。

- :rtype: bool

#### <span class="c-func">set_pos(pos)</span>

warp 光标到指定位置。**[垫底]** 仅更新状态表——不扭曲真实光标、
无合成事件（starfish-window v1 无光标扭曲能力）。

- :param pos: 目标位置
- :type pos: tuple[int, int]

# pygame.event

事件队列与事件对象——主循环 **pull 模型** 的输入面：每帧
`pygame.event.get()` 排空队列，逐条处理，无回调、无隐藏线程边界。

```{note}
[差异] 事件类型 `type` 为**字符串标签**（如 `"keydown"`）；真 pygame
为整数常量。键值常量 `K_*` 全表在 [pygame.key](key.md) ——
`pygame.event.K_*` 不存在。
```

## 模块顶层成员

::::{dropdown} <span class="tag attr">attr</span> QUIT / KEYDOWN / KEYUP / MOUSEMOTION / MOUSEBUTTONDOWN / MOUSEBUTTONUP / TEXTINPUT
:open:
:animate: fade-in

**事件类型标签（字符串形态）**—— `Event.type` 字段的比对值。

| 常量 | 值 | 携带字段 |
|---|---|---|
| QUIT | "quit" | — |
| KEYDOWN / KEYUP | "keydown" / "keyup" | key mod repeat |
| MOUSEMOTION | "mousemotion" | x y |
| MOUSEBUTTONDOWN / UP | "mousebuttondown" / "mousebuttonup" | button x y |
| TEXTINPUT | "textinput" | text |

::::

::::{dropdown} <span class="bd bd-ok">函数</span><span class="bd bd-async">async-able</span> get() -> list[Event]
:animate: fade-in

排空事件队列，返回**全部**待处理事件（非阻塞）。

```python
for e in pygame.event.get():
    if e.type == pygame.event.QUIT:
        return
```

- :returns: 本帧全部事件
- :rtype: list[Event]

::::

::::{dropdown} <span class="bd bd-ok">函数</span> poll() -> Event | None
:animate: fade-in

弹出队首事件（其余留在队列——与 `get()` 的全量排空互补）。

- :returns: 队首事件，队列空时 None
- :rtype: Event | None

::::

::::{dropdown} <span class="bd bd-ok">函数</span> clear()
:animate: fade-in

清空事件队列。

::::

## 类

::::{dropdown} <span class="bd bd-ok">类</span> Event —— 事件对象
:open:
:animate: fade-in

`get()` / `poll()` 产物。**[差异]** 由引擎创建，Python 侧无
`__init__`（不可手工构造）；全保真载荷——`pygame_gui.event(e)`
等逐条消费者可重建完整原始输入。

&nbsp;

#### <span class="bd bd-magic">魔术方法</span> `__repr__() -> str`

`<Event keydown key=K_ESCAPE pos=(0, 0) button=0>` 形态的可读字符串。

#### <span class="bd bd-ok">属性</span> `type`

**事件类型标签**——字符串形态（`"keydown"` 等），比对上方常量表。

#### <span class="bd bd-ok">属性</span> `key`

键盘事件 = `pygame.key.Key` 实例，值语义相等可比对
（`e.key == pygame.key.K_x` ✓）；非键盘事件 = `int 0`。

#### <span class="bd bd-ok">属性</span> `x` / `y`

鼠标/触摸坐标（物理点已映射至 display 逻辑坐标，HiDPI 安全）；
非指针事件 = 0。

#### <span class="bd bd-ok">属性</span> `button`

鼠标按键序号；非鼠标事件 = 0。

#### <span class="bd bd-ok">属性</span> `mod`

修饰键位集（KMOD 位；仅 keydown/keyup 携带，其余 0）。

#### <span class="bd bd-ok">属性</span> `repeat`

自动重复标记（仅 keydown；其余 False）。

#### <span class="bd bd-ok">属性</span> `text`

文本输入内容（仅 textinput；其余 `""`）。

#### <span class="bd bd-ok">属性</span> `w` / `h`

窗口新尺寸（仅 resized；其余 0）。

::::

::::{dropdown} <span class="bd bd-ok">类</span> Key —— 类型化键值
:animate: fade-in

`e.key` 的载体；K_* 常量全表在 [pygame.key](key.md)。

&nbsp;

#### <span class="bd bd-magic">魔术方法</span> `__eq__() -> bool`

值语义相等比对：`e.key == pygame.key.K_x` ✓。

::::

## 使用示例

```python
for e in pygame.event.get():
    if e.type == pygame.event.QUIT:
        return
    if e.type == pygame.event.KEYDOWN and e.key == pygame.key.K_ESCAPE:
        return
    if e.type == pygame.event.MOUSEBUTTONDOWN:
        print("点击于", e.x, e.y, "按键", e.button)
    if e.type == pygame.event.TEXTINPUT:
        print("输入:", e.text)
```

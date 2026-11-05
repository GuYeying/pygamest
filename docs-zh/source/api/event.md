# pygame.event

事件队列与事件对象——主循环 **pull 模型** 的输入面：每帧
`pygame.event.get()` 排空队列，逐条处理，无回调、无隐藏线程边界。

```{note}
[差异] 事件类型 `type` 为**字符串标签**（如 `"keydown"`）；真 pygame
为整数常量。键值常量 `K_*` 全表在 [pygame.key](key.md) ——
`pygame.event.K_*` 不存在。
```

## <span class="c-const">常量</span>

事件类型标签（字符串形态）—— `Event.type` 字段的比对值。

| 常量 | 值 | 携带字段 |
|---|---|---|
| QUIT | "quit" | — |
| KEYDOWN / KEYUP | "keydown" / "keyup" | key mod repeat |
| MOUSEMOTION | "mousemotion" | x y |
| MOUSEBUTTONDOWN / UP | "mousebuttondown" / "mousebuttonup" | button x y |
| TEXTINPUT | "textinput" | text |

`[差异]` `mousewheel` / `resized` / `touch` / `focusgained` / `focuslost` /
`suspend` / `resume` / `user` / `other` 标签事件存在但常量未暴露——比对时
直接写字符串即可。

## <span class="c-func">函数</span>

#### <span class="c-func">get() -> list[Event]</span>

排空事件队列，返回**全部**待处理事件（非阻塞）。

```python
for e in pygame.event.get():
    if e.type == pygame.event.QUIT:
        return
```

- :returns: 本帧全部事件
- :rtype: list[Event]

#### <span class="c-func">poll() -> Event | None</span>

弹出队首事件（其余留在队列——与 `get()` 的全量排空互补）。

- :returns: 队首事件，队列空时 None
- :rtype: Event | None

#### <span class="c-func">clear()</span>

清空事件队列。

## <span class="c-class">类</span>

### <span class="c-class">Event —— 事件对象</span>

`get()` / `poll()` 产物。**[差异]** 由引擎创建，Python 侧无
`__init__`（不可手工构造）；全保真载荷——`pygame_gui.event(e)`
等逐条消费者可重建完整原始输入。

##### <span class="c-magic">\_\_repr\_\_() -> str</span>

`<Event keydown key=K_ESCAPE pos=(0, 0) button=0>` 形态的可读字符串。

##### <span class="c-attr">type</span>

事件类型标签——字符串形态（`"keydown"` 等），比对上方常量表。

##### <span class="c-attr">key</span>

键盘事件 = `pygame.key.Key` 实例，值语义相等可比对
（`e.key == pygame.key.K_x` ✓）；非键盘事件 = `int 0`。

##### <span class="c-attr">x / y</span>

鼠标/触摸坐标（**逻辑中心系**——原点屏幕中心、y 向上；物理点经
泵层统一换算，HiDPI / letterbox 安全）；非指针事件 = 0。

##### <span class="c-attr">button</span>

鼠标按键序号；非鼠标事件 = 0。

##### <span class="c-attr">mod</span>

修饰键位集（KMOD 位；仅 keydown/keyup 携带，其余 0）。

##### <span class="c-attr">repeat</span>

自动重复标记（仅 keydown；其余 False）。

##### <span class="c-attr">text</span>

文本输入内容（仅 textinput；其余 `""`）。

##### <span class="c-attr">w / h</span>

窗口新尺寸（仅 resized；其余 0）。

## <span class="c-func">自定义事件</span>

#### <span class="c-func">custom_code() -> int</span>

自定义事件码分配（原子递增，进程内唯一——`post_user` 的配对键）。

- :rtype: int

#### <span class="c-func">post_user(code)</span>

用户事件入队——下次 `get()` / `poll()` 可收：`e.type == "user"` 且
`e.key == code`。**保一帧定约**：post 后下一帧可见；再不消费随下帧
pump 丢弃（与引擎事件同寿命）。

- :param code: `custom_code()` 产物
- :type code: int

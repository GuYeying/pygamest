# pygame.event

事件队列与事件对象——主循环 **pull 模型**的输入面：每帧
`pygame.event.get()` 排空队列，逐条处理，无回调、无隐藏线程边界。

> **[差异]** 事件类型 `type` 为**字符串标签**（如 `"keydown"`）；真 pygame
> 为整数常量。键值常量 `K_*` 全表在 **pygame.key**——`pygame.event.K_*`
> 不存在。

## 常量

事件类型标签（`Event.type` 字段的比对值，字符串形态）：

| 常量 | 值 | 触发时机 |
|---|---|---|
| `QUIT` | `"quit"` | 关窗按钮 |
| `KEYDOWN` / `KEYUP` | `"keydown"` / `"keyup"` | 键盘按下/抬起（携带 `key` `mod` `repeat`）|
| `MOUSEMOTION` | `"mousemotion"` | 鼠标移动（`x` `y`）|
| `MOUSEBUTTONDOWN` / `MOUSEBUTTONUP` | `"mousebuttondown"` / `"mousebuttonup"` | 鼠标按键（`button` `x` `y`）|
| `TEXTINPUT` | `"textinput"` | 文本输入（`text`）|

`[差异]` `mousewheel` / `resized` / `touch` / `focusgained` / `focuslost` /
`suspend` / `resume` / `user` / `other` 标签事件存在但常量未暴露——比对时
直接写字符串即可。

## 函数

### <span class="bd bd-ok">函数</span> `get() -> list[Event]` { #get }

排空事件队列，返回**全部**待处理事件（非阻塞）。

```python
for e in pygame.event.get():
    if e.type == pygame.event.QUIT:
        return
```

### <span class="bd bd-ok">函数</span> `poll() -> Event | None` { #poll }

弹出队首事件（其余留在队列——与 `get()` 的全量排空互补）。

### <span class="bd bd-ok">函数</span> `clear()` { #clear }

清空事件队列。

## 类

### <span class="bd bd-ok">类</span> `Event` { #Event }

事件对象（`get()` / `poll()` 产物）。**[差异]** 由引擎创建，Python 侧无
`__init__`（不可手工构造）；全保真载荷——`pygame_gui.event(e)` 等逐条
消费者可重建完整原始输入。

#### <span class="bd bd-magic">魔术方法</span> `__repr__()` { #Event-repr }

`<Event keydown key=K_ESCAPE pos=(0, 0) button=0>` 形态的可读字符串。

#### <span class="bd bd-ok">属性</span> `type` { #Event-type }

事件类型标签（字符串，见上方常量表）。

#### <span class="bd bd-ok">属性</span> `key` { #Event-key }

键盘事件 = [`pygame.key.Key`](key.md) 实例（值相等可比对：
`e.key == pygame.key.K_x`）；非键盘事件 = `int 0`。

#### <span class="bd bd-ok">属性</span> `x` / `y` { #Event-pos }

鼠标/触摸坐标（物理点已映射至 display 逻辑坐标）；非指针事件 = 0。

#### <span class="bd bd-ok">属性</span> `button` { #Event-button }

鼠标按键序号；非鼠标事件 = 0。

#### <span class="bd bd-ok">属性</span> `mod` { #Event-mod }

修饰键位集（KMOD 位；仅 keydown/keyup 携带，其余 0）。

#### <span class="bd bd-ok">属性</span> `repeat` { #Event-repeat }

自动重复标记（keydown；其余 False）。

#### <span class="bd bd-ok">属性</span> `text` { #Event-text }

文本输入内容（仅 textinput；其余 `""`）。

#### <span class="bd bd-ok">属性</span> `w` / `h` { #Event-wh }

窗口新尺寸（仅 resized；其余 0）。

### <span class="bd bd-ok">类</span> `Key` { #Key }

类型化键值（`e.key` 的载体；K_* 常量全表在
[pygame.key](key.md)）。

#### <span class="bd bd-magic">魔术方法</span> `__eq__()` { #Key-eq }

值语义相等比对：`e.key == pygame.key.K_x` ✓。

!!! note "Pressed 视图"

    `pygame.key.get_pressed()` 返回的按下视图（`get(key) -> bool`）
    属于 [pygame.key](key.md) 模块——事件页不重复。

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

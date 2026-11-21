# pygame.event

事件模块——输入与系统事件的处理入口。游戏主循环每帧调用
`pygame.event.get()` 排空事件队列，逐条处理。

事件对象的 `type` 属性为**字符串标签**（如 `"keydown"`），各事件的
携带字段见下方事件类型表。键值常量 `K_*` 全表在
[pygame.key](key.md)。

```python
for event in pygame.event.get():
    if event.type == pygame.event.QUIT:
        quit_game()
    elif event.type == pygame.event.KEYDOWN and event.key == pygame.key.K_SPACE:
        jump()
```

## 事件类型

| 类型标签 | 触发时机 | 携带字段 |
|---|---|---|
| QUIT | 窗口被关闭 | — |
| KEYDOWN / KEYUP | 键盘按下 / 抬起 | key mod repeat |
| MOUSEMOTION | 鼠标移动 | x y |
| MOUSEBUTTONDOWN / MOUSEBUTTONUP | 鼠标按下 / 抬起 | button x y |
| TEXTINPUT | 文本输入 | text |
| MOUSEWHEEL | 滚轮滚动 | x y |
| RESIZED | 窗口尺寸变化 | w h |
| FOCUSGAINED / FOCUSLOST | 窗口获得 / 失去焦点 | — |
| FULLSCREENENTERED / FULLSCREENEXITED | 进入 / 退出全屏（Web 端含用户 ESC 退出） | — |
| TRAYMENU / TRAYCLICK / TRAYDOUBLECLICK | 托盘菜单项 / 图标点击 / 图标双击 | id button x y |

托盘系列事件需要构建时启用 `tray` feature（详见
[pygame.display](display.md) 桌宠能力位）。

## 函数

### get() -> list[Event]

取走队列中的**全部**待处理事件（非阻塞）。这是游戏主循环的标准
用法——每帧调用一次：

```python
for event in pygame.event.get():
    if event.type == pygame.event.QUIT:
        quit_game()
```

### poll() -> Event | None

从队列弹出一条事件（其余保留在队列中）。适合逐条处理：

```python
event = pygame.event.poll()
while event.type != pygame.event.QUIT:
    process(event)
    event = pygame.event.poll()
```

### clear()

清空事件队列。

## 自定义事件

### custom_code() -> int

分配一个自定义事件码（进程内唯一，原子递增）——与 `post_user()`
配对使用。

```python
MY_EVENT = pygame.event.custom_code()
```

### post_user(code)

向队列投递一个自定义事件。投递后**下一帧**即可从 `get()` /
`poll()` 收到（`event.type == "user"` 且 `event.key == code`）；
未被消费的事件随本帧事件泵结束丢弃。

```python
code = pygame.event.custom_code()
pygame.event.post_user(code)

# 下一次 get() 时收到：
for e in pygame.event.get():
    if e.type == "user" and e.key == code:
        handle_message()
```

## Event 对象

`get()` / `poll()` 返回的事件对象。由引擎创建（脚本侧不可构造），
携带完整原始输入载荷——`pygame_gui.event(e)` 等消费者可重建完整
输入。

属性一览：

| 属性 | 语义 | 事件范围 |
|---|---|---|
| type | 事件类型标签（字符串） | 全部 |
| key | 键值（`pygame.key.Key` 实例，值语义可比对） | keydown / keyup |
| x, y | 鼠标/触摸坐标（逻辑中心系，HiDPI 与 letterbox 已映射） | 指针与触摸事件 |
| button | 鼠标按键序号 / 托盘图标键 | 鼠标与托盘事件 |
| mod | 修饰键位集（KMOD 位） | keydown / keyup |
| repeat | 自动重复标记 | keydown |
| text | 文本输入内容 | textinput |
| w, h | 窗口新尺寸 | resized |
| id | 托盘事件标识 | 托盘事件 |

非对应事件的属性取零值（数值 `0`、文本 `""`）。

`repr()` 提供可读形态：
`<Event keydown key=K_ESCAPE pos=(0, 0) button=0>`。

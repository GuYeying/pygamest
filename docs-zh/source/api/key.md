# pygame.key

键盘模块——键值常量、修饰键、按下态查询、文本输入控制。

与 `pygame.event` 配对使用：键盘事件由事件系统推送（`KEYDOWN` /
`KEYUP`，事件的 `key` 属性携带键值），本模块负责查询即时按下状态。

键值是**类型化的 `Key` 对象**（非整数键码）：比较使用值相等——
`e.key == pygame.key.K_a`。

```python
for event in pygame.event.get():
    if event.type == pygame.event.KEYDOWN and event.key == pygame.key.K_SPACE:
        jump()
```

## 常量

### 键值 K_*（108 键全表）

`pygame.key.K_x` 形式的键值常量。SHIFT / CTRL / ALT / META 为左键
别名，右键另有 `K_R*` 全名。

| 分组 | 常量 |
|---|---|
| 字母 | K_a … K_z |
| 数字行 | K_0 … K_9 |
| 功能键 | K_F1 … K_F12 |
| 控制键 | K_ESCAPE K_TAB K_CAPSLOCK K_SPACE K_RETURN K_BACKSPACE K_DELETE K_INSERT |
| 修饰键 | K_SHIFT K_LSHIFT K_RSHIFT K_CTRL K_LCTRL K_RCTRL K_ALT K_LALT K_RALT K_META K_LMETA K_RMETA K_MENU |
| 方向/翻页 | K_UP K_DOWN K_LEFT K_RIGHT K_HOME K_END K_PAGEUP K_PAGEDOWN |
| 符号 | K_MINUS K_EQUALS K_LEFTBRACKET K_RIGHTBRACKET K_BACKSLASH K_SEMICOLON K_QUOTE K_BACKQUOTE K_COMMA K_PERIOD K_SLASH |
| 小键盘 | K_KP0 … K_KP9 K_KP_PLUS K_KP_MINUS K_KP_MULTIPLY K_KP_DIVIDE K_KP_ENTER K_KP_PERIOD |
| 其他 | K_PRINT K_SCROLLLOCK K_PAUSE K_NUMLOCK |

### 修饰键位 KMOD_*

`event.mod` 与 `get_mods()` 返回的位集。

| 常量 | 语义 |
|---|---|
| KMOD_SHIFT | Shift |
| KMOD_CTRL | Ctrl |
| KMOD_ALT | Alt |
| KMOD_META | Meta（Win / ⌘）|

## 函数

### get_pressed() -> Pressed

返回当前按下态视图。每帧调用一次，配合方向键类连续输入：

```python
keys = pygame.key.get_pressed()
if keys.get(pygame.key.K_LEFT):
    player.x -= 4
```

### get_mods() -> int

返回当前修饰键位集（KMOD_* 按位或）。

```python
if pygame.key.get_mods() & pygame.key.KMOD_CTRL:
    save()
```

### name(key) -> str

返回键的名称（W3C `KeyboardEvent.code` 命名）。

```python
pygame.key.name(pygame.key.K_a)     # "KeyA"
pygame.key.name(pygame.key.K_UP)    # "ArrowUp"
```

### get_focused() -> bool

返回窗口是否拥有键盘聚焦。失焦期间按下态查询不可靠——多窗口与全屏
切换场景可用此判断。

### start_text_input()

启动文本输入（移动端呼出软键盘）。桌面平台为空操作。

```python
pygame.key.start_text_input()
```

注意：使用 egui（`draw_ui`）绘制 UI 的脚本**不要调用**——键盘生命
周期由 egui 焦点状态自动驱动；显式口只用于非 egui 的自绘文本框。

### stop_text_input()

停止文本输入（移动端收起软键盘）。桌面平台为空操作。

### set_repeat(delay, interval)

键重复软件合成设置。当前实现收下参数但不生效——按键自动重复由操作
系统的重复标记承载（`KEYDOWN` 事件的 `repeat` 属性）。

### get_repeat() -> (delay, interval)

返回键重复配置。当前恒为 `(0, 0)`。

## 类

### Key

类型化键值对象——`event.key` 的载体、`K_*` 常量的类型。

注意：`Key` 对象**不可哈希**（不能作为 dict 或 set 的键）；比较一律
使用值相等：

```python
event.key == pygame.key.K_a    # 值相等比对
```

### Pressed

`get_pressed()` 返回的按下态视图。

#### get(key) -> bool

查询指定键当前是否按下：

```python
pressed = pygame.key.get_pressed()
if pressed.get(pygame.key.K_ESCAPE):
    quit_game()
```

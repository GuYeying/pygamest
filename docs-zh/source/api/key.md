# pygame.key

键盘输入的键值面：**K_* 键值常量全表**、修饰键位 `KMOD_*`、按下态查询
`get_pressed()`。与 `pygame.event` 配对——事件推（`Event.key` 携带
`Key`），这里查（按下态 / 修饰键位 / 键名）。

```{note}
[差异] 键值是**类型化 `Key` 对象**（真 pygame 为整数键码）：比较恒用
值相等——`e.key == pygame.key.K_a` ✓。`name()` 返回 W3C
`KeyboardEvent.code` 命名（`name(K_a) == "KeyA"`）。
```

## <span class="c-const">常量</span>

### <span class="c-const">键值 K_*（108 键全表）</span>

typed `Key` 常量——`pygame.key.K_a` 直接取用。SHIFT / CTRL / ALT / META
为左键别名，右键位另有 `K_R*` 全名。

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

### <span class="c-const">修饰键位 KMOD_*</span>

`Event.mod` / `get_mods()` 的位集。

| 常量 | 位语义 |
|---|---|
| KMOD_SHIFT | Shift |
| KMOD_CTRL | Ctrl |
| KMOD_ALT | Alt |
| KMOD_META | Meta（Win / ⌘）|

## <span class="c-func">函数</span>

#### <span class="c-func">get_pressed() -> Pressed</span>

取当前按下态视图（非阻塞；每帧取一次即可）。

```python
keys = pygame.key.get_pressed()
if keys.get(pygame.key.K_LEFT):
    player.x -= 4
```

- :returns: 按下态视图（见下方 `Pressed` 类）
- :rtype: Pressed

#### <span class="c-func">get_mods() -> int</span>

当前修饰键位集（KMOD 位按位或）。

- :returns: KMOD 位集
- :rtype: int

#### <span class="c-func">name(key) -> str</span>

键名——W3C `KeyboardEvent.code` 命名。

```python
pygame.key.name(pygame.key.K_a)     # "KeyA"
pygame.key.name(pygame.key.K_UP)    # "ArrowUp"
```

- :returns: 键名字符串
- :rtype: str

#### <span class="c-func">get_focused() -> bool</span>

窗口键盘聚焦态。失焦时按下态不可靠——全屏切换、多窗口场景用此判断。

- :rtype: bool

#### <span class="c-func">set_repeat(delay, interval)</span>

键重复软件合成。**[垫底]** 收下不生效——自动重复由 OS repeat 标记承载
（keydown 的 `Event.repeat`）。

#### <span class="c-func">get_repeat() -> tuple[int, int]</span>

键重复配置查询。**[垫底]** 恒返回 `(0, 0)` = 未启用软件重复。

- :rtype: tuple[int, int]

## <span class="c-class">类</span>

### <span class="c-class">Key —— 类型化键值</span>

`Event.key` 的载体、`K_*` 常量的类型。**[差异]** 不可哈希
（不能作 dict / set 键）——值比较走 `==`。

##### <span class="c-magic">\_\_eq\_\_() -> bool</span>

值语义相等比对：`e.key == pygame.key.K_x` ✓；与 `int` 比对恒 False
（真 pygame 的整数键码在这里不存在）。

### <span class="c-class">Pressed —— 按下态视图</span>

`get_pressed()` 的返回物——按键 → 按下态的只读查询视图。

##### <span class="c-func">get(key) -> bool</span>

查询某键当前是否按下。

```python
if pygame.key.get_pressed().get(pygame.key.K_ESCAPE):
    return
```

- :param key: `K_*` 常量（`Key` 实例）
- :returns: 该键是否按下
- :rtype: bool

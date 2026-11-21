# pygame.mouse

鼠标模块——查询光标位置与按键状态、控制光标可见性、锁定光标。

位置坐标为**逻辑中心系**：原点在屏幕中心、y 轴向上。与 `pygame.event`
配对使用：事件负责推送（MOUSEMOTION / MOUSEBUTTONDOWN），这里负责查询
即时状态。

## 函数

### get_pos() -> (x, y)

返回光标当前位置 `(x, y)`。事件泵启动前为 `(0, 0)`。

```python
x, y = pygame.mouse.get_pos()
```

### get_pressed() -> (left, middle, right)

返回左、中、右三键的按下状态。

```python
left, middle, right = pygame.mouse.get_pressed()
if left:
    shoot()
```

### get_rel() -> (dx, dy)

返回自上次调用本函数以来的光标位移。连续调用时逐段累加，
适合实现鼠标视角控制：

```python
dx, dy = pygame.mouse.get_rel()
yaw += dx * 0.3
pitch += dy * 0.3
```

### set_visible(visible)

设置光标是否可见。

```python
pygame.mouse.set_visible(False)   # 隐藏光标
```

### get_visible() -> bool

返回光标当前是否可见。

### set_grab(grabbed)

锁定光标到窗口客户区内——光标无法移出窗口，配合
`set_visible(False)` 实现 FPS 类游戏的沉浸式鼠标控制。

```python
pygame.mouse.set_grab(True)        # 锁定
pygame.mouse.set_visible(False)    # 隐藏光标
# ...游戏进行中...
pygame.mouse.set_grab(False)       # 释放
pygame.mouse.set_visible(True)
```

### get_grab() -> bool

返回光标当前是否被锁定。

### get_focused() -> bool

返回窗口是否拥有鼠标聚焦。

### set_pos(pos)

将光标状态表更新到指定位置。注意：仅更新内部状态表，不移动真实
光标、也不产生合成移动事件。

```python
pygame.mouse.set_pos((400, 300))
```

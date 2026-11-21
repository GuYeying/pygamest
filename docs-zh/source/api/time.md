# pygame.time

时间模块——毫秒计时器、延时、帧时钟（帧率上限与帧距度量）。

## 函数

### get_ticks() -> int

返回引擎启动以来的毫秒数。

```python
start = pygame.time.get_ticks()
```

### delay(ms)

阻塞延时 `ms` 毫秒。

注意：**Web 平台上为空操作**——浏览器由 requestAnimationFrame 控制帧
距，延时无法生效。

## 类

### Clock()

帧时钟对象——提供帧率上限节流与帧距度量。每个游戏循环配一个即可：

```python
clock = pygame.time.Clock()
while True:
    dt = clock.tick(60)   # 限制帧率 60，返回距上帧的毫秒数
    ...
```

#### tick(fps=0) -> float

推进一帧，并将帧率限制在 `fps` 上限（引擎补睡到目标帧距）。返回本帧
的真实间隔（毫秒）——可直接用作物理步长：

```python
dt = clock.tick(60) / 1000.0   # 换算为秒
world.step_frame(dt)
```

- fps —— 目标帧率上限；`0` 表示不限制
- 当垂直同步已把帧距拉到超过目标值时，引擎不会再额外补睡（避免双重节流）
- Web 平台上补睡为空操作，`tick()` 仅作帧距度量

#### get_time() -> float

返回本帧的真实间隔（毫秒）——与 `tick()` 的返回值相同，但不受节流影响。

#### get_fps() -> float

返回平滑处理后的帧率（指数移动平均）。

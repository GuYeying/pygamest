# pygame.time

帧时钟与计时——`get_ticks()` 毫秒计时、`Clock` 帧节流。

## <span class="c-func">函数</span>

#### <span class="c-func">get_ticks() -> int</span>

引擎启动以来的毫秒数。

- :rtype: int

#### <span class="c-func">delay(ms)</span>

阻塞延时。**[差异]** Web 上 no-op（无阻塞模型，rAF 节流承载帧距）。

## <span class="c-class">类</span>

### <span class="c-class">Clock —— 帧时钟</span>

上限节流语义：`tick(fps)` 推进一帧并补睡到目标帧距；vsync 已把帧距
拉到 ≥ 目标时不再补睡（三端同语义，无双重节流）。**[差异]** Web 上
睡眠 no-op，`tick()` 只作度量。

#### <span class="c-func">Clock()</span>

构造帧时钟。

```python
clock = pygame.time.Clock()
while True:
    dt = clock.tick(60)   # 毫秒
    ...
```

#### <span class="c-func">tick(fps=0) -> float</span>

推进一帧 + 上限节流，返回本帧真实间隔（**毫秒**，pygame 语义）。

- :param fps: 目标帧率上限（0 = 不节流）
- :returns: 本帧间隔毫秒
- :rtype: float

#### <span class="c-func">get_time() -> float</span>

本帧真实间隔（毫秒，同 `tick()` 返回值，不节流）。

- :rtype: float

#### <span class="c-func">get_fps() -> float</span>

EMA 平滑帧率。

- :rtype: float

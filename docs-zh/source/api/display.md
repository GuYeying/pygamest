# pygame.display

窗口与显示面——`set_mode()` 开窗（返回 `Screen` 绘制目标）、`flip()`
换帧收尾。窗口旗标创建期收下、运行时生效（`RESIZABLE` / `FULLSCREEN` /
`NOFRAME` 即时可切换）。

## <span class="c-const">常量</span>

窗口旗标位（`set_mode` 的 `flags` 参数；收 int 位掩码或 `[位, ...]`
序列两种形态）。

| 旗标 | 语义 |
|---|---|
| RESIZABLE | 可拖边缩放（默认已带边框）|
| FULLSCREEN | 全屏铺满显示器 |
| NOFRAME | 无边框 |
| HIDDEN | 创建期隐藏窗口 |
| ALWAYS_ON_TOP | 置顶（桌宠形态）|
| TRANSPARENT | 透明窗口（alpha 0 区域透出桌面）|
| VULKAN / DIRECTX / METAL / OPENGLES | 渲染后端声明（前向兼容，收下不解析）|

## <span class="c-func">函数</span>

#### <span class="c-func">set_mode(size, flags=0) -> Screen</span>

开窗并返回 Screen 绘制目标。

```python
screen = pygame.display.set_mode((800, 600))
screen = pygame.display.set_mode(
    (0, 0), [pygame.display.FULLSCREEN, pygame.display.RESIZABLE])
```

- :param size: 逻辑尺寸
- :type size: tuple[int, int]
- :param flags: 旗标位集
- :type flags: int | list[int]
- :returns: Screen 绘制目标
- :rtype: Screen

#### <span class="c-func">flip()</span>

帧收尾——把本帧绘制结果呈现到屏幕（主循环每帧末尾调用）。

#### <span class="c-func">set_opacity(v)</span>

窗口整体不透明度（`0.0` 透明 .. `1.0` 不透明）——桌宠/悬浮件形态。
与 `TRANSPARENT` 创建旗标正交：后者让 alpha 0 区域透出桌面，前者调
窗口整体 alpha。

#### <span class="c-func">set_caption(title)</span>

窗口标题。

#### <span class="c-func">get_init() -> bool</span>

display 初始化态（`set_mode` 是否已调用）。

- :rtype: bool

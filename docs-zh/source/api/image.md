# pygame.image

图片解码面——路径加载与字节加载**全平台同语义**（Web 由壳启动期预载表
承载字节，桌面直读落盘），支持 PNG / JPEG / BMP / GIF / WebP 按内容
解码。

```{note}
[差异] **缺资源不崩溃**：路径不存在 / 解码失败时返回 64px 品红棋盘格
占位图（错误打进日志），游戏照常跑；设环境变量 `STARFISH_STRICT_ASSETS=1`
恢复真异常，供 CI 测漏。
```

## <span class="c-func">函数</span>

#### <span class="c-func">load(path) -> Surface</span> <span class="tag async">async</span>

按路径加载图片（**await 形态**；全平台同语义——Web 由壳启动期预载表
承载字节）。

```python
async def main():
    sprite = await pygame.image.load("assets/player.png")
```

- :param path: 资源路径
- :type path: str
- :returns: Surface
- :rtype: Surface

#### <span class="c-func">load_from_bytes(bytes) -> Surface</span>

按内容解码（Web 首选入口——`io.fetch` 取回字节后接这里）。

- :param bytes: 图片原始字节
- :type bytes: bytes
- :rtype: Surface

#### <span class="c-func">save(surf, path)</span>

Surface 落盘（格式按扩展名，无扩展名默认 PNG）。**[差异]** 桌面专用
——Web 端调用抛 `NotImplementedError`（`save_async` 异步语义后续批次）。

- :param surf: 目标
- :type surf: Surface
- :param path: 输出路径
- :type path: str

#### <span class="c-func">get_extended() -> bool</span>

扩展图像格式支持态。

- :rtype: bool

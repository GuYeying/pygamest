# pygame.image

图片加载与保存模块——按内容自动识别 PNG / JPEG / BMP / GIF / WebP
格式，三平台语义一致（Web 端字节由壳启动期预载表承载，桌面直接读取
文件）。

```python
sprite = await pygame.image.load("assets/player.png")
```

注意：路径不存在或解码失败时，返回 64px 品红棋盘格**占位图**（错误
记录到日志），游戏继续运行。设置环境变量 `STARFISH_STRICT_ASSETS=1`
可改为直接抛出异常——适合 CI 中排查资源遗漏。

## 函数

### load(path) -> Surface

异步加载图片文件（三平台通用）。

```python
async def main():
    sprite = await pygame.image.load("assets/player.png")
```

### load_from_bytes(bytes) -> Surface

从图片字节解码。Web 端的推荐入口——先用 `pygame.io.fetch` 取回字节，
再交给本函数：

```python
data = await pygame.io.fetch("assets/player.png")
sprite = pygame.image.load_from_bytes(data)
```

### save(surface, path)

将 Surface 保存为图片文件。格式按扩展名判断，无扩展名默认 PNG。

注意：**仅桌面平台可用**——Web 端调用抛 `NotImplementedError`
（异步保存为后续批次）。

### get_extended() -> bool

返回扩展图像格式（PNG/JPEG 等）支持状态。

### placeholder_surface(size) -> Surface

返回占位面（`size`×`size` 正方形，引擎内建品红棋盘格图案）。加载
失败兜底、美术资源未到位时的合法 Surface——transform / draw 全链
可用。

### strict_assets() -> bool

返回严格资产模式的当前状态（环境变量 `STARFISH_STRICT_ASSETS=1`
时为 `True`——此时 load 链缺资源直接抛异常而非静默兜底，CI 测漏用）。

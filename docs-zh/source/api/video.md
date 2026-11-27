# pygame.video

视频解码与播放模块。提供从文件打开视频、逐帧解码、在画布上绘制画面的完整能力。

模块包含两个层次：

- `VideoSystem()` —— 解码系统入口，通过 `open()` 打开媒体文件（同步，桌面与移动平台）
- `pygame.video.open(path)` —— 异步打开（可 `await`，三平台通用；Web 平台唯一入口）

打开后得到 `Video` 对象：在游戏循环中每帧调用 `update()` 推进解码，
用 `frame()` 取当前帧绘制到画布上。

```python
import pygame

video = await pygame.video.open("cutscene.mp4")

while True:
    video.update(1.0 / 60.0)
    frame = video.frame()
    if frame is not None:
        with screen.render() as r:
            # 中心系像素域：dst_rect 铺满 800×600 画布（先声明正交相机）
            pygame.gfxdraw.draw_surface(
                r, frame, dst_rect=(-400, -300, 800, 600))
    await pygame.tasks.yield_now()
```

## 类

### VideoSystem()

解码系统入口。在 `pygame.display.set_mode()` 之后创建（进程内可复用，
依次打开多个视频）。

```python
vs = pygame.video.VideoSystem()
```

#### open(path) -> Video

同步打开媒体文件。桌面与移动平台直接可用；**Web 平台抛 `ValueError`**
（Web 端请使用异步口 `pygame.video.open()`）。

- path —— 媒体文件路径（相对路径基于 `set_mode` 时的工作目录；Web 端
  即页面基址下的 URL）
- 返回 `Video` 对象
- 文件缺失或格式不支持时抛 `ValueError`

```python
vs = pygame.video.VideoSystem()
video = vs.open("cutscene.mp4")
```

### open(path)（可等待）

异步打开媒体文件，三平台通用。打开过程（文件读取、解复用、解码器
初始化）在后台进行，不阻塞游戏循环——适合加载需要上百毫秒的视频。

```python
video = await pygame.video.open("cutscene.mp4")
```

- Web 平台基于 WebCodecs 硬解（浏览器负责解码）
- 文件缺失或格式不支持时抛 `ValueError`

### Video

已打开的媒体对象。在游戏循环中每帧调用 `update(dt)` 推进解码，
`frame()` 取当前帧绘制。

#### update(dt)

推进解码与播放。每帧调用一次，`dt` 传距上次调用的秒数
（通常 `1.0 / 60.0`）。解码错误抛 `ValueError`。

```python
video.update(1.0 / 60.0)
```

#### frame() -> Surface | None

返回当前帧画面，可直接 `pygame.gfxdraw.draw_surface()` 绘制。
首个视频帧尚未解码出时返回 `None`（此时画面仍应正常绘制其余内容）。

```python
frame = video.frame()
if frame is not None:
    pygame.gfxdraw.draw_surface(r, frame, dst_rect=(-400, -300, 800, 600))
```

#### size() -> (width, height)

视频原始尺寸（像素）。首个视频帧解码出之前返回 `(0, 0)`。

#### position() -> float

当前播放位置（秒）。

#### ended() -> bool

是否已播放到末尾。为 `True` 时 `frame()` 停在最后一帧。

#### has_audio() -> bool

媒体是否包含音轨。

#### set_video_enabled(enabled)

视频轨开关。设为 `False` 后只播音频不播画面（纯音乐模式）。

```python
video.set_video_enabled(False)
```

#### video_enabled() -> bool

视频轨当前开关态。

#### set_audio_volume(volume)

音量，`0.0`（静音）到 `1.0`（原始音量）。

#### set_muted(muted)

静音开关。`True` = 静音（不影响音量设定）。

## 平台差异

| 平台 | 解码方式 | 同步口 `VideoSystem.open` | 异步口 `pygame.video.open` |
|---|---|---|---|
| Windows / macOS / Linux / Android | 系统硬件解码（Media Foundation / VideoToolbox / GStreamer / MediaCodec） | 可用 | 可用 |
| Web | WebCodecs 硬解（浏览器决定硬解或软解） | 抛 `ValueError` | 可用 |

Web 端请注意：

- 视频文件通过 URL 加载，需放在页面可访问的路径下
- 浏览器不支持目标编码时抛 `ValueError`（如缺少 H.264 解码授权的浏览器）
- 音轨自动播放，`set_muted()` 可静音

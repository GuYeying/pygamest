# pygame.video

视频解码与播放——`VideoSystem` 打开媒体、`Video` 逐帧驱动（解码错误
抛 `ValueError`；缺失 / 不支持格式 `open` 即抛）。

## <span class="c-class">类</span>

### <span class="c-class">VideoSystem —— 解码系统入口</span>

`set_mode` 后创建。

#### <span class="c-func">VideoSystem()</span>

构造解码系统。

```python
vs = pygame.video.VideoSystem()
video = vs.open("cutscene.mp4")
```

##### <span class="c-func">open(path) -> Video</span>

打开媒体文件（缺失 / 不支持格式抛 `ValueError`）。

- :rtype: Video

### <span class="c-class">Video —— 已打开的媒体</span>

`update(dt)` 驱动解码推进（游戏循环里每帧喂真实间隔秒数）。

#### <span class="c-func">update(dt)</span>

推进解码。

- :param dt: 距上次调用的秒数
- :type dt: float

#### <span class="c-func">size() -> tuple[int, int]</span>

视频原始尺寸。

#### <span class="c-func">position() -> float</span>

当前播放位置（秒）。

- :rtype: float

#### <span class="c-func">ended() -> bool</span>

是否播放到尾。

- :rtype: bool

#### <span class="c-func">has_audio() -> bool</span>

是否含音轨。

- :rtype: bool

#### <span class="c-func">set_video_enabled(enabled)</span>

视频轨开关（纯音轨模式）。

#### <span class="c-func">video_enabled() -> bool</span>

视频轨开关态。

- :rtype: bool

#### <span class="c-func">set_audio_volume(v)</span>

音量（`0.0` .. `1.0`）。

#### <span class="c-func">set_muted(m)</span>

静音开关。

## 模块级异步口（2026-11-10 定约）

```python
v = await pygame.video.open("assets/video/cutscene.mp4")   # 打开不卡帧
```

- **机制**：`_open_start/_poll/_open_take` 原生三口 + 帧轮询 shim——
  open（含 MediaCodec configure 等百毫秒级初始化）在后台线程，引擎
  线程逐帧渲染不冻；`VideoSystem` 在引擎线程构造（GPU 槽线程局部），
  open 本体只携带 device/queue 的 Arc 克隆过线程
- `VideoSystem.open(path)` 同步口保留（底层/兼容——桌面毫秒级场景）
- **[差异]** Web 端 v1 站桩 ValueError（WebCodecs 播放管线后续批次）

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

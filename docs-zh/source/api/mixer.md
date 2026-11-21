# pygame.mixer

音效与背景音乐模块。包含 `Sound` 短音效（带通道模型）、`Channel` 定点
通道、`Effect` 实时特效，以及 `mixer.music` 流式 BGM 子模块。

```python
sfx = await pygame.mixer.load("assets/jump.wav")
sfx.play()

pygame.mixer.music.load("assets/bgm.mp3")
pygame.mixer.music.play(-1)   # 循环播放
```

## 函数

### load(path) -> Sound

异步加载音效文件（三平台通用）。

```python
sfx = await pygame.mixer.load("assets/jump.wav")
```

### load_from_bytes(bytes) -> Sound

从音频字节构造音效。

### init(channels=8)

显式初始化音频设备。不调用也可以——首次播放时会自动创建。

### quit()

卸载音频设备。

### get_init() -> bool

返回音频设备是否已初始化。

### set_master_volume(volume)

设置全部声道的主音量，`0.0` 到 `1.0`（越界自动收敛）。

### get_busy() -> bool

返回是否有任意音效或音乐正在播放。

### set_reserved(n)

保留 `0` 到 `n` 号通道——这些通道不参与 `Sound.play()` 的自动分配，
专供 `Channel(idx)` 定点使用。

### num_channels() -> int

返回当前通道总数（init 时确定）。

### output_sample_rate() -> int

返回混音输出的采样率（Hz）。

### beep() -> Sound

返回内嵌提示音（用于快速反馈的权宜音效）。

## Sound 类

短音效对象。

#### Sound(source=None)

构造音效：传字节按音频数据解码；不传参数 = 内嵌提示音。路径加载请
使用异步口 `pygame.mixer.load(path)`。

#### play(loops=0)

播放音效。`loops` = `-1` 无限循环、`0` 单次、`N` 重复 N 次。播放会
记录使用的通道，供后续 `stop()` / `set_volume()` 定位。

注意：通道索引在停止后会被回收复用——长期持有通道序号需注意此模型
限制。

#### stop()

停止该音效最近使用的播放通道（无播放记录时不动作）。

#### set_volume(volume)

设置该音效最近使用通道的音量，`0.0` 到 `1.0`。

#### get_length() -> float

返回音效时长（秒）。

## Channel 类

定点通道——fade 与特效挂载在此。

#### Channel(idx)

构造定点通道。`idx` 为通道序号（`0` 到 `num_channels()-1`），配合
`set_reserved()` 使用以避免被自动分配占用。

#### play(sound, loops=0)

在本通道定点播放。

#### stop() / pause() / resume()

停止 / 暂停 / 恢复本通道。

#### set_volume(volume)

设置本通道音量（`0.0` 到 `1.0`；无自动收敛——越界行为未定义）。

#### fade_in(sound, ms, loops=0)

淡入播放。注意：当前实现 `loops` 不生效（实际播放一次）。

#### fade_out(ms)

淡出停止本通道（`ms` 为淡出时长，毫秒）。

#### get_busy() -> bool

返回本通道是否正在播放。

#### get_sound() -> Sound | None

返回本通道当前装载的音效。

#### add_effect(effect)

将特效挂载到本通道的数据通路。挂载后通过 `effect.set(key, value)`
实时调参仍然生效。

## Effect 类

音频实时特效。

#### Effect(kind)

构造特效。`kind` 取值：`"gain"`（增益）/ `"echo"`（回声）/
`"distortion"`（失真）/ `"lowpass"`（低通滤波）。

#### set(key, value)

实时调整特效参数。可用键随特效类型：

| 特效 | 参数键 |
|---|---|
| gain | `"gain"` |
| echo | `"delay_ms"`、`"feedback"` |
| distortion | `"drive"` |
| lowpass | `"cutoff"` |

## mixer.music 子模块

流式播放的背景音乐——与短音效通道独立。

#### load(path)

加载背景音乐文件。文件缺失时不抛异常——静音继续播放其余内容
（游戏性不受阻，日志记录原因）；设置环境变量
`STARFISH_STRICT_ASSETS=1` 可改为抛出异常。

#### load_bytes(bytes)

从字节流加载背景音乐（内嵌 BGM 零落盘）。

#### play(loops=0)

播放背景音乐。`loops` = `-1` 无限循环、`0` 单次、`N` 重复 N 次。

#### stop() / pause() / resume() / rewind()

停止 / 暂停 / 恢复 / 回到曲首（`rewind` 等价于 `set_pos(0)`）。

#### set_volume(volume) / get_volume() -> float

背景音乐音量（`0.0` 到 `1.0`；`get_volume()` 返回当前值）。

#### get_length() -> float

返回音乐总时长（秒；未加载时为 `0.0`）——进度条等 UI 的分母。

#### set_pos(seconds) / get_pos() -> float

进度跳转与当前进度查询（单位秒，为底层设备的**真实播放位置**——
注意与 pygame 的"已播放时长"语义不同）。未加载时 `set_pos` 静默。

#### fadeout(ms)

淡出停止（`ms` 为淡出时长，毫秒）。

#### queue(path)

排队下一首——当前曲目播放完毕自动接播。文件缺失的处理与 `load`
一致：静默跳过（严格模式下抛异常）。

#### get_busy() -> bool

返回背景音乐是否正在播放（不含音效——与 `mixer.get_busy()` 分工）。

# pygame.mixer

音效与 BGM——`Sound` 短音效（通道模型）、`Channel` 定点通道、`Effect`
实时特效、`mixer.music` 流式 BGM 子模块。

```{note}
[差异] **Sound 路径构造退役**——路径加载统一 `await pygame.mixer.load(path)`，
字节走 `load_from_bytes(bytes)`；`Sound(bytes)` / `Sound()` 仍可用
（不传 = 内嵌提示音）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">load(path) -> Sound</span> <span class="tag async">async</span>

路径加载音效（统一入口；三线规范同 `image.load`）。

```python
sfx = await pygame.mixer.load("assets/jump.wav")
```

- :rtype: Sound

#### <span class="c-func">load_from_bytes(bytes) -> Sound</span>

字节构造（同步；三线同规范）。

- :rtype: Sound

#### <span class="c-func">init(channels=8)</span>

显式初始化音频设备（不调也行——首次播放自动建）。

#### <span class="c-func">quit()</span>

卸载音频设备。

#### <span class="c-func">get_init() -> bool</span>

初始化态。**[差异]** bool 形态（真 pygame 返回参数三元组 / None）。

- :rtype: bool

#### <span class="c-func">set_master_volume(v)</span>

主音量（`0.0` .. `1.0`，越界 clamp）。

#### <span class="c-func">get_busy() -> bool</span>

是否播放中（任意 SFX / 音乐）。

- :rtype: bool

#### <span class="c-func">set_reserved(n)</span>

保留 `0..n` 通道（不参与 `Sound.play` 自动分配——专供 `Channel(idx)`
定点使用）。

#### <span class="c-func">num_channels() -> int</span>

通道数（init 定格）。

- :rtype: int

#### <span class="c-func">output_sample_rate() -> int</span>

混音输出采样率（Hz）。

- :rtype: int

#### <span class="c-func">beep() -> Sound</span>

内嵌提示音（快速反馈用权宜音）。

- :rtype: Sound

## <span class="c-class">类</span>

### <span class="c-class">Sound —— 音效</span>

##### <span class="c-func">Sound(source=None)</span>

`Sound(bytes)` 按字节构造；`Sound()` = 内嵌提示音；**路径形态已退役**
（走 `await mixer.load(path)`）。

##### <span class="c-func">play(loops=0)</span>

播放（`-1` 无限 / `0` 单次 / `N` 次）。记录最近通道供 `stop()` /
`set_volume()` 定位——**[差异]** 通道索引停止后会被回收复用，长持
句柄需注意（starfish 通道模型固有限制）。

##### <span class="c-func">stop()</span>

停止最近播放的通道（无记录则不动）。

##### <span class="c-func">set_volume(v)</span>

音量（`0.0` .. `1.0`，作用于最近通道）。

##### <span class="c-func">get_length() -> float</span>

时长（秒）。

- :rtype: float

### <span class="c-class">Channel —— 定点通道</span>

fade / 特效挂载在此。

##### <span class="c-func">Channel(idx)</span>

构造定点通道（`idx` 不透明 int，`0..num_channels`；配合
`set_reserved`）。

##### <span class="c-func">play(sound, loops=0)</span>

本通道定点播放。

##### <span class="c-func">stop() / pause() / resume()</span>

停 / 暂停 / 恢复本通道。

##### <span class="c-func">set_volume(v)</span>

本通道音量（`0.0` .. `1.0`；**无 clamp**——越界行为未定义）。

##### <span class="c-func">fade_in(sound, ms, loops=0)</span>

淡入播放。**[差异]** `loops` 收下未转发底层（实际单次播放）。

##### <span class="c-func">fade_out(ms)</span>

淡出。

##### <span class="c-func">get_busy() -> bool</span>

本通道播放中。

- :rtype: bool

##### <span class="c-func">get_sound() -> Sound | None</span>

本通道当前装载的声音。

- :rtype: Sound | None

##### <span class="c-func">add_effect(effect)</span>

挂特效到本通道数据通路（Effect 句柄克隆共享参数态——挂载后
`effect.set(key, v)` 仍实时生效）。

### <span class="c-class">Effect —— 音频特效</span>

##### <span class="c-func">Effect(kind)</span>

构造特效：`"gain"` / `"echo"` / `"distortion"` / `"lowpass"`。

##### <span class="c-func">set(key, value)</span>

实时调参（键随预设）：`gain` → `"gain"`；`echo` →
`"delay_ms"` / `"feedback"`；`distortion` → `"drive"`；`lowpass` →
`"cutoff"`。

## <span class="c-func">mixer.music —— 流式 BGM 子模块</span>

#### <span class="c-func">load(path)</span>

加载 BGM。**[差异]** 缺资源不崩溃 = 静音继续（游戏性不受阻，日志记
因）；`STARFISH_STRICT_ASSETS=1` 恢复真异常。

- :param path: 音乐文件路径
- :type path: str

#### <span class="c-func">load_bytes(bytes)</span>

字节流式加载（内嵌 BGM 零落盘）。

#### <span class="c-func">play(loops=0)</span>

播放（`-1` 无限循环 / `0` 单次 / `N` 次）。

#### <span class="c-func">stop() / pause() / resume()</span>

停 / 暂停 / 恢复。

#### <span class="c-func">set_volume(v)</span>

音乐音量（`0.0` .. `1.0`）。

#### <span class="c-func">get_busy() -> bool</span>

音乐播放中（不含 SFX——与 `mixer.get_busy()` 分工）。

- :rtype: bool

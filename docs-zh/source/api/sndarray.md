# pygame.sndarray

声音 ↔ 样本数组互转——程序化合成音效用。

```{note}
[差异] 无 numpy 环境——数组用**嵌套 list / tuple** 承载（真 pygame 是
numpy 数组）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">array(sound) -> list</span>

Sound → 样本数组：立体声 `[(L, R), ...]` / 单声道 `[(v,), ...]`。

- :param sound: 音效对象
- :type sound: Sound
- :rtype: list

#### <span class="c-func">make_sound(array) -> Sound</span>

样本数组 → Sound。行宽 `1` = 单声道 / `2` = 立体声（混排报
"行宽不一致"；行宽其他值直接报错）。

**[差异]** 采样率 v1 固定 44100（真 pygame 从 array 推断采样率）。

```python
beep = pygame.sndarray.make_sound(
    [(0.0, 0.0)] * 100 + [(0.5, 0.5)] * 100)
```

- :param array: 嵌套 list（元素 float，-1.0 .. 1.0）
- :type array: list[list[float]]
- :returns: Sound
- :rtype: Sound

#### <span class="c-func">frame_count(sound) -> int</span>

音频帧数（一帧 = 一次采样的全部声道——单声道 = 样本数；立体声 =
每声道样本数）。

- :rtype: int

#### <span class="c-func">mono(samples, rate) -> Sound</span>

原始样本直接构造单声道（程序化音频——正弦/噪声发生器的落点）。

- :param samples: f32 列表（-1.0 .. 1.0）
- :param rate: 采样率（须 > 0）
- :rtype: Sound

#### <span class="c-func">stereo(samples, rate) -> Sound</span>

原始样本直接构造立体声（`samples` **交错**排列 L,R,L,R……帧数 =
样本数 / 2；奇数长度 ValueError）。

- :rtype: Sound

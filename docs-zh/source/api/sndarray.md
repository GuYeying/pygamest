# pygame.sndarray

声音 ↔ 样本数组互转——程序化合成音效用。

```{note}
[差异] 无 numpy 环境——数组用**嵌套 list / tuple** 承载（真 pygame 是
numpy 数组）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">array(sound) -> list</span>

Sound → 样本数组：立体声 `[(L, R), ...]` / 单声道 `[(v,), ...]`。

- :param sound: `pygame.mixer.Sound`
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
- :returns: Sound
- :rtype: Sound

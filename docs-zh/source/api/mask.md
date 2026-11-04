# pygame.mask

位面遮罩——像素级碰撞与轮廓（`from_surface` 一步生成）。

```{note}
[差异] GPU 出生面的回读走慢路径——Web 端建议资产装载期构建、缓存
复用（`from_surface_async` 语义后续批次，本同步面对其返回错误）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">from_surface(surface, threshold=127) -> Mask</span>

离屏面 → 位面（`alpha > threshold` 置位）。

- :param surface: 源
- :type surface: Surface
- :param threshold: alpha 阈值（默认 127）
- :type threshold: int
- :rtype: Mask

## <span class="c-class">类</span>

### <span class="c-class">Mask —— 位面</span>

#### <span class="c-func">Mask(size, fill=False)</span>

构造位面。

- :param size: 位面尺寸
- :type size: tuple[int, int]
- :param fill: 初始全置位（默认全空）
- :type fill: bool

##### <span class="c-func">get_at(pos) -> bool</span>

位读取。

- :rtype: bool

##### <span class="c-func">set_at(pos, value=True)</span>

位写入。

##### <span class="c-func">count() -> int</span>

置位数。

- :rtype: int

##### <span class="c-func">get_size() -> tuple[int, int]</span>

位面尺寸。

- :rtype: tuple[int, int]

##### <span class="c-func">fill(value=True)</span>

全位置位 / 清零。

##### <span class="c-func">invert()</span>

全位取反。

##### <span class="c-func">overlap(other, offset) -> tuple[int, int] | None</span>

首个重叠点（None = 无重叠；`offset` = other 相对本的偏移）——像素级
碰撞检测主口。

```python
hit = mask_a.overlap(mask_b, (b_x - a_x, b_y - a_y))
```

- :rtype: tuple[int, int] | None

##### <span class="c-func">overlap_area(other, offset) -> int</span>

重叠位数量。

- :rtype: int

##### <span class="c-func">overlap_mask(other, offset) -> Mask</span>

重叠位图（新 Mask）。

- :rtype: Mask

##### <span class="c-func">scale(size) -> Mask</span>

最近邻缩放（新位面）。

- :rtype: Mask

##### <span class="c-func">to_surface() -> Surface</span>

位面可视化（置位 = 白）。

- :rtype: Surface

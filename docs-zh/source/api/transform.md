# pygame.transform

Surface 几何变换——翻转、缩放、旋转、缩放旋转复合。全部返回**新
Surface**（源不修改）。

```{note}
[差异] `scale()` 与 `smoothscale()` 等同——采样器恒线性平滑（真 pygame
两名分 nearest / linear；像素风 nearest 变体 v2 预留）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">flip_x(s) -> Surface</span>

水平镜像。

#### <span class="c-func">flip_y(s) -> Surface</span>

垂直镜像。

#### <span class="c-func">scale(s, size) -> Surface</span>

线性采样缩放到 `(w, h)`（恒平滑）。

#### <span class="c-func">smoothscale(s, size) -> Surface</span>

平滑缩放——与 `scale()` 等同（见上方差异说明），为 pygame 心智保留
两名。

#### <span class="c-func">rotate(s, deg) -> Surface</span>

逆时针旋转 `deg` 度——输出为包围盒，内容居中。

#### <span class="c-func">rotozoom(s, deg, zoom) -> Surface</span>

旋转 + 缩放复合（`deg` 逆时针度数，`zoom` 倍率）。

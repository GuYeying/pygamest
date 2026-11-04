# pygame.gfxdraw

SDL_gfx 风格图元——像素 / 线 / 圆 / 椭圆 / 弧 / 三角 / 多边形 / 矩形，
全部无返回值、整数坐标（原点左上，y 向下）。

```{note}
[差异] 首参 `f` 是**渲染会话**（`with screen.render() as f` 的 `f`，
或 Surface 会话）——非真 pygame 的 Surface。尾参颜色统一为**单个颜色
值** `(r, g, b)` / `(r, g, b, a)` / `pygame.Color`（非真 pygame 的
三个独立 int）。
```

```{note}
[差异] `aa*` 系全部是**兼容别名**——与对应非 aa 版本同一实现（AA 由
wgpu 线性采样承载，无每像素抗锯齿路径）。`rounded_*` **垫底**：圆角
v1 未建，`radius` 收下忽略，降级直角。
```

## <span class="c-func">点 / 线</span>

#### <span class="c-func">pixel(f, x, y, color)</span>

画单个像素。

#### <span class="c-func">hline(f, x1, x2, y, color)</span>

水平线。

#### <span class="c-func">vline(f, x, y1, y2, color)</span>

垂直线。

#### <span class="c-func">line(f, x1, y1, x2, y2, color)</span>

线段（宽度 1）。

#### <span class="c-func">aaline(f, x1, y1, x2, y2, color)</span>

抗锯齿线 = `line` 兼容别名。

## <span class="c-func">圆 / 椭圆</span>

#### <span class="c-func">circle(f, x, y, radius, color)</span>

1px 描边圆（圆心 + 半径；负半径钳 0）。

#### <span class="c-func">aacircle(f, x, y, radius, color)</span>

= `circle` 兼容别名。

#### <span class="c-func">filled_circle(f, x, y, radius, color)</span>

填充圆。

#### <span class="c-func">ellipse(f, x, y, rx, ry, color)</span>

1px 描边椭圆（中心 + 半径轴，非外接矩形）。

#### <span class="c-func">aaellipse(f, x, y, rx, ry, color)</span>

= `ellipse` 兼容别名。

#### <span class="c-func">filled_ellipse(f, x, y, rx, ry, color)</span>

填充椭圆。

## <span class="c-func">弧</span>

#### <span class="c-func">arc(f, x, y, radius, start_angle, stop_angle, color)</span>

1px 描边圆弧。**[差异]** 角度**度数制**（SDL_gfx 惯例）；v1 简化——
仅圆弧，无椭圆弧 / 填充弧。

## <span class="c-func">三角 / 多边形</span>

#### <span class="c-func">trigon(f, x1, y1, x2, y2, x3, y3, color)</span>

描边三角（实现 = 填充 + 1px 轮廓两步组合）。

#### <span class="c-func">aatrigon(f, x1, y1, x2, y2, x3, y3, color)</span>

= `trigon` 兼容别名。

#### <span class="c-func">filled_trigon(f, x1, y1, x2, y2, x3, y3, color)</span>

填充三角。

#### <span class="c-func">polygon(f, points, color)</span>

1px 描边多边形——`points = [(x, y), ...]`。

#### <span class="c-func">aapolygon(f, points, color)</span>

= `polygon` 兼容别名。

#### <span class="c-func">filled_polygon(f, points, color)</span>

填充多边形。

## <span class="c-func">矩形</span>

`rect` 形态统一 `(left, top, w, h)` 序列。

#### <span class="c-func">rectangle(f, rect, color)</span>

1px 描边矩形。

#### <span class="c-func">box(f, rect, color)</span>

填充矩形。

#### <span class="c-func">rounded_rectangle(f, rect, radius, color)</span>

**[垫底]** 圆角 v1 未建——`radius` 忽略，降级描边直角矩形。

#### <span class="c-func">rounded_box(f, rect, radius, color)</span>

**[垫底]** 同上，降级填充矩形。

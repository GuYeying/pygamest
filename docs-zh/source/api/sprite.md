# pygame.sprite

精灵与组——`Sprite` 链式配置对象 + `Group` 池化组。配置即挂（add 前
纯配置，add 后成为组句柄）；`(z, y)` 排序会话绘制。

```{note}
[差异] `with_tag(tag)` v1 返回**数量**（int）而非句柄列表——id 级过滤
无法反查 Python 对象，句柄反查随 v2 资源句柄表立项。
```

## <span class="c-class">类</span>

### <span class="c-class">Sprite —— 精灵配置对象</span>

##### <span class="c-func">Sprite(img)</span>

构造精灵。

```python
player = (
    pygame.sprite.Sprite(await pygame.image.load("player.png"))
    .at(100, 200)
    .tag("player")
    .z(10)
)
group.add(player)
```

- :param img: 帧序列（帧动画 fps 由 `set_fps` 定，缺省 4；空列表报错）
- :type img: Surface | list[Surface]

##### <span class="c-func">at(x, y) -> self</span>

初始位置（链式）。

##### <span class="c-func">tag(name) -> self</span>

分组标签（链式；`with_tag` 查询用）。

##### <span class="c-func">z(v) -> self</span>

层级（`(z, y)` 排序会话绘制）。

##### <span class="c-func">rotation(deg) -> self</span>

旋转角（度，链式）。

##### <span class="c-func">scale(f) -> self</span>

等比缩放（链式）。

##### <span class="c-func">alpha(a) -> self</span>

表面级半透明 `0.0` .. `1.0`（越界 clamp，链式）。

##### <span class="c-func">visible(b) -> self</span>

可见性（链式）。

##### <span class="c-func">set_fps(f)</span>

多帧动画帧率（缺省 4，下限 0.1）。

##### <span class="c-func">alive() -> bool</span>

是否已在组内存活（未 add = False）。

- :rtype: bool

### <span class="c-class">Group —— 池化组</span>

世代 id 语义——`remove` 后旧句柄自动失效（后续操作安全报错而非悬垂）。

##### <span class="c-func">Group()</span>

新建池化组。

##### <span class="c-func">add(sprite) -> Sprite</span>

物化配置入组并回写世代 id（返回原对象以便链式 / 即取即用）。

##### <span class="c-func">remove(sprite) -> bool</span>

出组（世代 id 失效，旧句柄安全）；返回是否实际移除。

- :rtype: bool

##### <span class="c-func">alive(sprite) -> bool</span>

句柄是否存活（未 add 过直接 False）。

- :rtype: bool

##### <span class="c-func">count() -> int</span>

组内存活精灵数。

- :rtype: int

##### <span class="c-func">position(sprite) -> tuple[int, int] | None</span>

查询位置（组内不存在该 id = None）。

- :rtype: tuple[int, int] | None

##### <span class="c-func">move_by(sprite, dx, dy)</span>

相对位移。

##### <span class="c-func">set_position(sprite, x, y)</span>

绝对定位。

##### <span class="c-func">set_z(sprite, v) / set_rotation(sprite, deg) / set_scale(sprite, f) / set_visible(sprite, b)</span>

入组后动态改属性（层级 / 旋转 / 缩放 / 可见性）。

##### <span class="c-func">with_tag(tag) -> int</span>

标签过滤——v1 返回存活数量。

- :rtype: int

##### <span class="c-func">tick(dt)</span>

推进帧动画 / 时序。

- :param dt: 帧间隔秒数
- :type dt: float

##### <span class="c-func">draw()</span>

全组绘制到活跃会话——**单形态定约：须在 `with screen.render() as r:`
块内调用**。

```python
with screen.render() as r:
    group.draw()
```

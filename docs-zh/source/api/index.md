# API

```{note}
📌 文档持续铺开中。**已完成**：``pygame.event`` / ``pygame.key``。
**待铺开**：display / draw / font / image / mixer / time / mouse /
io / gui / physics 等模块页。
```

## <span class="c-func">分类着色图例</span>

API 页的成员标题按类别**着色 + 同色淡调底线**；标题缩进即层级
（分类 → 类型 → 成员）。

| 标记 | 类别 | 说明 |
|---|---|---|
| <span class="c-const">常量</span> | 石墨灰 | 模块级常量（`QUIT`、`K_a`、`KMOD_SHIFT`…）|
| <span class="c-func">函数</span> | Rust 橙 | 模块函数与普通方法（`get()`、`get_pressed()`…）|
| <span class="c-class">类</span> | 焦橙棕 | 类（`Event`、`Key`…）|
| <span class="c-magic">魔术方法</span> | 紫 | `__init__` / `__repr__` / `__eq__` 等 |
| <span class="c-attr">属性</span> | 深青 | 实例属性 / 字段（`Event.type`、`Event.key`…）|

```{toctree}
:maxdepth: 1

event
key
```

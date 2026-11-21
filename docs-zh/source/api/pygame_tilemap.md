# pygame_tilemap

```{warning}
⚠️ **使用前必读**：本模块**编译默认不含**——feature `tilemap` 门控。
构建：`cargo build --features tilemap`（stable 可），否则
`import pygame_tilemap` 即 ImportError。
```

Tiled 编辑器地图绑定——上游
[tiled](https://crates.io/crates/tiled)（0.16.0，MIT——TMX/TSX 格式
标准解析，Tiled Map Editor 行业格式）。经 `ResourceReader` trait 闭包
blanket impl 接 `starfish_pygame::sfx::read_block`——TMX/外部 TSX/
图集引用全走密文资源链（dev 宿主明文透传，runtime 全平台统一），绑
定零 fs 直访。**独立顶层 Python 模块**（`import pygame_tilemap` 直达）。

```{note}
**模块分工**：本模块只负责数据解析，不包含渲染——每层的渲染数组出口
对接 `pygame.render` 的 mesh / pipeline。着色器渲染分层扩展：每层
一个 mesh 一个 pipeline，按层序绘制（视差 / 光照 / 水波等层专属
shader 分层叠加）——样张见 `tilemap_layers_test.py`（三层三 shader）。

**坐标系**：层格 (x, y) 对应像素 `(x * tile_width, y * tile_height)`
（Tiled y 向下、顶格对齐，世界原点为层左上）；UV 为图集归一化域
（翻转的 UV 序列变换在绑定层处理，Python 侧零感知）。

**当前限制**：仅支持有限 tile 层（无限层 / 对象层 / 图片层只返回
layer_info 类别）；图集 tileset 支持（image-collection tileset 跳过）；
对象层绑定面待 v2。
```
```

## <span class="c-class">类</span>

### <span class="c-class">Tilemap —— Tiled 地图会话</span>

tiled::Map 薄壳（多层 tile 层 + 图集）。

#### <span class="c-func">Tilemap(path)</span>

从 TMX 资源路径解析（sfx 密文链——外部 TSX 引用同链；解析失败抛
ValueError）。

#### <span class="c-func">width() -> int / height() -> int</span>

地图宽/高（格数）。

#### <span class="c-func">tile_width() -> int / tile_height() -> int</span>

瓦片像素尺寸。

#### <span class="c-func">layer_count() -> int</span>

层数（含 tiles/objects/image/group 各类）。

#### <span class="c-func">layer_info(i) -> (name, kind, visible, opacity)</span>

层元数据——kind ∈ `"tiles"` / `"objects"` / `"image"` / `"group"`。

#### <span class="c-func">layer_tiles(i) -> [(x, y, tileset_index, tile_id, fh, fv, fd)]</span>

有限 tile 层全量展开（air 格跳过；fh/fv/fd = 三向翻转标志）。
tile_id = 图集内局部 id（UV 切分基准）。非 tile 层/无限层抛
ValueError。

#### <span class="c-func">layer_mesh(i) -> (verts, uvs)</span>

per-tile quad 顺序展开渲染数组（**每 tile 6 顶点**两三角）——verts
= 像素域 x/y，uvs = 图集归一 UV。布局 `["float32x2", "float32x2"]`
直喂 `pygame.render.access().mesh()`；空层返回空列表。

#### <span class="c-func">tileset_info(i) -> (name, image_path, img_w, img_h, tile_w, tile_h, columns)</span>

图集切分元数据（图字节 py 侧 `pygame.image.load(image_path)` 走
sfx；外部 TSX 引用同链）。image-collection tileset 抛 ValueError。

#### <span class="c-func">tileset_count() -> int</span>

图集数。

## 样例

`pygamest-dev/examples/tilemap_layers_test.py`——程序生成 TMX（3 层
finite 地图）+ 三 shader 分层渲染（ground 贴图 / deco tint / fx 水
波 alpha）+ 600 帧自动退；控制台 `[tilemap-layers]` 逐项断言。

# pygame_voxelis

```{warning}
⚠️ **使用前必读**：本模块**编译默认不含**——feature `voxelis` 门控。
构建：`cargo build --features voxelis`（stable 可），否则
`import pygame_voxelis` 即 ImportError。
```

稀疏体素八叉树绑定——上游
[voxelis](https://crates.io/crates/voxelis)（25.4.0，MPL/Apache 双许可）
——VoxModel 分块体素树（chunk 八叉树 VoxTree 承载 + VoxInterner 去重
存储），MC 类玩法供数：体素编辑 + 内建 mesh 数组出口。**独立顶层
Python 模块**（`import pygame_voxelis` 直达）。

```{note}
[分工定约] 本模块只供数不包渲染/物理——渲染走 `pygame.render` 的
mesh/pipeline 面（generate_mesh() 出口直接喂 mesh_builder），物理/
射线走 `pygame_physics` 的 physics3d 面（cuboid per 体素 + cast_ray）。

[坐标定约] 全局体素坐标（整数格）直接编辑——chunk 定位
（div_euclid）+ chunk 内局部坐标（rem_euclid）绑定层封装，Python 零
octree 概念；负坐标支持（MC 无限平面语义）。体素世界尺寸 =
chunk_world_size / 2^max_depth。

[air 语义] 体素值 0 = 空（上游 `voxel != T::default()` 分支证明
set(pos, 0) 即删除并 prune 空节点）；非 0 值 = 放置/改色（interner
去重存储）。get_voxel 对空格返回 None。

[渲染出口] generate_mesh() 面剔除不在上游能力内（tree 遍历跳空块，
被遮挡内面照画）——案例级规模直接可用；大规模玩法建议 v2 增量
remesh/面剔除。
```

## <span class="c-class">类</span>

### <span class="c-class">VoxWorld —— 体素世界会话</span>

VoxModel 薄壳（稀疏分块体素树 + interner 去重）。

#### <span class="c-func">VoxWorld(max_depth=4, chunk_world_size=16.0, memory_budget=16MB)</span>

构造（三参全可选）。`max_depth` = 每 chunk 八叉树深度（每轴体素数 =
2^max_depth，默认 4 = 16×16×16/chunk）；`chunk_world_size` = chunk
世界边长（默认 16 → 体素 1.0）；`memory_budget` = interner 内存预算
字节。chunk 按需创建（写哪建哪）。

超限（max_depth 过大 / chunk_world_size ≤ 0）抛 ValueError。

#### <span class="c-func">set_voxel(x, y, z, value)</span>

置值（全局体素坐标）。value=0 即删除（air 语义，prune 空节点）；非
0 值 = 放置/改色。

#### <span class="c-func">remove_voxel(x, y, z)</span>

删除体素（set_voxel(…, 0) 别名——MC 语义显式面）。

#### <span class="c-func">get_voxel(x, y, z) -> int | None</span>

查询体素值（空/air 均回 None）。

#### <span class="c-func">generate_mesh() -> (vertices, normals, indices)</span>

渲染数组出口——三个列表（f32/f32/u32 语义），全局坐标（含 chunk
世界 offset）；空 chunk 跳过，LOD 恒 0。接
`pygame.render.access().mesh(["float32x3", "float32x3"], …)`：
indices 顺序展开成 (pos3+normal3)×N 顶点流或走索引面自定。

#### <span class="c-func">voxel_size() -> float</span>

体素世界尺寸 = chunk_world_size / 2^max_depth（物理 collider half
extent 与命中点→体素坐标换算的基准）。

#### <span class="c-func">chunk_count() -> int</span>

chunk 表长度（⚠️ 上游 VoxModel::new 预建 world_bounds 网格全量
chunk——非稀疏指标，调试打点用）。

#### <span class="c-func">clear()</span>

全清 chunk 表（⚠️ interner 项引用计数不回退——上游无 interner 清面，
案例级内存可接受；长会话建议重建 VoxWorld 代替）。

## 样例

`pygamest-dev/examples/voxel_mc_test.py`——极简 MC：稀疏地形 + rapier3d
体素物理（表面 cuboid + 动态方块发射）+ cast_ray 选择（删除/放置 +
collider 同步 + remesh）+ 法线光照程序染色。控制台 `[voxel-mc]`
逐项断言。

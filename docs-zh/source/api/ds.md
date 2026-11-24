# pygame.ds

游戏数据结构层（**stdlite 定约 2026-11-24**）——starfish-ds 直通绑定：
Heap（二叉最小堆）/ SortedList（有序数组）/ UnionFind（并查集）。
全部 Rust 侧直算——**值载荷索引定约**：堆/有序表体只持 `(f64 优先级,
usize 索引)` 配对，py 值由绑定层 Vec 侧表持有，零 `__lt__` 回调编组
（A* 主场景 = f32 cost 直算，一次边界穿越一批操作）。

## 快速上手

```python
import pygame

# ── Heap：优先队列（Dijkstra / A* 开放表）──
open_table = pygame.ds.Heap()
open_table.push(2.5, "node_a")            # (priority, value)
open_table.push(1.0, "node_b")
open_table.push(0.5, "node_c")
print(open_table.len())                   # 3
print(open_table.pop())                   # (0.5, 'node_c') 最小优先级先出

# 批量建堆（Floyd 线性下滤——比逐个 push 快）
heap2 = pygame.ds.Heap()
heap2.from_pairs([(3.0, "c"), (1.0, "a"), (2.0, "b")])

# push_pop：入堆即出（A* 高频组合——被顶掉的旧项或自身直接返回）
old = heap2.push_pop(0.2, "new")

# ── SortedList：有序数组（bisect 等价）──
sl = pygame.ds.SortedList()
sl.insert(1.5, "enemy_a")
sl.insert(0.3, "enemy_b")
sl.insert(2.7, "enemy_c")
print(sl.first())                         # (0.3, 'enemy_b')
print(sl.last())                          # (2.7, 'enemy_c')
print(sl.bisect_left(1.0))                # 1（左插入位）
print(sl.range(0.0, 2.0))                 # ['enemy_b', 'enemy_a']

# ── UnionFind：并查集（路径压缩 + 按秩合并）──
uf = pygame.ds.UnionFind(10)              # n 个独立元素
uf.union(0, 1)
uf.union(1, 2)
print(uf.connected(0, 2))                 # True
print(uf.connected(0, 3))                 # False
print(uf.components())                    # 8（10 - 2 次有效合并）
```

## 典型用法：A* 寻路开放表

```python
def astar(grid, start, goal):
    open_table = pygame.ds.Heap()
    open_table.push(0.0, start)
    g_cost = {start: 0.0}
    while open_table.len():
        _, cur = open_table.pop()
        if cur == goal:
            return True
        for nxt in grid.neighbors(cur):
            ng = g_cost[cur] + grid.step_cost(cur, nxt)
            if ng < g_cost.get(nxt, float("inf")):
                g_cost[nxt] = ng
                open_table.push(ng, nxt)   # cost 直算零回调
    return False
```

## API 一览

### pygame.ds.Heap

| 方法 | 说明 |
|---|---|
| `Heap(capacity=...)` | 构造（capacity 为预估提示） |
| `push(priority, value)` | 入堆 |
| `pop()` | 最小 priority 项出堆 → `(priority, value)`，空堆 None |
| `push_pop(priority, value)` | 入堆即出 → `(priority, value)`（被顶掉的旧项或自身） |
| `from_pairs(pairs)` | 批量建堆（Floyd 线性下滤），pairs = `[(priority, value)]` |
| `len()` | 堆内项数 |
| `is_empty()` | 空判定 |

### pygame.ds.SortedList

| 方法 | 说明 |
|---|---|
| `SortedList()` | 构造 |
| `insert(key, value)` | 有序插入（二分定位 O(log n) 查位 + O(n) 搬移） |
| `get(index)` | 有序第 i 项 → `(key, value)`，越界 None |
| `first()` / `last()` | 最小/最大键项 |
| `bisect_left(key)` / `bisect_right(key)` | 插入位 |
| `contains(key)` | 键存在性 O(log n) |
| `remove_at(index)` | 移除有序第 i 项 |
| `range(lo, hi)` | 键 ∈ [lo, hi) 的值列表 |
| `len()` / `is_empty()` | 规模 |

### pygame.ds.UnionFind

| 方法 | 说明 |
|---|---|
| `UnionFind(n)` | n 个独立元素 |
| `find(a)` | 根（路径压缩） |
| `union(a, b)` | 合并，不同树才合并返回 True |
| `connected(a, b)` | 连通判定 |
| `components()` | 当前连通分量数 |
| `len()` | 元素总数 |

```{note}
📌 **值载荷索引定约**：堆/有序表内不持 py 对象——Rust 体只存
`(f64 优先级, usize 索引)`，值存绑定层侧表。`pop()` 后该项的侧表
槽不回收（A* 短生命周期场景无碍；超长生命周期大流量场景注意侧表
缓慢增长，必要时重建实例）。
```

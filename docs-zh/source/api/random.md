# pygame.random

确定性随机层（**stdlite 定约 2026-11-24**）——starfish-rng（PCG32）直通
绑定：**跨平台确定性**（同 seed 同序列——PC 端/Android/web 逐位一致，
回放/种子地图/联调取证基础）。自实现零三方依赖（CPython random 依赖
math/os/_random 全缺——RP Lib 路线走不通的正解）。

双面定约：**模块级函数走默认流**（thread_local PCG32——零实例快取），
`Random(seed)` **独立流**（多系统子流互不踩——地图生成/AI/视觉噪声各持
一个，序列互不干扰）。

## 快速上手

```python
import pygame

# ── 模块级（默认流）──
pygame.random.seed(42)
print(pygame.random.random())             # [0.0, 1.0)
print(pygame.random.uniform(-1.0, 1.0))   # 区间均匀
print(pygame.random.randint(1, 6))        # 骰子（含两端）

face = pygame.random.choice(["rock", "paper", "scissors"])
hand = pygame.random.shuffle([1, 2, 3, 4, 5])  # 新列表返回（原表不动）

print(pygame.random.gaussian(0.0, 1.0))   # 正态 (mu, sigma)

# ── 2D/3D 游戏特化 ──
print(pygame.random.unit_vec2())          # 单位圆均匀方向
print(pygame.random.unit_vec3())          # 球面均匀方向（无极点偏聚）
print(pygame.random.in_circle(5.0))       # 圆内均匀点（半径 5）
print(pygame.random.in_sphere(5.0))       # 球内均匀点

# ── 批量口（一次边界穿越 n 个——粒子/地形高频）──
vals = pygame.random.rand_list(1000)      # [f64] × 1000

# ── 独立确定性流 ──
terrain = pygame.random.Random(1234)
enemy_ai = pygame.random.Random(5678)
print(terrain.randint(0, 100), enemy_ai.randint(0, 100))  # 序列互不踩
terrain.reseed(4321)                      # 实例重播种
```

## 典型用法：种子地形（跨平台可复现）

```python
def gen_world(seed):
    rng = pygame.random.Random(seed)      # seed 即存档值——世界可复现
    heights = [rng.gaussian(0.0, 10.0) for _ in range(256)]
    props = [(rng.in_circle(50.0)) for _ in range(30)]
    return heights, props
```

## API 一览

### 模块级函数（默认流）

| 函数 | 说明 |
|---|---|
| `seed(n)` | 重播种默认流 |
| `random() -> float` | `[0.0, 1.0)` |
| `uniform(a, b) -> float` | 区间均匀 |
| `randint(a, b) -> int` | 含两端整数 |
| `choice(seq)` | 序列取一（通用序列口——list/tuple/str） |
| `shuffle(seq) -> list` | **新列表** Fisher-Yates（in-place 突变留 v2） |
| `gaussian(mu, sigma) -> float` | 正态分布 |
| `unit_vec2() -> (x, y)` | 单位圆均匀方向 |
| `unit_vec3() -> (x, y, z)` | 球面均匀方向 |
| `in_circle(r) -> (x, y)` | 圆内均匀点 |
| `in_sphere(r) -> (x, y, z)` | 球内均匀点 |
| `rand_list(n) -> list[float]` | 批量口（一次穿越 n 个） |

### pygame.random.Random

独立确定性流（`Random(seed)` 构造）——同上全 API 实例化
（`r.random()` / `r.uniform(a,b)` / `r.randint(a,b)` / `r.gaussian(mu,sigma)` /
`r.unit_vec2()` / `r.unit_vec3()` / `r.in_circle(r)` / `r.in_sphere(r)` /
`r.rand_list(n)`），另：

| 方法 | 说明 |
|---|---|
| `reseed(n)` | 实例重播种 |
| `choice_from(len) -> int` | len 索引取一（py 侧与 choice(seq) 组装） |

```{note}
📌 **f32 内核 f64 出口**：PCG32 产 f32 精度随机数，绑定层显式升 f64
出口——py 面全 f64 一致形态。**跨平台确定性前提** = 同 seed + 同 API
调用序列（多实例各持独立流，勿混默认流做联机状态）。
```

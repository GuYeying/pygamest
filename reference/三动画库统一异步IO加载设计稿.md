# 三动画库统一异步 IO 加载设计稿

> 2026-11-07 · 状态：**设计定稿，未开工**
> 前置：spine(wg○) / ozz(ozz-animation-rs) / live2d(mocari) 三库加载面已映射

## 一、现状：三库加载数据面

| 库 | 路径加载 | 字节加载 | 依赖文件自动加载 | crates.io |
|---|---|---|---|---|
| **spine** (chine 0.2.0) | `from_path` ❌ | `from_json(&str)` + `from_binary(&[u8])` ✅ | **无**——单文件自足 | ✅ crates.io |
| **ozz** (ozz-animation-rs git) | `from_path` ❌ | `from_archive(&mut impl Read)` ✅ | **无**——单 archive 自足 | ❌ git 依赖（上游无 crates.io 发布） |
| **live2d** (mocari vendored) | `load_model_runtime(path)` ✅ | `load_model_runtime_from_assets(RuntimeModelAssets)` ✅（零 fs 访问——字段全 pub） | **有**——model3.json 引用 moc3/纹理（同目录） | ❌ vendored（上游仓已删） |

## 二、"自行导入依赖文件"问题回答

| 库 | 是否自行加载文件 | 方案 |
|---|---|---|
| **spine** | **否**——`from_json/from_binary` 纯字节入参，零 fs 访问 | 无需解决 ✅ |
| **ozz** | **否**——`from_archive(impl Read)` 接受任意 Reader | 无需解决 ✅ |
| **live2d** | **是**——`load_model_runtime(path)` 内部：读 model3.json → 按引用读同目录 moc3/纹理 | **不另开接口**——绑定层新增 async `load_runtime(path)`：FRAME_BRIDGE 异步逐文件 fetch → 组装 `RuntimeModelAssets` → 调用已有 `load_model_runtime_from_assets`（零 fs 路径） |

**核心思路**：不是给上游加接口，而是**绑定层用 RP async 桥把"读文件"变成"读数据"**——调用方给路径，绑定层异步逐文件 fetch 字节，拼装成上游已支持的 bytes/结构体入参。上游零改动。

## 三、统一异步 IO 接口设计

### 3.1 底层：pygame.io.load（跨平台异步字节加载）

```python
# pygame.io 新增（async——RP FRAME_BRIDGE 模式，同 image.load_async）
bytes = await pygame.io.load(path)       # 桌面: block_on(fs::read) / web: fetch
text  = await pygame.io.load_text(path)  # bytes + UTF-8 decode
```

Rust 侧实现 = 现有 io.rs FRAME_BRIDGE 的 `__await__` yield → host poll →
fetch/fs::read → 填槽恢复。与 `image.load_async` 同一机制。

### 3.2 三库绑定层 async 加载

```python
# ── spine（已有——纯文本/字节直入）──
sdata = pygame_spine.load_json(text)
sdata = pygame_spine.load_binary(bytes)

# ── ozz（已有——纯字节直入）──
player = pygame_ozz.Player(skel_bytes, anim_bytes)

# ── live2d（新增 async——绑定层逐文件 fetch + 组装）──
rt = await pygame_live2d.load_runtime_async(model_dir)
# 绑定层内部：
#   1. model3_text = await pygame.io.load_text(f"{dir}/X.model3.json")
#   2. 解析 model3 → 提取 moc3/textures 文件名
#   3. moc3_bytes = await pygame.io.load(f"{dir}/{moc3_name}")
#   4. textures = [await pygame.io.load(f"{dir}/{tex_name}") for ...]
#   5. 组装 RuntimeModelAssets → load_model_runtime_from_assets
```

### 3.3 live2d bytes 构造链（Rust 侧已有，补 Python 面）

```rust
// pygame_live2d 新增 pub fn（薄适配——上游 RuntimeModelAssets 已支持）
pub fn load_runtime_from_data(
    model3_text: &str,
    moc3_bytes: &[u8],
    texture_files: &[(String, Vec<u8>)],  // (文件名, PNG 字节)
) -> Result<RuntimeModel, AssetLoadError> {
    let model3: Model3 = serde_json::from_str(model3_text)?;
    let textures = texture_files.iter()
        .map(|(name, bytes)| DecodedTexture::from_png(name, bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let assets = RuntimeModelAssets {
        model: model3,
        moc: moc3_bytes.to_vec(),
        physics: None,      // 可选——按 model3 引用延迟提供
        pose: None,
        textures,
        model_dir: PathBuf::new(),  // 纯数据形态——无目录
    };
    load_model_runtime_from_assets(assets)
}
```

### 3.4 跨平台行为

| 平台 | `pygame.io.load` | 三库加载 | 效果 |
|---|---|---|---|
| **桌面 Win/Mac/Linux** | `block_on(fs::read)` | 全部可用 | 同步语义（异步壳） |
| **Web** | `fetch(path)` spawn_local | spine/ozz ✅；live2d 需 FRAME_BRIDGE | 异步语义 |
| **Android** | `block_on(asset read)` | 全部可用 | 同步语义 |

## 四、Python 统一案例（目标形态）

```python
import pygame
import pygame_spine, pygame_ozz, pygame_live2d

async def main():
    screen = pygame.display.set_mode((800, 600))

    # ── 统一异步 IO 加载（零平台分支）──
    spine_json = await pygame.io.load_text("assets/spineboy/spineboy-ess.json")
    spine_atlas = await pygame.io.load_text("assets/spineboy/spineboy.atlas")
    spine_png = await pygame.io.load("assets/spineboy/spineboy.png")

    ozz_skel = await pygame.io.load("assets/astro/astro_max_skeleton.ozz")
    ozz_anim = await pygame.io.load("assets/astro/astro_max_animation.ozz")

    # live2d：一行 async（绑定层内部逐文件 fetch + 组装）
    lrt = await pygame_live2d.load_runtime_async("assets/models/Hiyori")

    # ── 构建播放会话 ──
    sk = pygame_spine.Skeleton(spine_json_data)
    ...
```

## 五、实施里程碑

| 阶段 | 内容 | 量级 |
|---|---|---|
| **M1** | `pygame.io.load/load_text` 异步字节加载口（FRAME_BRIDGE 模式） | ~30 行（io.rs 已有 fetch 桥骨架） |
| **M2** | pygame_live2d `load_runtime_from_data` Rust 面 + `load_runtime_async` Python 面 | ~80 行（RuntimeModelAssets 已有） |
| **M3** | 三库 demo 统一改走异步 IO（examples 迁移） | 脚本改造 |
| **M4** | wasm/android 回归（跨平台闭环验证） | 构建+部署 |

## 六、panic 注意事项

- `pygame.io.load`：路径不存在 → **ValueError**（不 panic）
- `pygame_live2d.load_runtime_async`：model3 解析失败 → **ValueError**；
  moc3 坏数据 → **ValueError**；纹理解码失败 → **ValueError**（不 panic）
- `pygame_spine.load_json/load_binary`：坏数据 → **ValueError**（已标注）
- `pygame_ozz.Player`：archive 坏数据 → **OzzError**→ **ValueError**（不 panic）

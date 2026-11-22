# AI 速查文档（agent 专用）

本页面向 AI agent：**不翻长文档、快速建立全局认知、避开已知坑位**。
事实源 = 本页 + `docs-zh/source/api/*.md`（逐模块细节）+ `examples/`（78 案例 = 唯一事实源）。
文档与代码冲突时，**以代码与案例为准**并在回报中标注。

## 1. 项目拓扑与构建

三仓链（均为 GuYeying 名下 GitHub 仓，本地 path 相互 patch）：

```
starfish/            # 引擎 Rust workspace（0.9.x）
  crates/            # starfish-we(窗口/平台) render pygame io audio video
                     # dialog net gamepad font gfx mask time permission debug std
pygame-rs/           # pygamest（0.3.x）= RustPython 解释器 + pygame 全模块绑定
  src/rpy/           # py 绑定层（host.rs = 帧桥 + tasks/io 异步桥核心）
pygame-rs-dev/       # 开发解释器 pygamest-dev + examples/（78 案例）+ web/
  examples/          # Python 案例库——**改案例 = 改这里**
  web/               # wasm 宿主页 + examples 分发副本（手动 cp 同步！）
```

外接：`egui-starfish-we`（egui 桥）、`rapier-starfish-debug`（物理调试）、
`pygame_spine/pygame_live2d/pygame_ozz`（动画 guests，feature 门控）。

**构建**（全部 `cargo +nightly`——ozz 依赖 std::simd，stable 必败属预期）：

| 产物 | 命令 |
|---|---|
| 桌面开发解释器 | `pygame-rs-dev: cargo +nightly build --features voxelis`（MC 类案例；spine/live2d 按需追加） |
| wasm | `pygame-rs-dev/web: python build.py`（产物 web/pkg；大文件资产预置 build.py） |
| Android APK | dev 仓 android_host（NDK；产物重编后须真机复测） |

- `.cargo/config.toml`（dev 仓）把 starfish/pygamest 全家 patch 到本地 path——**新增
  starfish crate 依赖时须同步补 patch 条目**，否则走 GitHub git 源（断网/代理断 =
  解析失败，报"locked to X 但 candidate Y"）。
- cargo 指纹有假增量前科：怀疑没编进去时 `grep -a "新符号" target/debug/python.exe`
  验证；仍无则 `cargo clean -p <crate> --release --target <target>` 强制。
- 版本号：starfish workspace Cargo.toml 与 pygame-rs Cargo.toml 各自独立提升
  （+0.0.1 惯例），双仓各自 commit + push。

## 2. Python API 面（与标准 pygame 的关键差异）

- **模块**：display/event/key/mouse/font/image/mixer/time/draw/gfxdraw/math/
  rect/sprite/transform/mask/sndarray/touch/gamepad/net/permission/version +
  引擎扩展 `render`(GPU 管线) `pygame_gui`(egui) `pygame_physics`(rapier 2d/3d)
  `pygame_voxelis`(稀疏体素) `pygame_tilemap/spine/ozz/live2d` `io` `tasks` `video` `dialog`。
- **坐标系**：逻辑中心原点、y 向上（逻辑像素）；`set_mode((W,H))` 后恒像素域图元
  直接用逻辑像素坐标。
- **鼠标**：`mouse.get_pos/get_pressed/get_rel/set_visible/set_grab/get_grab/set_pos`。
  grab = Win32 ClipCursor / macOS CG；**web = no-op（Pointer Lock 挂账）**、移动端 no-op。
  MOUSEMOTION 事件**无 rel 属性**——`e.x/e.y` 差分自算（last_pos 锚）。
- **按键**：`pygame.key.K_*` 常量 + `key.get_pressed()` 返回 dict 样视图
  （`.get(K_w)`——**Key 对象 unhashable，不能当 dict key 以外用法**）。
- **键盘/文本**：TEXTINPUT 事件走 IME；Android 壳每程序复位 TEXT_INPUT_ACTIVE。
- **render 低层**：`ac = pygame.render.access()` → buffer/mesh/shader/bind_group/
  pipeline/raw_render/raw_compute；`screen.raw_render()` 直绘会话（NDC 域、
  相机 buffer 用户全权管理）。

## 3. 渲染域 v3.1（全显式相机）

- **一切图元 = `present_fit × Screen.camera.mvp()`**（段 override 优先）；
  `gfxdraw.set_camera(rt, cam)` 段级显式。
- **pygame 像素图元 API 恒像素域**（draw/blit/gfxdraw 无相机口/字体/HUD）。
- **无显式相机 = NDC 恒等直通**（raw 段/自写 WGSL 默认域）。
- **`screen.set_camera(cam)` 一次设置缓存**：跨会话/resize/换画布不动；
  `clear_camera()` 显式回落像素基线段。
- 相机类型**唯一** = `pygame.render.game_camera_ortho/perspective(...)`（Auto 隐式
  相机已退役）；`cam.with_view(view)` 喂视图矩阵；`cam.mvp()` 直写相机 buffer。
- 54 个不设相机案例数学逐位不变（pixel_ortho 分解恒等）。

## 4. io 走向矩阵（2026-11-19 定约）

| 接口 | native/Android | wasm |
|---|---|---|
| `io.read/read_text` | block_on fs（**走 sfx 加密面**） | 同步 XHR 全量（裸读**不走加密面**——加密资产 wasm 同步读不支持） |
| `io.write/write_text` | fs 覆盖写 | **Err**（宿主预加载/异步面承载） |
| `io.exists` | fs metadata | 同步 XHR HEAD |
| `io.io_size` | fs metadata | **None**（调用方据此退化全量） |
| `io.io_read_chunk(path,off,n)` | seek+read（**分块流式帧级口**，明文） | **Err**（fetch 全量承载） |
| `io.io_take/io_ready/io_take_chunk` | 完成表（路径键 / `path#off` 分块键） | 同左 |
| `await io.fetch(path)` | 帧首排空队列异步读→入表 | spawn_local 真 fetch→入表（**网络层流式不卡帧**） |

分块流式加载正典（native）：`io_size` → 每帧 `io_read_chunk` 一块（≤8MB）+
`await pygame.tasks.yield_now()`；wasm：`io_size is None` → `await io.fetch(path)` +
`io_take(path)`（参考 `examples/async_io_test.py`）。

## 5. tasks v1（协作式微调度器）

`pygame.tasks.{spawn, gather, sleep_frames, sleep, wait_for, Queue}`；
`await pygame.tasks.yield_now()` 交出控制权一帧（主循环协程内）。

- `t = tasks.spawn(coro, name=...)` → `t.done / t.failed() / t.exception() /
  t.name / t.cancel() / await t.wait()`；`wait()` 返回 `_TaskWait`（恒先 yield
  一帧——规避 RP 裸 StopIteration 病态）。
- 任务内挂起三通道：`yield ("io_read", path)` / `yield ("io_read_chunk",
  path, off, n)` / `await pygame.io.fetch(path)`（推荐——纯 coroutine 无裸 yield）。
- **视频/音乐流式**：task 只负责"找资源不卡帧"（`await video.open_async(path)` /
  `await mixer.music_load_async`），解码/播流 = 平台解码器自治，task 不碰内部加载。
- gather/wait_for 失败重抛：`raise t.exc`（引擎内部已修）。

**RP（RustPython）坑位——与本会话实测强相关**：

1. **委派链丢 return 值**：spawn 任务的 `return X` 恒 None（yield from 委派链）。
   数据走引用承载（hud 属性/容器）或完成表（io_take）。`tasks_min.py` = 定约示范。
2. **裸 generator 不可 await**：`'generator' object can't be awaited`——可等待体
   必须实现 `__await__`（`_TaskWait`/`_SleepFrames`/`_IoFetch` 均为此而生）。
3. **async gen（async def 内裸 yield）驱动有病态**——用纯 coroutine +
   `await pygame.tasks.yield_now()` 或协议 yield 元组（普通 gen）形态。
4. **属性遮蔽方法**：实例属性与同名方法共存时方法不可达（exception 属性遮蔽
   exception() 事故）——属性命名避开方法名。

## 6. 常见坑位清单（实锤事故档案）

**GPU/render**：

- `raw_render` **必须上下文形态**：`with t.raw_render(depth=True) as p:`——裸调用
  `p = t.raw_render()` 命令在 `__exit__` 才排程回放，不进 with = **静默无图**。
- **raw 回放恒在游戏内容之上**（画家定约）——gfxdraw/内容层 HUD 会被 raw 的
  3D 不透明像素盖住。HUD-over-3D 正道 = fluid3d/voxel_mc 的**覆盖面模式**：
  HUD 画进透明底离屏面 → 3D 之后第二个 raw pass alpha 合成回主屏。
- **全屏 quad uv v 翻转**：wgpu 纹理 v=0 = 顶行，quad 直排映射 = 内容上下颠倒
  （fluid3d/voxel_mc 均踩过；纹理顶行 ↔ NDC y=-1）。
- **`gfxdraw.set_color` 是 one-shot**（状态机——每次 draw 消费，漏设回落引擎
  默认紫 128,0,128）；同批多图元逐个重设。
- **fill quad z 的 RH 语义**：RH directx 正交 z 映射 = `-2·z_view-1`（视空间
  前方恒负 z）——背景 quad 远平面 = `z_view=-1.0`；曾写 +1.0 = 相机背后 →
  裁剪剔除 = fill 消失（fill 恒近黑两日无人察觉——BMP 真值 + 纯红判别实锤）。
- `ac.mesh` 默认 **TriangleList**：全屏 quad 要 6 顶点两三角——4 顶点 = 半屏。
- buffer 建完不写 = **全 0**：shader 依赖的 uniform 逐字段核对写入（含 WGSL
  uniform 对齐布局：vec2@0/f32@8/vec3@16 倍数）。
- WGSL 内建名 camelCase：`dpdx/dpdy`（不是 dFdx）。
- 同帧多 raw 会话共享相机 buffer = 后写覆盖（回放延迟性）——各持独立 buf。
- Surface（离屏）颜色格式 `Rgba8Unorm`（ADR-3）、逻辑==物理 fit 恒等；
  主屏格式运行时查询（dump 取证为准）。
- **`ac.update_mesh` 不增长**（裸 queue.write_buffer）——顶点数可变的 mesh
  用**索引驱动**：建 mesh 时容量垫底（零顶点尾段），update 原地写 + 按索引
  计数绘制（draw 恒 index_count）——超容量才整 mesh 重建。动态 mesh 每
  次新建 = GPU 分配 churn + 编辑卡帧（voxel_mc remesh 140→21→6ms 实录：
  ①py 循环挪 Rust ②索引驱动原地更新 ③bytes 直喂消 RP 逐元素转换）。
- **`Surface::draw_surface` 的变换位置 = 贴图中心**（实现 ±hw/2 四角；文档
  "(0,0)-(sw,sh)" 过时）——锚语义按中心锚用。
- **覆盖面离屏面每帧须真清屏**（`surf.clear(color)` = 下一条 pack 的
  GPU LoadOp::Clear）——alpha-blend fill 无法穿透已写像素（透明 quad
  混合后 dst 原样保留）= 文字跨帧堆叠。离屏面尺寸须跟随屏幕
  （`screen.get_size()`）——覆盖面全屏合成 1:1，尺寸失配 = 半比例错位。
  ⚠️ clear 曾走 write_texture 全尺寸上传捷径（2048×1280 = 10.5MB/帧
  CPU→GPU 拷贝，debug 帧耗时 16.7ms@60）→ 改 GPU 清屏后 8.3ms@120。
- **vsync 锯齿**：渲染贴近 vsync 上限时，尖峰帧整帧推迟到下一同步刻 = 直接
  掉半帧率（debug 构建 + 高分辨率易触发）——体感比无 vsync 的均匀掉帧差
  得多；性能验收看**平均帧耗时**而非 fps 数字。
- **体素/物理（parry 0.31）**：voxels 形状射线键在 **`hit.subshape`**
  （flat_id = chunk_id×512 + 格内序），`hit.feature` 只是该 voxel AABB 的面
  id——`voxel_at_flat_id(hit.subshape)` 解码格键；`set_voxel` 原位编辑经
  `ColliderSet::get_mut` 脏标记 → 下步自动刷 broadphase（零刚体插拔）。
- **video wasm（WebCodecs，2026-11-21 接线）**：异步 open 拆两半——
  `_open_start`（spawn_local 驱动 VideoModule::open：fetch+demux+WebCodecs）
  + thread_local 交互盒 + `_poll` 轮询 / `_open_take` 经 `Video::from_base`
  包装。**同步口 VideoSystem.open 恒站桩**（Web 无阻塞 IO 架构性限制）。
  帧交付 = VideoFrame GPU 直拷纹理（copy_external_image_to_texture）。
  ⚠️ 跨 SSH/会话启动的 GUI 进程会被 job object 连坐杀——远端测试用
  计划任务或让用户本地启动。

**平台/发布**：

- `web/*.py` 是 examples 的**分发副本**——改案例后必须 `cp examples/X.py web/`
  （build.py 自动同步挂账；忘 cp = 用户看到旧行为）。
- **feature 门控**：`pygame_voxelis`（--features voxelis）、`fluid`（FluidWorld——
  缺失按 F2 即 AttributeError 崩脚本）、spine/live2d——案例调缺失模块先查
  构建特性，别只看代码。
- **debug 构建的物理/渲染数值代码慢 10-50 倍**（rapier 求解器/wgpu 上传
  无内联）——性能验收用 `target/release/python.exe`；"30 刚体就卡"类
  报障先问构建形态（dev 日常 = debug）。
- **wasm 控制台的日志级别过滤**：starfish-debug 的 wasm 输出器**只落
  INFO 及以上**——`log_debug!` 在 wasm console 不可见（桌面可见）——
  wasm 取证一律 `log_info!`（2026-11-21 video 排查：取证行 DEBUG 级
  全被吞 = 链路"看似没执行"的误报源）。
- wasm 同步 XHR 禁 `responseType`（InvalidAccessError）——二进制走
  `x-user-defined` 字节映射；XHR API = `open/send/set_request_header/
  override_mime_type`（无 set_method）。
- 浏览器启发式缓存吃旧资产：fetch `cache=no-store` / XHR `Cache-Control:
  no-cache` 恒新鲜。
- Android：surface 竞态修在 embedded.rs（release 延迟到引擎完全停止）；
  IME 残留每程序复位；真机 swap 报错先查 detach 时序。
- dev 产物重编后 = 用户可感知行为变化；报"还是不行"先核对二进制/副本真新鲜
  （文件 mtime + grep 符号）。

**流程**：

- 多轮 heredoc/sed 替换事故（残行 `X`、双定义）——复杂 python 修改用 Edit
  工具精确替换，别 heredoc 生成。
- python `print` 到不了 web console（wasm stdout 未接）——web 取证走
  `[log]` 桥行 / canvas HUD / dump 文件；桌面走 stdout 重定向文件。
- PrintWindow 截图可能撞别的窗口（title 匹配竞态）；窗口小 ≠ 渲染错。

## 7. 诊断/取证工具箱

| 工具 | 用法 |
|---|---|
| 帧缓冲真值 | `pygame.render.set_fb_dump(True)` + env `STARFISH_FB_BMP=path` → 逐帧覆写 BMP（present 前唯一有效采样点） |
| 屏幕网格回读 | `pygame.render.screen_probe()` → 7×5 [(x,y,r,g,b,a)]（绕过截图/合成器） |
| 窗口截图 | `pygame-rs-dev/shot_pw.ps1 -Out x.png`（PrintWindow；DWM 合成态兜底） |
| wasm 控制台 | web 宿主页 console log 区（`[log]` 行）；`?script=X.py` 直跑案例 |
| B 探针 | `[B] B0/B1/B2...` 阶段打点（web_start→runtime::run 载荷链） |

## 8. 文档地图

- 模块 API 细节：`docs-zh/source/api/<module>.md`（io.md 内含走向矩阵全表）。
- 渲染域/管线：`api/render.md`；物理：`api/pygame_physics.md`；GUI：`api/pygame_gui.md`。
- 案例即文档：`examples/<case>.py` 文件头注释 = 设计定约记录（**先读头注释再改**）。
- 发布流程：版本 +0.0.1 → 双仓 commit → push github（消息体 = 本批次改动摘要，
  中文一行流惯例）。

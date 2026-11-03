//! 逐模块独立测试案例的共享宿主壳（binding_probe 同构固化）。
//!
//! 模块专项案例（examples/<模块>_test.rs）只提供 Python 脚本，装配/
//! 字体预载/事件泵/帧循环/收口全部由 [`run`] 固化——案例间零重复。
//! 收口顺序与坑位（关窗先于卸载、mixer 显式收口）与 binding_probe
//! 保持一字不差。
//!
//! 用法（案例侧）：
//! ```rust, ignore
//! const SCRIPT: &str = include_str!("font_test.py");
//! async fn run() -> Result<(), Box<dyn std::error::Error>> {
//!     pygamest::rpy::testhost::run(SCRIPT, "font_test").await
//! }
//! starfish::app_entry!(run());
//! ```

use crate::rpy;
use starfish_we::next_frame;
use starfish_pygame::display;
use std::cell::{Cell, RefCell};
use std::io::Write;

/// 帧桥 + async 微调度器（**单一定义，游戏门/脚本直跑门两宿主共用**）：
/// 协作式任务表——spawn/gather/sleep_frames，帧边界推进零竞态；
/// 替代 asyncio 的游戏场景子集（asyncio 已随冻结裁剪移除）
const FRAME_BRIDGE: &str = r#"
class _FrameAwait:
    def __await__(self):
        _async_pump()      # 每帧挂起点先推进 spawn 的任务（协作式调度）
        yield "frame"
_next_frame_state = _FrameAwait()
def next_frame():
    return _next_frame_state

# ── 帧泵驱动的协作式微调度器 ──
class _AsyncTask:
    def __init__(self, coro):
        self.coro = coro
        self.done = False
        self.result = None
        self.wake_frame = 0
        self.wait_io = False
        self.io_path = None

    def tick(self, now_frame):
        if self.done or now_frame < self.wake_frame:
            return
        if self.wait_io:
            # io 未就绪不 send（提前恢复 = 病态挂起/空数据——按路径
            # 完成表判定，2026-11-03 修）
            if not _io_ready(self.io_path):
                return
            self.wait_io = False
            self.io_path = None
        try:
            req = self.coro.send(None)
            if type(req) is tuple and req[0] == "io_read":
                # 任务挂起等 io：压队（Rust 帧首排空发起 fetch）→
                # 下一 tick 起查就绪，就绪帧恢复取数
                _io_pending_append(req[1])
                self.io_path = req[1]
                self.wait_io = True
            elif type(req) is tuple and req[0] == "sleep_frames":
                self.wake_frame = now_frame + req[1]
        except StopIteration as e:
            self.done = True
            self.result = e.value

_async_tasks = []
_async_frame = 0

def _async_pump():
    global _async_frame
    _async_frame += 1
    for t in _async_tasks:
        t.tick(_async_frame)

class _SleepFrames:
    def __init__(self, frames):
        self.frames = frames
    def __await__(self):
        # 单次挂起（yield 一次即返回——任务下一 tick 恢复执行；
        # 无限 yield 会让 wake_frame 永远前移 = 死锁，首版实测）
        yield ("sleep_frames", self.frames)

class _Gather:
    def __init__(self, tasks):
        self.tasks = tasks
    def __await__(self):
        while any(not t.done for t in self.tasks):
            _async_pump()
            yield "frame"
        return [t.result for t in self.tasks]

def _async_spawn(coro):
    # 幂等：传入的已是任务（spawn 产物/gather 复用）直接返回
    if hasattr(coro, "tick"):
        return coro
    t = _AsyncTask(coro)
    _async_tasks.append(t)
    return t

def _async_gather(*coros):
    return _Gather([_async_spawn(c) for c in coros])   # 实例即 awaitable

def _async_sleep_frames(frames):
    return _SleepFrames(max(1, int(frames)))

class _AsyncNs:
    pass

_async_ns = _AsyncNs()
_async_ns.spawn = _async_spawn
_async_ns.gather = _async_gather
_async_ns.sleep_frames = _async_sleep_frames
_async_ns.frame = lambda: _async_frame

# ── 异步 IO 桥（两拍模式，**单层委派构型**——与 next_frame 同构）──
# yield ("io_read", path) → Rust 完成（按路径完成表）→ send(None) 恢复
# → io_take(path) 取字节。恒单层 __await__：主协程/任务直接委派到 gen
# （不走 async def 中间层——协程套协程的多层委派在 RP 实测恢复病态，
# 2026-11-03；io_wait 闸门须帧间持久，循环局部变量会漏闸）
class _IoFetch:
    def __init__(self, path):
        self._p = path
    def __await__(self):
        yield ("io_read", self._p)
        return _io_take(self._p)

def fetch(path):
    # 底层通用口：await 挂起（Rust fetch+解密+按路径入表）→ 恢复取走
    # （.py 自动注册 import；sync 面用 load_from_bytes）
    return _IoFetch(path)

def payload_list():
    return _io_payload

class _IoNs:
    pass

_io_ns = _IoNs()
_io_ns.fetch = fetch
_io_ns.payload_list = payload_list

_io_payload = []

# ── 三线统一异步加载（load = fetch + from_bytes；*_from_bytes 由
#    挂载点从 pygame 模块注入 scope——late binding，调用期解析）──
class _LoadImage:
    def __init__(self, path):
        self._p = path
    def __await__(self):
        yield ("io_read", self._p)
        return _image_from_bytes(_io_take(self._p))

class _LoadSound:
    def __init__(self, path):
        self._p = path
    def __await__(self):
        yield ("io_read", self._p)
        return _mixer_from_bytes(_io_take(self._p))

class _LoadFont:
    def __init__(self, path, size):
        self._p = path
        self._s = size
    def __await__(self):
        yield ("io_read", self._p)
        return _font_from_bytes(_io_take(self._p), self._s)

def _load_image(path):
    return _LoadImage(path)

def _load_sound(path):
    return _LoadSound(path)

def _load_font(path, size):
    return _LoadFont(path, size)

# （资源加密嵌入钥接线已移至 runtime bin Rust 侧——init_crypto 由
# 宿主 main 直接调 starfish_io::init_crypto，不经过 Python 层）
"#;

/// 诊断日志（统一走 starfish-debug log 模块：桌面/Android 文件 +
/// wasm 控制台回退；与 binding_probe 的 dlog 同构）。**Debug 级**——
/// 发行壳 Info 级别下不落盘，dev Debug 级别全见
fn dlog(tag: &str, msg: &str) {
    let line = format!("[{tag}] {msg}");
    starfish_debug::log_debug!(&line);
    #[cfg(target_os = "android")]
    net_log(&line);
    #[cfg(target_os = "android")]
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/sdcard/Download/bp_log.txt")
    {
        let _ = writeln!(f, "{line}");
    }
}

/// 失败/异常出口（恒 Error——发行壳 Info 级别下也必落盘，崩溃取证主通道）
fn elog(tag: &str, msg: &str) {
    let line = format!("[{tag}] {msg}");
    starfish_debug::log_error!(&line);
    #[cfg(target_os = "android")]
    net_log(&line);
}

// ── 异步 IO 桥（pygame.io.fetch 两拍模式的 Rust 侧）──────────────
// Python 单层 `__await__` yield ("io_read", path) → 帧循环发起 fetch
// → 按路径入完成表 → send(None) 恢复 → gen 内 io_take(path) 取走。

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// 主协程正在等的 io 路径（④挂起时记下；完成表就绪后①清闸——
    /// 纯表驱动，无独立信号槽：任务 fetch 完成不扰主路径判据）
    static MAIN_IO_PATH: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// 发起单路径 fetch：wasm = spawn_local 异步入完成表；native = read_sync
/// 直入表（fs 微秒级，同帧可恢复）。已在飞防抖；.py 完成后自动注册
/// import（finder 表）。
fn start_io_fetch(tag: &str, path: &str) {
    if !crate::rpy::bridge::mark_inflight(path) {
        return; // 已在飞（任务/主路径共用去重）
    }
    #[cfg(target_arch = "wasm32")]
    {
        starfish_debug::log_info!(format!(
            "[io-bridge] 发起拍：fetch {path}（spawn_local）"
        ));
        let path2 = path.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let r = starfish_pygame::sfx::read(&path2).await;
            starfish_debug::log_info!(format!(
                "[io-bridge] fetch 完成 {path2} ok={}",
                r.is_ok()
            ));
            match r {
                Ok(b) => {
                    // .py fetch 自动注册 import（fetch→register→import
                    // 定约——热更脚本与 native 分支同构）
                    if path2.ends_with(".py") {
                        let name = path2.trim_end_matches(".py").replace("/", ".");
                        let src = String::from_utf8_lossy(&b).to_string();
                        let _ = crate::rpy::modfind::register_module(
                            &name, &path2, &src, false,
                        );
                    }
                    crate::rpy::bridge::store_io_result(&path2, b);
                }
                Err(e) => {
                    starfish_debug::log_error!(format!(
                        "[io-bridge] fetch 失败 {path2}: {e}"
                    ));
                    crate::rpy::bridge::store_io_result(&path2, Vec::new());
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    match starfish_pygame::sfx::read_sync(path) {
        Ok(b) => {
            // .py fetch 自动注册 import（finder 表）
            if path.ends_with(".py") {
                let name = path.trim_end_matches(".py").replace("/", ".");
                let src = String::from_utf8_lossy(&b).to_string();
                let _ = crate::rpy::modfind::register_module(&name, path, &src, false);
            }
            crate::rpy::bridge::store_io_result(path, b);
        }
        Err(e) => {
            elog(tag, &format!("io fetch 失败 {path}: {e}"));
            crate::rpy::bridge::store_io_result(path, Vec::new());
        }
    }
}

/// 三线统一接口挂载（两处挂载点共用）：from_bytes 引用入 scope（桥内
/// async load late-binding 调用）+ image/mixer/font.load 覆盖为桥 async
/// 版 + pygame.io 扩展（fetch/payload_list）
fn mount_unified_io(
    tag: &str,
    pg: &rustpython_vm::PyObjectRef,
    scope: &rustpython_vm::scope::Scope,
    io_ns: &rustpython_vm::PyObjectRef,
    vm: &rustpython_vm::VirtualMachine,
) {
    for (mod_name, fb) in [
        ("image", "_image_from_bytes"),
        ("mixer", "_mixer_from_bytes"),
        ("font", "_font_from_bytes"),
    ] {
        let m = match pg.get_attr(mod_name, vm) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let f = match m.get_attr("load_from_bytes", vm) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let _ = scope.globals.set_item(fb, f, vm);
    }
    for (mod_name, load_name) in [
        ("image", "_load_image"),
        ("mixer", "_load_sound"),
        ("font", "_load_font"),
    ] {
        if let (Ok(m), Ok(f)) = (pg.get_attr(mod_name, vm), scope.globals.get_item(load_name, vm)) {
            let _ = m.set_attr("load", f, vm);
        }
    }
    if let Ok(io_mod) = pg.get_attr("io", vm) {
        for attr in ["fetch", "payload_list", "io_take", "io_ready", "io_pending_append"] {
            if let Ok(f) = io_ns.get_attr(attr, vm) {
                let _ = io_mod.set_attr(attr, f, vm);
            }
        }
    }
    // 桥内取数/判据口引用（late binding——pygame 不在桥 scope）
    if let Ok(io_mod) = pg.get_attr("io", vm) {
        for (attr, as_name) in [
            ("io_take", "_io_take"),
            ("io_ready", "_io_ready"),
            ("io_pending_append", "_io_pending_append"),
        ] {
            if let Ok(lr) = io_mod.get_attr(attr, vm) {
                let _ = scope.globals.set_item(as_name, lr, vm);
            }
        }
    }
    dlog(tag, "三线统一接口挂载 ✓（image/mixer/font.load → async）");
}

/// 协程单步步进结果
/// 协程单步步进结果
enum StepOutcome {
    /// 本帧逻辑完成——继续帧循环（下一拍常规 send(None)）
    Continue,
    /// web fetch 进行中——帧拍继续（不 send；主等路径完成表就绪后恢复）
    WaitIo,
    /// 协程收尾（StopIteration）或异常（已 elog）
    Done,
}

/// 协程单步（恒 send(None) 恢复——**恢复值不走 RP 委派链**：RP 多层
/// 协程委派对 send 值传播有语义坑；数据走按路径完成表）：
/// yield ("io_read", path) → 帧循环发起 fetch 入表 → send(None) 恢复
/// → gen 内 pygame.io.io_take(path) 按路径取字节。
/// `io_wait` = web 形态下正在等 fetch（**帧间持久**——调用方须声明在
/// 帧循环外，循环局部变量每帧重置 = 漏闸提前恢复，2026-11-03 实测挂死
/// 根因）；`errored` = 运行期异常旗标。
fn step_coro(
    tag: &str,
    entry_obj: &rustpython_vm::PyObjectRef,
    vm: &rustpython_vm::VirtualMachine,
    io_wait: &mut bool,
    errored: &std::cell::Cell<bool>,
) -> StepOutcome {
    use rustpython_vm::convert::TryFromObject;
    use rustpython_vm::builtins::PyStrRef;
    use rustpython_vm::builtins::PyTupleRef;

    // ⓪ 任务级 io 挂起排空（帧首发起 fetch——完成表驱动，双宿主同构）
    for path in crate::rpy::bridge::drain_io_pending() {
        start_io_fetch(tag, &path);
    }

    // ① 主路径就绪判定（wasm：按路径完成表——等待中的 fetch 入表即
    //    放行；任务 fetch 完成只入表不扰主路径判据）
    #[cfg(target_arch = "wasm32")]
    if *io_wait {
        let ready = MAIN_IO_PATH.with(|s| {
            let mut g = s.borrow_mut();
            match g.as_ref() {
                Some(p) if crate::rpy::bridge::io_ready(p) => {
                    let p = g.take().unwrap();
                    Some(p)
                }
                _ => None,
            }
        });
        if ready.is_some() {
            dlog(tag, "①主路径 io 就绪放行");
            *io_wait = false;
        }
    }

    // ② 等 io 中 → 本帧不 send（fetch 在飞；就绪后下一拍恢复）
    if *io_wait {
        return StepOutcome::WaitIo;
    }

    // ③ 恒 None 恢复（next_frame 同款安全路径）
    match vm.call_method(entry_obj, "send", (vm.ctx.none.clone(),)) {
        Ok(req) => {
            // 常规帧拍（"frame" 字符串）静默；其余挂起形态打点（异常
            // 挂起形态 = 恢复语义问题的观测窗口）
            let is_frame = req
                .downcast_ref::<rustpython_vm::builtins::PyStr>()
                .map(|s| s.to_str() == Some("frame"))
                .unwrap_or(false);
            if !is_frame {
                dlog(
                    tag,
                    &format!("③异形挂起 cls={}", req.class().name()),
                );
            }
            // ("io_read", path) 协议识别（其余 yield = 帧桥常规挂起）
            let io_path: Option<String> = PyTupleRef::try_from_object(vm, req.clone())
                .ok()
                .filter(|t| t.len() == 2)
                .and_then(|t| {
                    let head = PyStrRef::try_from_object(vm, t[0].clone()).ok()?;
                    if head.to_str()? != "io_read" {
                        return None;
                    }
                    let s = PyStrRef::try_from_object(vm, t[1].clone()).ok()?;
                    Some(s.to_str()?.to_string())
                });
            match io_path {
                None => StepOutcome::Continue, // 常规挂起（帧桥/任务表）
                Some(path) => {
                    // ④ 真实 IO：发起 fetch（按路径完成表——恢复侧 gen 调
                    //    io_take(path) 取走；恢复值恒 None 不走委派链）
                    #[cfg(target_arch = "wasm32")]
                    {
                        *io_wait = true;
                        MAIN_IO_PATH.with(|s| *s.borrow_mut() = Some(path.clone()));
                        start_io_fetch(tag, &path);
                        StepOutcome::WaitIo
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        start_io_fetch(tag, &path);
                        StepOutcome::Continue // fs 同步入表，同帧即恢复
                    }
                }
            }
        }
        Err(exc) => {
            let obj: rustpython_vm::PyObjectRef = exc.clone().into();
            let exc_cls = obj.class().name().to_string();
            if exc_cls == "StopIteration" {
                starfish_debug::log_info!(format!(
                    "[{tag}] 脚本完成（入口耗尽）"
                ));
            } else {
                errored.set(true);
                elog(tag, &format!("脚本异常:
{}", format_traceback(vm, &exc)));
            }
            StepOutcome::Done
        }
    }
}



/// Android 专用诊断：单行 HTTP POST 到 PC 收集器（300ms 超时 + 一次失败
/// 拉黑——无超时阻塞 connect 在主线程 = ANR）
#[cfg(target_os = "android")]
fn net_log(msg: &str) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static TCP_DEAD: AtomicBool = AtomicBool::new(false);
    if TCP_DEAD.load(Ordering::Relaxed) {
        return;
    }
    let addr: std::net::SocketAddr = "192.168.124.13:9000".parse().unwrap();
    match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)) {
        Ok(mut s) => {
            let req = format!(
                "POST /log HTTP/1.1\r\nHost: 192.168.124.13:9000\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                msg.len(),
                msg
            );
            let _ = s.write_all(req.as_bytes());
            let _ = s.flush();
        }
        Err(_) => TCP_DEAD.store(true, Ordering::Relaxed),
    }
}

/// CJK 字体预载链（三平台；批次 G + Android/Linux 系统候选链——与
/// binding_probe 保持一字不差，命中即 log）
pub async fn preload_cjk(tag: &str) {
    #[cfg(windows)]
    match std::fs::read("C:/Windows/Fonts/msyh.ttc") {
        Ok(b) => {
            starfish_pygame::font::preload("cjk", b);
            dlog(tag, "CJK 字体已预载（系统 msyh.ttc）");
        }
        Err(_) => dlog(tag, "CJK 预载失败（msyh.ttc 不存在）"),
    }
    match starfish_pygame::sfx::read("fonts/cjk.ttf").await {
        Ok(b) => {
            starfish_pygame::font::preload("cjk", b);
            dlog(tag, "CJK 字体已预载（fonts/cjk.ttf）");
        }
        Err(_) => dlog(tag, "CJK 字体未找到（fonts/cjk.ttf）"),
    }
    #[cfg(target_os = "android")]
    {
        const CJK_CANDIDATES: [&str; 7] = [
            "/system/fonts/NotoSansCJK-Regular.ttc",
            "/system/fonts/NotoSansCJKjp-Regular.otf",
            "/system/fonts/NotoSansSC-Regular.otf",
            "/system/fonts/NotoSerifCJK-Regular.ttc",
            "/system/fonts/HarmonyOS_Sans_SC.ttf",
            "/system/fonts/miui.ttf",
            "/system/fonts/DroidSansFallback.ttf",
        ];
        let mut hit = false;
        for p in CJK_CANDIDATES {
            match starfish_pygame::sfx::read(p).await {
                Ok(b) => {
                    starfish_pygame::font::preload("cjk", b);
                    dlog(tag, &format!("CJK 字体已预载（系统 {p}）"));
                    hit = true;
                    break;
                }
                Err(_) => continue,
            }
        }
        if !hit {
            dlog(tag, "CJK 系统字体候选全 miss（中文将缺字）");
        }
    }
    #[cfg(target_os = "linux")]
    {
        const CJK_CANDIDATES: [&str; 7] = [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",   // Debian/Ubuntu fonts-noto-cjk
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",        // Arch/Fedora 布局
            "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf", // 按语言单发 OTF 布局
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",           // 文泉驿微米黑
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",             // 文泉驿正黑
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/truetype/droid/DroidSansFallback.ttf",
        ];
        let mut hit = false;
        for p in CJK_CANDIDATES {
            match starfish_pygame::sfx::read(p).await {
                Ok(b) => {
                    starfish_pygame::font::preload("cjk", b);
                    dlog(tag, &format!("CJK 字体已预载（系统 {p}）"));
                    hit = true;
                    break;
                }
                Err(_) => continue,
            }
        }
        if !hit {
            dlog(tag, "CJK 系统字体候选全 miss（中文将缺字）");
        }
    }
}

/// 经典 Python traceback 格式化（RustPython `vm.write_exception` 直通：
/// Traceback (most recent call last) → 逐帧 File/line/in func →
/// 异常类型: 消息；含 __cause__/__context__ 链与 SyntaxError 行列）。
/// 异常出口（脚本执行 / 生成器创建 / 帧循环）统一走这里。
fn format_traceback(
    vm: &rustpython_vm::VirtualMachine,
    exc: &rustpython_vm::builtins::PyBaseExceptionRef,
) -> String {
    let mut buf = std::io::Cursor::new(Vec::<u8>::new());
    let _ = vm.write_exception(
        &mut rustpython_vm::py_io::IoWriter(&mut buf),
        exc,
    );
    String::from_utf8_lossy(buf.get_ref()).trim_end().to_string()
}

/// 运行一个模块专项测试脚本（生成器门：脚本定义 `main()`，宿主持循环）
///
/// - `script`：include 静态嵌入的 Python 脚本全文；
/// - `tag`：日志前缀（如 "font_test"）；
/// - `transparent`：透明窗口创建（TRANSPARENT 创建旗标 + 引擎透明底
///   清屏——桌宠/悬浮件形态。透明时跳过铺底，脚本内勿填不透明底色）。
pub async fn run(script: &str, tag: &str, transparent: bool) -> Result<(), Box<dyn std::error::Error>> {
    use starfish_pygame::display as dflags;
    dlog(tag, "starting");
    let flags = if transparent { dflags::TRANSPARENT } else { 0 };
    match display::set_mode_ex((800, 600), flags).await {
        Ok(_) => {
            dlog(tag, "set_mode OK");
        }
        Err(e) => {
            dlog(tag, &format!("set_mode ERR: {e}"));
            return Err(e.into());
        }
    }

    preload_cjk(tag).await;

    let interp = rpy::new_interpreter();
    dlog(tag, "RP genesis 完成");

    let gen_cell: RefCell<Option<rustpython_vm::PyObjectRef>> = RefCell::new(None);

    // 装配：原生 pygame 模块注入 → 用户脚本 → 生成器（enter 内、
    // essential init 完成后——init_hook 时点会爆栈）
    let install_err: RefCell<Option<String>> = RefCell::new(None);
    interp.enter(|vm| {
        let run = || -> Result<(), String> {
            let scope = vm.new_scope_with_builtins();
            rpy::install_modules(vm);
            dlog(tag, "pygame 原生模块已注入");
            let code = match vm.compile(
                script,
                rustpython_vm::compiler::Mode::Exec,
                format!("{tag}.py"),
            ) {
                Ok(c) => c,
                Err(e) => {
                    // 语法错 → SyntaxError 异常对象（带文件/行列）→
                    // 同一 traceback 格式化（CPython 一致的诊断输出）
                    let exc = e.into_pyexception(vm, Some(script));
                    return Err(format!(
                        "脚本编译异常:\n{}",
                        format_traceback(vm, &exc)
                    ));
                }
            };
            if let Err(e) = vm.run_code_obj(code, scope.clone()) {
                return Err(format!("脚本执行异常:\n{}", format_traceback(vm, &e)));
            }
            // ── 入口定约（批次 59 归一化定稿，用户定夺）：**唯一形态
            // `async def main():`**（帧体 `await pygame.next_frame()`）
            // ——async 形态向下兼容纯同步写法（帧逻辑照常顺序写），
            // 向上承接将来 `await` 网络 IO 不卡帧（net awaitable 接
            // 帧桥同款机制）；生成器形态（def main + yield）退役。
            let bridge_code = vm
                .compile(
                    FRAME_BRIDGE,
                    rustpython_vm::compiler::Mode::Exec,
                    "<frame_bridge>".to_string(),
                )
                .map_err(|e| format!("帧桥编译失败: {e}"))?;
            if let Err(e) = vm.run_code_obj(bridge_code, scope.clone()) {
                return Err(format!("帧桥注入失败:
{}", format_traceback(vm, &e)));
            }
            // pygame.next_frame / pygame.tasks 挂载（帧桥 → pygame 模块属性）。
            // modload **可选**（_sf_loader 存在才挂——web 形态 module_root/
            // embedded_scripts 双 None 时模块表不装配，帧桥照常供帧；
            // 2026-11-02 web 实测：四件套一票否决 = next_frame 缺失）
            let sf_loader = vm
                .sys_module
                .get_attr("modules", vm)
                .ok()
                .and_then(|mods| mods.get_item("_sf_loader", vm).ok());
            match (
                vm.import("pygame", 0),
                scope.globals.get_item("next_frame", vm),
                scope.globals.get_item("_async_ns", vm),
                scope.globals.get_item("_io_ns", vm),
            ) {
                (Ok(pg), Ok(nf), Ok(an), Ok(io_ns)) => {
                    let mounted_nf = pg.set_attr("next_frame", nf, vm);
                    let mounted_tasks = pg.set_attr("tasks", an, vm);
                    let mounted_ml = sf_loader
                        .and_then(|ml| pg.set_attr("modload", ml, vm).ok());
                    // 三线统一接口挂载（from_bytes 入 scope + async load 覆盖 + io 扩展）
                    mount_unified_io(tag, &pg, &scope, &io_ns, vm);
                    let mounted_io = ();
                    starfish_debug::log_debug!(format!(
                        "[bridge-run] next_frame: {mounted_nf:?} / tasks: {mounted_tasks:?} / modload: {mounted_ml:?} / io: {mounted_io:?}"
                    ));
                }
                _ => {
                    starfish_debug::log_error!(
                        "[bridge-run] 挂载失败（pygame/next_frame/tasks/modload 任一缺失）"
                    );
                }
            }
            let entry = scope.globals.get_item("main", vm).map_err(|_| {
                "入口缺失：脚本须定义 `async def main():`（帧体 await \
                 pygame.next_frame()）".to_string()
            })?;
            let genobj = entry.call((), vm).map_err(|e| {
                format!("入口 main() 调用异常:\n{}", format_traceback(vm, &e))
            })?;
            // 归一化校验：必须是协程（async def）——生成器形态指路
            let cls = genobj.class().name().to_string();
            if cls != "coroutine" {
                return Err(
                    "入口形态错误：main 须为 `async def main():`（生成器形态 \
                     def main + yield 已退役——把 yield 改为 await \
                     pygame.next_frame()）"
                        .to_string(),
                );
            }
            *gen_cell.borrow_mut() = Some(genobj);
            dlog(tag, "async main() 协程已建");
            Ok(())
        };
        if let Err(msg) = run() {
            *install_err.borrow_mut() = Some(msg);
        }
    });
    if let Some(msg) = install_err.into_inner() {
        dlog(tag, &msg);
        display::quit();
        return Err(msg.into());
    }

    // 主循环（调用方持）：事件泵 → Python 帧步进 → 上屏 → 帧拍
    dlog(tag, "entering frame loop");
    // io 等待旗标**帧间持久**（循环内声明 = 每帧重置 → fetch 未完成即
    // 二次 send → RP 病态挂死，2026-11-03 实测根因）
    let mut io_wait = false;
    loop {
        if starfish_pygame::event::pump() || starfish_std::run_control::stop_requested() {
            starfish_debug::log_warn!("[host] 嵌入式停止信号命中——帧循环收口");
            dlog(tag, "退出请求，结束");
            break;
        }

        let mut done = false;
        let mut errored = std::cell::Cell::new(false);
        interp.enter(|vm| {
            let borrowed = gen_cell.borrow();
            let Some(entry_obj) = borrowed.as_ref() else {
                done = true;
                return;
            };
            // 协程步进（共享步进器）：send 恢复——await 帧桥挂起 = 本帧完成；
            // await io fetch = 两拍桥（native 即取 / web spawn 后续帧恢复）
            match step_coro(tag, entry_obj, vm, &mut io_wait, &mut errored) {
                StepOutcome::Continue | StepOutcome::WaitIo => {}
                StepOutcome::Done => done = true,
            }
        });
        if done {
            break;
        }

        starfish_pygame::mixer::pump(); // Web 流式解码驱动（桌面 no-op）
        next_frame().await;
    }
    // 先关窗再卸载（RP 卸载期间窗口 ghost）；mixer 显式收口（TLS 析构期）
    display::quit();
    starfish_pygame::mixer::quit();
    dlog(tag, "main loop ended");
    Ok(())
}

/// 运行任意 Python 脚本（**pygamest-dev 脚本直跑形态**——与 [`run`] 的游戏门
/// 同构的完整驱动）。自建解释器后转交 [`run_script_in`]。
pub async fn run_script(script: &str, source_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    run_script_in(crate::rpy::new_interpreter(), script, source_name, Vec::new()).await
}

/// [`run_script`] 的**解释器外置版**（pygamest-dev 用）：解释器由调用方装配并
/// 持有——genesis / sys.argv 接线等启动段可先行于建窗完成（控制台
/// 阶段先行，窗口出现即进入脚本执行）。
///
/// 流程：帧桥注入 → 顶层执行 → 入口自动探测（`async def main():` 协程
/// → 帧循环驱动；无 main / 同步 main / 非函数 main = 顶层执行完即收
/// ——python 语义：定义 ≠ 调用）→ 关窗 → mixer 收口。
/// 编译错 = SyntaxError 经典 traceback（与 [`run`] 同款诊断）；
/// 运行期异常（帧循环内）= 打印 traceback 后返回 Err（调用方据此置
/// 退出码 1，python 同语义）。
/// 首帧打点（"装配段→首帧"耗时）常开——启动诊断仪表；可见性不做
/// 任何刻意安排（2026-10-26 用户定夺：白窗已短暂，HIDDEN+首帧后显示
/// 的机制不跨平台，退役）。
pub async fn run_script_in(
    interp: rustpython_vm::Interpreter,
    script: &str,
    source_name: &str,
    payload: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = starfish_std::debug::Stopwatch::now();
    let err: RefCell<Option<String>> = RefCell::new(None);
    let gen_cell: RefCell<Option<rustpython_vm::PyObjectRef>> = RefCell::new(None);
    let errored = Cell::new(false);
    interp.enter(|vm| {
        let run = || -> Result<(), String> {
            let scope = vm.new_scope_with_builtins();
            crate::rpy::install_modules(vm);
            // 帧桥注入（与游戏门同款——脚本直跑形态同样支持 async
            // def main + await pygame.next_frame() 的协程游戏）
            let bridge_code = vm
                .compile(
                    FRAME_BRIDGE,
                    rustpython_vm::compiler::Mode::Exec,
                    "<run_script frame_bridge>".to_string(),
                )
                .map_err(|e| format!("帧桥编译失败: {e}"))?;
            if let Err(e) = vm.run_code_obj(bridge_code, scope.clone()) {
                return Err(format!("帧桥注入失败:
{}", format_traceback(vm, &e)));
            }
            // payload 清单注入（pygame.io.payload_list 的数据源——按需
            // fetch 的文件名列表；.py 已由 io.fetch 自动注册 import）
            let pl: Vec<rustpython_vm::PyObjectRef> = payload
                .iter()
                .map(|s| vm.ctx.new_str(s.clone()).into())
                .collect();
            scope
                .globals
                .set_item("_io_payload", vm.ctx.new_list(pl).into(), vm)
                .map_err(|e| format!("_io_payload 注入失败: {}", format_traceback(vm, &e)))?;
            let sf_loader = vm
                .sys_module
                .get_attr("modules", vm)
                .ok()
                .and_then(|mods| mods.get_item("_sf_loader", vm).ok());
            // modload 可选（web 双 None 形态模块表不装配——见 bridge-run 侧定约）
            match (
                vm.import("pygame", 0),
                scope.globals.get_item("next_frame", vm),
                scope.globals.get_item("_async_ns", vm),
                scope.globals.get_item("_io_ns", vm),
            ) {
                (Ok(pg), Ok(nf), Ok(an), Ok(io_ns)) => {
                    let mounted_nf = pg.set_attr("next_frame", nf, vm);
                    let mounted_tasks = pg.set_attr("tasks", an, vm);
                    let mounted_ml = sf_loader
                        .and_then(|ml| pg.set_attr("modload", ml, vm).ok());
                    // 三线统一接口挂载（from_bytes 入 scope + async load 覆盖 + io 扩展）
                    mount_unified_io("pygamest-dev", &pg, &scope, &io_ns, vm);
                    let mounted_io = ();
                    starfish_debug::log_debug!(format!(
                        "[bridge] next_frame: {mounted_nf:?} / tasks: {mounted_tasks:?} / modload: {mounted_ml:?} / io: {mounted_io:?}"
                    ));
                }
                _ => {
                    starfish_debug::log_error!(
                        "[bridge] 挂载失败（pygame/next_frame/tasks 任一缺失）"
                    );
                }
            }
            let code = match vm.compile(
                script,
                rustpython_vm::compiler::Mode::Exec,
                source_name.to_string(),
            ) {
                Ok(c) => c,
                Err(e) => {
                    // 语法错 → SyntaxError 异常对象 → 经典 traceback
                    //（与游戏门 run 同款诊断；第二参 = 源文本（脱字符
                    // 定位行显示用），非文件名）
                    let exc = e.into_pyexception(vm, Some(script));
                    return Err(format!("脚本编译异常:\n{}", format_traceback(vm, &exc)));
                }
            };
            if let Err(e) = vm.run_code_obj(code, scope.clone()) {
                return Err(format!("脚本执行异常:\n{}", format_traceback(vm, &e)));
            }
            dlog(
                source_name,
                &format!("顶层执行完成（{}ms）", t0.ms()),
            );
            // ── 入口自动探测：`async def main()` 协程 → 记入 gen_cell
            // 交外层驱动循环。同步 main 不代调、非函数名 main 忽略
            //（python 语义：定义 ≠ 调用）。
            if let Ok(main_f) = scope.globals.get_item("main", vm) {
                let is_fn = main_f
                    .downcast_ref::<rustpython_vm::builtins::PyFunction>()
                    .is_some();
                if is_fn {
                    match main_f.call((), vm) {
                        Ok(co) => {
                            if co.class().name().to_string() == "coroutine" {
                                *gen_cell.borrow_mut() = Some(co);
                            }
                        }
                        Err(e) => {
                            return Err(format!(
                                "入口 main() 调用异常:\n{}",
                                format_traceback(vm, &e)
                            ));
                        }
                    }
                }
            }
            Ok(())
        };
        if let Err(msg) = run() {
            *err.borrow_mut() = Some(msg);
        }
    });
    if let Some(msg) = err.into_inner() {
        // 关窗先于收口（与 run 同约）；消息交调用方打印（单路输出）
        display::quit();
        starfish_pygame::mixer::quit();
        return Err(msg.into());
    }
    dlog(
        source_name,
        &format!("装配段完成（+{}ms）——进入帧循环", t0.ms()),
    );

    // ── 帧驱动循环（与 testhost::run 同构）：事件泵 → 协程 send 步进
    // → StopIteration 正常收尾 → 帧拍。首帧打点 = 启动诊断仪表。
    let mut first_frame = true;
    let mut frame_n: u64 = 0;
    // io 等待旗标**帧间持久**（同 run 循环——循环内声明 = 漏闸挂死）
    let mut io_wait = false;
    loop {
        let seg_pump = starfish_std::debug::Stopwatch::now();
        if starfish_pygame::event::pump() || starfish_std::run_control::stop_requested() {
            if starfish_std::run_control::stop_requested() {
                starfish_debug::log_warn!("[host] 嵌入式停止信号命中——帧循环收口");
            }
            dlog("pygamest-dev", "退出请求，结束");
            break;
        }
        frame_n += 1;
        if frame_n % 120 == 0 {
            dlog(source_name, &format!("帧拍 x{frame_n}"));
        }
        let ms_pump = seg_pump.ms();
        let seg_send = starfish_std::debug::Stopwatch::now();
        let mut done = false;
        interp.enter(|vm| {
            let borrowed = gen_cell.borrow();
            let Some(genobj) = borrowed.as_ref() else {
                done = true;
                return;
            };
            // 协程步进（共享步进器）：send 恢复——await 帧桥挂起 = 本帧完成；
            // await io fetch = 两拍桥（native 即取 / web spawn 后续帧恢复）
            match step_coro(
                "pygamest-dev",
                genobj,
                vm,
                &mut io_wait,
                &errored,
            ) {
                StepOutcome::Continue | StepOutcome::WaitIo => {}
                StepOutcome::Done => done = true,
            }
        });
        if done {
            break;
        }
        starfish_pygame::mixer::pump(); // Web 流式解码驱动
        next_frame().await;
        if first_frame {
            first_frame = false;
            dlog(
                "pygamest-dev",
                &format!(
                    "首帧分段: pump={}ms send+收尾={}ms（累计 +{}ms）",
                    ms_pump,
                    seg_send.ms(),
                    t0.ms()
                ),
            );
        }
    }
    // 无协程（纯脚本形态）：顶层执行完即收（python 语义）
    // 先关窗再收口（与 run 同约——RP 活动期间窗口 ghost 防御）
    display::quit();
    starfish_pygame::mixer::quit();
    if errored.get() {
        return Err("脚本异常（traceback 见上）".into());
    }
    Ok(())
}

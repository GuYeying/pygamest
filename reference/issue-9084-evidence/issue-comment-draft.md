<!-- 复制以下全部内容贴到 https://github.com/RustPython/RustPython/issues/9084 -->
<!-- ⟨...⟩ 是占位符：E3 的数字等 IR 构建统计结果填入，E4 的数字等真机跑 keys_probe 填入 -->

Following up with measured data so the report can be verified independently — including a correction to my own numbers (§4). Sections 1–3 need no Android device; any Linux box with `rustup target add aarch64-linux-android` reproduces them.

## 1. On bionic, std gives *every* `thread_local!` static a pthread key — desktop targets don't

I compiled three probe statics covering the shapes rustpython uses (const-init without `Drop`, const-init with `Drop`, lazy init), rustc 1.97.1, `--emit=llvm-ir`:

| static shape | `--target aarch64-linux-android` | `--target x86_64-unknown-linux-gnu` |
|---|---|---|
| const-init, no Drop | `LazyKey` + `pthread_setspecific` | `llvm.threadlocal.address` (no OS key) |
| const-init, with Drop | `LazyKey` + `pthread_setspecific` | `__cxa_thread_atexit_impl` (no key) |
| lazy init | `LazyKey` + `pthread_setspecific` | `llvm.threadlocal.address` (no key) |

The android IR has zero `llvm.threadlocal` globals and zero emutls: all TLS storage lives in pthread keys created lazily by `sys/thread_local/key/racy.rs::LazyKey::lazy_init`. `fatal runtime error: out of TLS keys` is `rtabort!` in `library/std/src/sys/thread_local/key/unix.rs`, reached when `pthread_key_create` fails at `PTHREAD_KEYS_MAX` (1024 on bionic, process-wide, shared with every other component in the process).

So the identical interpreter code that costs zero OS keys on desktop costs one process-wide pthread key per static on bionic. CI never sees this because the `aarch64-linux-android` job builds but never runs genesis.

<details><summary>Probe source and commands (click)</summary>

```rust
// probe_a.rs — const-init, no Drop
pub fn f() -> u32 { thread_local! { static X: u32 = const { 0u32 }; } X.with(|v| *v) }

// probe_b.rs — const-init, with Drop
pub struct D; impl Drop for D { fn drop(&mut self) {} }
pub fn f() -> bool { thread_local! { static X: Option<D> = const { None }; } X.with(|v| v.is_some()) }

// probe_c.rs — lazy init
pub fn f() -> usize { thread_local! { static X: String = String::new(); } X.with(|v| v.len()) }
```

```bash
rustc --target aarch64-linux-android --crate-type=lib --emit=llvm-ir probe_a.rs -o a.ll
grep -c "LazyKey" a.ll                # 4
grep -c "pthread_setspecific" a.ll    # 4
grep -c "llvm.threadlocal" a.ll       # 0
# same for b, c; then compile the same sources for x86_64-unknown-linux-gnu:
grep -cE "llvm.threadlocal|linux_like.*register" l.ll   # native TLS + cxa_thread_atexit, no pthread keys
```

</details>

## 2. What the real rustpython-vm requests on bionic — measured on the compiled IR

`rustpython-vm` @ 23fcb8d9, `default-features = false`, features `compiler, wasmbind, gc, stdio, importlib, encodings, freeze-stdlib` (the embedding feature set from my report), built with `--emit=llvm-ir` (230 crate IR files in the dependency graph):

- **aarch64-linux-android: 0 `llvm.threadlocal` globals in the entire build; 132 `pthread_setspecific` call sites across 18 crates; 243 `LazyKey` references** — every TLS static's storage is a lazily-created pthread key
- **x86_64-pc-windows-msvc (same code, same lockfile): 0 `LazyKey`, 0 `pthread_setspecific`, 3174 `llvm.threadlocal` references — native TLS, zero OS keys**

Cross-checking the 35 source-level static names against the android IR: **22 are present as named TLS globals** (e.g. `INT_FREELIST023___RUST_STD_INTERNAL_VAL`, `DICT_FREELIST…`, `codecs REGISTRY…`); the other 14 are absent exactly as cfg predicts (the `threading`-gated ones: `vm/thread.rs` 7× incl. `INTERP_THREAD_SLOTS`/`CURRENT_THREAD_SLOT`/`CURRENT_STOP_REQUESTED`, `common/lock/detaching.rs` 2×, windows-only signal static, plus dead-code-eliminated ones). The 22 compiled-in statics are touched during genesis (type creation allocates through the freelists; importlib bootstrap initializes codecs), i.e. ~22 pthread key requests happen before the first Python statement runs.

<details><summary>Commands (click)</summary>

```toml
# Cargo.toml
[dependencies]
rustpython-vm = { path = "<checkout>/crates/vm", default-features = false,
    features = ["compiler", "wasmbind", "gc", "stdio", "importlib", "encodings", "freeze-stdlib"] }
```

```bash
export CC_aarch64_linux_android=<ndk>/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android26-clang
RUSTFLAGS="--emit=llvm-ir -C debuginfo=0" cargo build --release --target aarch64-linux-android
grep -c  "llvm.threadlocal"          target/aarch64-linux-android/release/deps/*.ll | awk -F: '{s+=$2} END{print s}'   # 0
grep -oE "call.*pthread_setspecific" target/aarch64-linux-android/release/deps/*.ll | wc -l                            # 132
grep -o  "LazyKey"                   target/aarch64-linux-android/release/deps/*.ll | wc -l                            # 243
# per-static presence (22 of 35 source names):
grep -l "INT_FREELIST0" target/aarch64-linux-android/release/deps/rustpython_vm-*.ll
```

</details>

## 3. Source census: 37 TLS statics in vm + common @ 23fcb8d9 (35 unique names, 22 compiled in)

```
vm:       32 statics across 24 files (vm/thread.rs: 7 [threading-gated], stdlib/sys/monitoring.rs: 2,
          one each in builtins/{int,float,dict,list,tuple,range,slice,complex,type}.rs freelist cells,
          codecs.rs, exception_group.rs, gc_state.rs, frame.rs, signal.rs, stdlib/_io.rs,
          stdlib/time.rs, stdlib/_imp.rs, stdlib/_interpqueues.rs, stdlib/_thread.rs,
          buffer.rs, object/core.rs, vm/context.rs, vm/runtime.rs, stdlib/_codecs.rs (2))
common:    5 statics (lock/detaching.rs: 2 [threading-gated], refcount.rs: 2)
```

Census script (brace-aware, counts `static NAME:` inside `thread_local!`/`static_cell!` blocks):

```bash
find crates/vm/src crates/common/src -name '*.rs' -exec awk '
  /thread_local!|static_cell!/ && !inblock { inblock=1; depth=0; armed=0; file=FILENAME }
  inblock { cnt=gsub(/{/,"{")-gsub(/}/,"}"); depth+=cnt; if(cnt>0)armed=1;
            if(armed && /static [A-Za-z_][A-Za-z_0-9]*:/) n++;
            if(armed && depth<=0){ print file": "n; inblock=0 } }' {} \;
```

## 4. Correction to my original report — and why the smaller number still crashes

My original report said `static_cell!` creates "hundreds" of TLS statics. **That was wrong.** The census above gives **37** in vm + common. I likely over-counted by including per-class `static_cell()` trait impls, which are not TLS statics in this snapshot.

The scale matters, and it actually sharpens the diagnosis: 37 key requests do not exhaust a clean 1024-key pool — which is consistent with the crash only showing up in embedded hosts whose process has *already* consumed most of the pool. Measured on my device with a minimal probe (1024 rustpython-shaped const-init statics + a free-key census, no rustpython involved):

- free pthread keys at process start: **⟨E4a⟩** of 1024
- abort threshold: **touching ⟨E4b⟩ statics** reproduces `fatal runtime error: out of TLS keys`

⟨一两句描述真机环境：aarch64 真机，HarmonyOS NEXT 上的 Android 兼容层（卓易通），嵌入式宿主进程⟩

So the interpreter's 37 key requests are the last straw, not the whole load — and `StaticCell`'s one-thread-local-per-builtin design is what makes genesis sensitive to key-constrained hosts at all. A key-efficient StaticCell (one TLS slot holding a registry of cells) would make genesis robust on any such host, thread-safe included; my global-cell workaround from the report is only safe under a single-threaded embedding contract, which is why I did not PR it.

## 5. Minimal repro (no pygamest needed)

```toml
# Cargo.toml — build as cdylib, load from any Android app, or run as bin via adb shell
[dependencies]
rustpython-vm = { git = "https://github.com/RustPython/RustPython", default-features = false,
    features = ["compiler", "wasmbind", "gc", "stdio", "importlib", "encodings", "freeze-stdlib"] }
```

```rust
use rustpython_vm::{Interpreter, Settings};
use rustpython_vm::compiler::Mode;

#[no_mangle]
pub extern "C" fn Java_demo_probe_MainActivity_run(_e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void) -> i32 {
    // first interpreter genesis on the process's main thread
    let interp = Interpreter::builder(Settings::default()).build();
    interp.enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        let code = vm.compile("print(1+1)", Mode::Exec, "probe.py".to_owned()).unwrap();
        vm.run_code_obj(code, scope).unwrap();
    });
    0
}
```

Expected on an affected host: abort with `fatal runtime error: out of TLS keys` during genesis (`build()` / first `enter`), before any real Python code runs. On a clean emulator with a fresh key pool this may not abort — see §4.

Happy to test any candidate fix on the affected device.

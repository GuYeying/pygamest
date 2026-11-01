//! pygame.io 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use crate::rpy::pygame::pygame::{PySurface, PySurfaceSession, PyScreen, PyScreenSession, PyColor};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    function::{FuncArgs, PyComparisonValue},
    types::{Comparable, Constructor, PyComparisonOp, Representable},
    builtins::{PyModule, PyStr, PyStrRef, PyType},
};
use std::cell::Cell;
#[pymodule]
    pub mod io {
        use rustpython_vm::convert::TryFromObject;
        use rustpython_vm::{PyObjectRef, PyResult, VirtualMachine, builtins::PyStrRef};

        fn path_str<'a>(p: &'a PyStrRef, vm: &VirtualMachine) -> PyResult<&'a str> {
            p.to_str().ok_or_else(|| vm.new_value_error("路径含非 UTF-8 字符"))
        }

        /// init_crypto(key_hex, magic)：资源加密 init（创建时一次性——
        /// runtime 宿主运行前调用）。key_hex = 64 位 hex（32B 解密密钥）；
        /// magic = 项目自定义封装魔数（5 字符，缺省 SFXE1）。
        #[pyfunction]
        fn init_crypto(key_hex: String, magic: String, vm: &VirtualMachine) -> PyResult<()> {
            if key_hex.len() != 64 {
                return Err(vm.new_value_error("key_hex 须为 64 位 hex（32 字节）"));
            }
            let mut key = [0u8; 32];
            for (i, b) in key_hex.as_bytes().chunks(2).enumerate() {
                let hexs = std::str::from_utf8(b)
                    .map_err(|e| vm.new_value_error(format!("key hex: {e}")))?;
                key[i] = u8::from_str_radix(hexs, 16)
                    .map_err(|e| vm.new_value_error(format!("key hex: {e}")))?;
            }
            let mut m = [b'S'; 5];
            let msrc = magic.as_bytes();
            if msrc.len() == 5 {
                m.copy_from_slice(msrc);
            }
            starfish_io::init_crypto(key, m);
            Ok(())
        }

        /// init_crypto_embedded()：宿主嵌入钥接线（build.rs 编译期嵌入的
        /// ASSET_KEY_HEX/ASSET_MAGIC → io init）。运行态宿主启动自动调用；
        /// 未嵌入（dev 明文形态）= no-op。
        #[pyfunction]
        fn init_crypto_embedded(vm: &VirtualMachine) -> PyResult<()> {
            match (
                std::option_env!("ASSET_KEY_HEX"),
                std::option_env!("ASSET_MAGIC"),
            ) {
                (Some(hex), magic) => {
                    if hex.len() != 64 {
                        return Err(vm.new_value_error("嵌入密钥须为 64 位 hex"));
                    }
                    let mut key = [0u8; 32];
                    for (i, b) in hex.as_bytes().chunks(2).enumerate() {
                        let hexs = std::str::from_utf8(b)
                            .map_err(|e| vm.new_value_error(format!("嵌入 key hex: {e}")))?;
                        key[i] = u8::from_str_radix(hexs, 16)
                            .map_err(|e| vm.new_value_error(format!("嵌入 key hex: {e}")))?;
                    }
                    let mut m = [b'S'; 5];
                    if let Some(magic) = magic {
                        let src = magic.as_bytes();
                        if src.len() == 5 {
                            m.copy_from_slice(src);
                        }
                    }
                    starfish_io::init_crypto(key, m);
                }
                _ => {} // 未嵌入 = dev 明文形态（io 透传）
            }
            Ok(())
        }

        /// export_game(runtime_exe, staging_dir, out_exe)：单文件导出——
        /// 遍历 staging（明文脚本+资源）→ SFXE1 封装（io 密钥态）→ SFIX1
        /// 组包 → 追加到 runtime 壳 exe 尾部 + trailer。
        #[pyfunction]
        fn export_game(
            runtime_exe: String,
            staging_dir: String,
            out_exe: String,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use chacha20poly1305::{
                aead::{Aead, KeyInit, Payload},
                ChaCha20Poly1305, Key, Nonce,
            };
            let Some(key) = starfish_io::io_key() else {
                return Err(vm.new_value_error("init_crypto 未调用——导出前须注入密钥"));
            };
            let magic = starfish_io::io_magic();
            fn derive_nonce(name: &str, len: usize) -> [u8; 12] {
                let mut n = [0u8; 12];
                let seed = format!("{name}:{len}");
                for (i, b) in seed.bytes().enumerate() {
                    n[i % 12] = n[i % 12].wrapping_add(b).rotate_left(1);
                }
                n
            }
            fn walk(
                dir: &std::path::Path,
                prefix: &str,
                key: &[u8; 32],
                magic: &[u8; 5],
                out: &mut Vec<(String, Vec<u8>)>,
            ) -> Result<(), String> {
                if let Ok(rd) = std::fs::read_dir(dir) {
                    let mut items: Vec<_> = rd.filter_map(|e| e.ok()).collect();
                    items.sort_by_key(|e| e.file_name());
                    for entry in items {
                        let p = entry.path();
                        let name = entry.file_name().to_string_lossy().to_string();
                        let key_name = if prefix.is_empty() {
                            name.clone()
                        } else {
                            format!("{prefix}/{name}")
                        };
                        if p.is_dir() {
                            walk(&p, &key_name, key, magic, out)?;
                        } else if let Ok(data) = std::fs::read(&p) {
                            let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
                            let nonce = derive_nonce(&key_name, data.len());
                            let ct = cipher
                                .encrypt(&Nonce::from_slice(&nonce), Payload {
                                    msg: &data,
                                    aad: b"starfish-asset-v1",
                                })
                                .map_err(|e| format!("封装失败 {key_name}: {e:?}"))?;
                            let mut env = magic.to_vec();
                            env.extend_from_slice(&nonce);
                            env.extend_from_slice(&ct);
                            out.push((key_name, env));
                        }
                    }
                }
                Ok(())
            }
            let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
            let root = std::path::Path::new(&staging_dir);
            walk(root, "", &key, &magic, &mut entries)
                .map_err(|e| vm.new_value_error(format!("遍历封装失败: {e}")))?;
            if entries.is_empty() {
                return Err(vm.new_value_error("staging 目录为空"));
            }
            let mut out = std::fs::read(&runtime_exe)
                .map_err(|e| vm.new_value_error(format!("runtime 壳读取失败: {e}")))?;
            let mut blob = Vec::new();
            let mut index_entries: Vec<(String, u64, u64)> = Vec::new();
            for (name, env) in &entries {
                index_entries.push((name.clone(), 41 + blob.len() as u64, env.len() as u64));
                blob.extend_from_slice(env);
            }
            let index_json = {
                let parts: Vec<String> = index_entries
                    .iter()
                    .map(|(p, o, l)| format!("{{\"p\":\"{p}\",\"o\":{o},\"l\":{l}}}"))
                    .collect();
                format!("{{\"entries\":[{}]}}", parts.join(","))
            };
            let index_nonce: [u8; 12] = *b"IDXNONCE0112";
            let index_ct = {
                let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
                cipher
                    .encrypt(
                        &Nonce::from_slice(&index_nonce),
                        chacha20poly1305::aead::Payload {
                            msg: index_json.as_bytes(),
                            aad: b"sf-index",
                        },
                    )
                    .map_err(|e| vm.new_value_error(format!("索引加密失败: {e:?}")))?
            };
            let mut pack = b"SFIX1".to_vec();
            pack.extend_from_slice(&1u32.to_le_bytes());
            pack.extend_from_slice(&1u32.to_le_bytes());
            pack.extend_from_slice(&index_nonce);
            pack.extend_from_slice(&(41 + blob.len() as u64).to_le_bytes());
            pack.extend_from_slice(&(index_ct.len() as u64).to_le_bytes());
            pack.extend_from_slice(&blob);
            pack.extend_from_slice(&index_ct);
            let pack_len = pack.len() as u64;
            let mut out = std::fs::read(&runtime_exe)
                .map_err(|e| vm.new_value_error(format!("runtime 壳读取失败: {e}")))?;
            out.extend_from_slice(&pack);
            out.extend_from_slice(&pack_len.to_le_bytes());
            out.extend_from_slice(b"SFBUNDL1");
            std::fs::write(&out_exe, &out)
                .map_err(|e| vm.new_value_error(format!("导出写入失败: {e}")))?;
            starfish_debug::log_info!(format!(
                "[export] 单文件导出完成: {out_exe}（{}B，{} 条目）",
                out.len(), entries.len()
            ));
            Ok(())
        }

        /// read(path) -> bytes
        #[pyfunction]
        fn read(path: PyStrRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            #[cfg(target_arch = "wasm32")]
            return Err(vm.new_value_error(
                "pygame.io: Web 端 v1 不提供同步 io（宿主预加载/异步资源装载承载）",
            ));
            #[cfg(not(target_arch = "wasm32"))]
            {
                let data = pollster::block_on(starfish_io::read(path_str(&path, vm)?))
                    .map_err(|e| vm.new_value_error(format!("io 读取失败: {e}")))?;
                Ok(vm.ctx.new_bytes(data).into())
            }
        }

        /// read_text(path) -> str（UTF-8，非法序列替换）
        #[pyfunction]
        fn read_text(path: PyStrRef, vm: &VirtualMachine) -> PyResult<String> {
            #[cfg(target_arch = "wasm32")]
            return Err(vm.new_value_error(
                "pygame.io: Web 端 v1 不提供同步 io（宿主预加载/异步资源装载承载）",
            ));
            #[cfg(not(target_arch = "wasm32"))]
            {
                let data = pollster::block_on(starfish_io::read(path_str(&path, vm)?))
                    .map_err(|e| vm.new_value_error(format!("io 读取失败: {e}")))?;
                Ok(String::from_utf8_lossy(&data).into_owned())
            }
        }

        /// write(path, data)：覆盖写（bytes 或 str——str 按 UTF-8 编码）
        #[pyfunction]
        fn write(path: PyStrRef, data: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            #[cfg(target_arch = "wasm32")]
            return Err(vm.new_value_error(
                "pygame.io: Web 端 v1 不提供同步 io（宿主预加载/异步资源装载承载）",
            ));
            #[cfg(not(target_arch = "wasm32"))]
            {
                let bytes: Vec<u8> = if let Some(s) = data.downcast_ref::<rustpython_vm::builtins::PyStr>()
                {
                    s.to_str().ok_or_else(|| vm.new_value_error("内容含非 UTF-8 字符"))?.as_bytes().to_vec()
                } else {
                    data.try_bytes_like(vm, |b| b.to_vec())?
                };
                pollster::block_on(starfish_io::write(path_str(&path, vm)?, bytes))
                    .map_err(|e| vm.new_value_error(format!("io 写入失败: {e}")))
            }
        }

        /// exists(path) -> bool
        #[pyfunction]
        fn exists(path: PyStrRef, vm: &VirtualMachine) -> PyResult<bool> {
            #[cfg(target_arch = "wasm32")]
            return Err(vm.new_value_error(
                "pygame.io: Web 端 v1 不提供同步 io（宿主预加载/异步资源装载承载）",
            ));
            #[cfg(not(target_arch = "wasm32"))]
            pollster::block_on(starfish_io::exists(path_str(&path, vm)?))
                .map_err(|e| vm.new_value_error(format!("io exists 失败: {e}")))
        }

        /// write_text(path, text)：覆盖写文本（UTF-8）
        #[pyfunction]
        fn write_text(path: PyStrRef, text: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
            #[cfg(target_arch = "wasm32")]
            return Err(vm.new_value_error(
                "pygame.io: Web 端 v1 不提供同步 io（宿主预加载/异步资源装载承载）",
            ));
            #[cfg(not(target_arch = "wasm32"))]
            {
                let t = text
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("内容含非 UTF-8 字符"))?;
                pollster::block_on(starfish_io::write_text(path_str(&path, vm)?, t))
                    .map_err(|e| vm.new_value_error(format!("io 写入失败: {e}")))
            }
        }
        // set_base_dir 不暴露（2026-10-26 定约）：基准目录 = 引擎内部
        // 语义（Android 自动注入），Python 面恒以 main 所在目录为基准
        // 写相对路径——打包形态不改写调用写法
    }

    // ── net：网络（批次 M；轮询式语义——帧内 send/recv，主线程永不阻塞）──
    // TCP = 消息分帧连接（4 字节大端长度前缀，单帧上限 16 MB；原生后台
    // 线程 + std 阻塞流，Web 自动 WebSocket——三平台同一轮询面）。

pub(crate) use io::module_def;

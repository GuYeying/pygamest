//! pygame.codec 绑定（**stdlite 定约 2026-11-24**：编解码层——
//! starfish-codec 直通：hash（sha256）/ zlib（flate2）/ base64）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use starfish_codec as sc;
use rustpython_vm::{PyObjectRef, PyResult, VirtualMachine};

#[pymodule]
    pub mod codec {
        use rustpython_derive::pyclass;
        use rustpython_vm::builtins::PyStrRef;
        use super::*;

        /// sha256_bytes(data) -> bytes：字节摘要（32B）
        #[pyfunction]
        fn sha256_bytes(data: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let data: Vec<u8> = data.try_into_value(vm)?;
            let out = sc::hash::sha256_bytes(&data);
            let items: Vec<PyObjectRef> = out
                .into_iter()
                .map(|b| vm.ctx.new_int(b as i64).into())
                .collect();
            Ok(vm.ctx.new_list(items).into())
        }

        /// sha256_hex(data) -> str：64 字符小写 hex 摘要
        #[pyfunction]
        fn sha256_hex(data: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
            let data: Vec<u8> = data.try_into_value(vm)?;
            Ok(sc::hash::sha256_hex(&data))
        }

        /// zlib_compress(data) -> bytes：zlib 压缩
        #[pyfunction]
        fn zlib_compress(data: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let data: Vec<u8> = data.try_into_value(vm)?;
            let out = sc::zlib::compress(&data);
            let items: Vec<PyObjectRef> = out
                .into_iter()
                .map(|b| vm.ctx.new_int(b as i64).into())
                .collect();
            Ok(vm.ctx.new_list(items).into())
        }

        /// zlib_decompress(data) -> bytes：zlib 解压（损坏 = 空）
        #[pyfunction]
        fn zlib_decompress(data: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let data: Vec<u8> = data.try_into_value(vm)?;
            let out = sc::zlib::decompress(&data);
            let items: Vec<PyObjectRef> = out
                .into_iter()
                .map(|b| vm.ctx.new_int(b as i64).into())
                .collect();
            Ok(vm.ctx.new_list(items).into())
        }

        /// b64_encode(data) -> str：字节 → base64 文本
        #[pyfunction]
        fn b64_encode(data: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
            let data: Vec<u8> = data.try_into_value(vm)?;
            Ok(sc::base64::encode(&data))
        }

        /// b64_decode(text) -> bytes：base64 文本 → 字节（非法 = ValueError）
        #[pyfunction]
        fn b64_decode(text: PyStrRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let text = text.to_str().ok_or_else(|| vm.new_value_error("base64 含非 UTF-8"))?;
            let out = sc::base64::decode(text).map_err(|e| vm.new_value_error(e))?;
            let items: Vec<PyObjectRef> = out
                .into_iter()
                .map(|b| vm.ctx.new_int(b as i64).into())
                .collect();
            Ok(vm.ctx.new_list(items).into())
        }
    }

pub(crate) use codec::module_def;

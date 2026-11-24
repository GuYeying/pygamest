//! pygame.parse 绑定（**stdlite 定约 2026-11-24**：结构化数据解析层
//! ——starfish-parse 直通；界限定约见该 crate：数据序列化格式收、
//! 资产格式不收）。
//!
//! py 值模型：dict / list / str / int / float / bool / None（双格式
//! json/toml 同构 serde_json::Value——绑定层零分派成本）。
//!
//! 转换定约：**TryFromObject 惯例**（bool/i64/f64/String 直试取 +
//! PyDict/PyList downcast——touch.rs 的 ctx.new_dict + set_item 组装
//! 先例；PyStrRef.to_str = Option 形态 ok_or_else 惯例）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use starfish_parse as sp;
use rustpython_vm::{
    PyObjectRef, Py, PyResult, VirtualMachine,
    builtins::{PyBool, PyDict, PyList, PyNone},
    convert::TryFromObject,
};

/// serde_json::Value → py 值树（dict/list/str/num/bool/None）
fn value_to_py(v: &sp::serde_json::Value, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    use sp::serde_json::Value;
    Ok(match v {
        Value::Null => vm.ctx.none().into(),
        Value::Bool(b) => vm.ctx.new_bool(*b).into(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                vm.ctx.new_int(i).into()
            } else if let Some(f) = n.as_f64() {
                vm.ctx.new_float(f).into()
            } else {
                vm.ctx.none().into()
            }
        }
        Value::String(s) => vm.ctx.new_str(s.clone()).into(),
        Value::Array(a) => {
            let items: Result<Vec<PyObjectRef>, _> =
                a.iter().map(|x| value_to_py(x, vm)).collect();
            vm.ctx.new_list(items?).into()
        }
        Value::Object(o) => {
            let d = vm.ctx.new_dict();
            for (k, val) in o {
                d.set_item(k.as_str(), value_to_py(val, vm)?, vm)?;
            }
            d.into()
        }
    })
}

/// py 值树 → serde_json::Value（仅 dict/list/str/int/float/bool/None
/// ——其他类型 = Err 带类型名）
fn py_to_value(obj: &PyObjectRef, vm: &VirtualMachine) -> Result<sp::serde_json::Value, String> {
    use sp::serde_json::Value;
    if obj.downcast_ref::<PyNone>().is_some() {
        return Ok(Value::Null);
    }
    if obj.downcast_ref::<PyBool>().is_some() {
        let b = bool::try_from_object(vm, obj.clone())
            .map_err(|e| crate::rpy::bridge::exc_msg(vm, e))?;
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = i64::try_from_object(vm, obj.clone()) {
        return Ok(Value::Number(i.into()));
    }
    if let Ok(f) = f64::try_from_object(vm, obj.clone()) {
        return match sp::serde_json::Number::from_f64(f) {
            Some(n) => Ok(Value::Number(n)),
            None => Err("float 非有限（json 不支持 NaN/inf）".into()),
        };
    }
    if let Ok(s) = String::try_from_object(vm, obj.clone()) {
        return Ok(Value::String(s));
    }
    if let Some(l) = obj.downcast_ref::<PyList>() {
        let arr: Result<Vec<Value>, String> = l
            .borrow_vec()
            .iter()
            .map(|x| py_to_value(x, vm))
            .collect();
        return Ok(Value::Array(arr?));
    }
    if let Some(d) = obj.downcast_ref::<PyDict>() {
        let mut map = sp::serde_json::Map::new();
        for (k, v) in d.items_vec() {
            let key = String::try_from_object(vm, k.clone())
                .map_err(|_| "dict 键须为 str".to_string())?;
            map.insert(key, py_to_value(&v, vm)?);
        }
        return Ok(Value::Object(map));
    }
    Err(format!(
        "仅支持 dict/list/str/int/float/bool/None，实际 {name:?}",
        name = obj.class().name()
    ))
}

#[pymodule]
    pub mod parse {
        use rustpython_derive::pyclass;
        // 内嵌模块继承外层全部导入（helper/类型/sp 别名——外层 use 集中）
        use super::*;

        /// loads(text) -> dict/list/...：json 文本 → py 值树
        #[pyfunction]
        fn loads(text: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let s = String::try_from_object(vm, text).map_err(|e| vm.new_value_error(bridge::exc_msg(vm, e)))?;
            let v = sp::json::parse(&s).map_err(|e| vm.new_value_error(e))?;
            value_to_py(&v, vm)
        }

        /// dumps(obj) -> str：py 值树 → json 文本（紧凑）
        #[pyfunction]
        fn dumps(obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
            let v = py_to_value(&obj, vm).map_err(|e| vm.new_value_error(e))?;
            Ok(sp::json::dump(&v))
        }

        /// dump_pretty(obj) -> str：py 值树 → json 文本（缩进美化）
        #[pyfunction]
        fn dump_pretty(obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
            let v = py_to_value(&obj, vm).map_err(|e| vm.new_value_error(e))?;
            Ok(sp::json::dump_pretty(&v))
        }

        /// loads_toml(text) -> dict/list/...：toml 文本 → py 值树
        #[pyfunction]
        fn loads_toml(text: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let s = String::try_from_object(vm, text).map_err(|e| vm.new_value_error(crate::rpy::bridge::exc_msg(vm, e)))?;
            let v = sp::toml::parse(&s).map_err(|e| vm.new_value_error(e))?;
            value_to_py(&v, vm)
        }

        /// dumps_toml(obj) -> str：py 值树 → toml 文本
        #[pyfunction]
        fn dumps_toml(obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<String> {
            let v = py_to_value(&obj, vm).map_err(|e| vm.new_value_error(e))?;
            Ok(sp::toml::dump(&v))
        }
    }

pub(crate) use parse::module_def;

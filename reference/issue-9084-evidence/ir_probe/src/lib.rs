// Touch the interpreter type surface so all statics are compiled in.
pub fn peek() -> usize {
    std::mem::size_of::<rustpython_vm::VirtualMachine>()
}

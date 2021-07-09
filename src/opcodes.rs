/// Native operations that are defined directly in the VM.
/// A scape-hatch for some low level operations.
#[derive(Debug, Clone)]
pub enum Native {
    Print,
    ToString,
    Length,
    Append,
    DumpStack,
}

/// List of opcodes supported by the VM
#[derive(Debug, Clone)]
pub enum Op {
    /// Jump target
    /// **Must not be present on actual final compiled code**
    Target(usize),

    /// No OPeration
    Nop,

    /// Call a native operation
    /// (#-of-args, which-call)
    Native(usize, Native),

    /// Push Integer
    PushI(i64),
    /// Push String
    PushS(String),
    /// Make top (value) elements from stack into a Value::List
    MakeList(usize),

    /// Sub-indexing (a b -- a[b])
    Index,

    /// Sub-indexed store (a b c -- c[b] = a)
    IndexStore,

    /// TODO: function call

    /// Duplicate (top - value)
    Dup(usize),
    /// Pop (discard) top
    Pop,

    /// Load a global
    LoadG(String),
    /// Store into a global (keep on stack)
    StoreG(String),
    /// Move into a global (pop stack)
    MoveG(String),

    Lt, Lte,
    Gt, Gte,
    Eq, Neq,

    /// Jump if top stack if false
    JmpF(usize),

    /// Unconditional jump
    Jmp(usize),

    Add, Sub,
    Mul, Div, Mod,
}

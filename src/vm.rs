use std::collections::HashMap;
use crate::{
    value::Value,
    opcodes::{Op, Native},
    errors::Error,
};

/// Result of a operation on the VM
pub type Result<T> = std::result::Result<T, Error>;

/// A pointer into the managed heap
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct HeapPtr(usize);

// TODO: HeapPtr can also store 63-bit integers and tagged pointers (assuming usize is 64-bit...)

/// Script execution Virtual Machine
#[derive(Debug)]
pub struct VM {
    /// GC'ed heap.
    /// A position is None if previously allocated, but released during a collection
    heap: Vec<Option<Value>>,

    /// Value stack.
    /// Stack only store pointers into heap (all values are boxed -- even basic integers)
    stack: Vec<HeapPtr>,

    /// Top-level (globals) indexed by name
    top: HashMap<String, HeapPtr>,

    /// List of free heap entries during last collection
    free_list: Vec<usize>,
}

/*
    On a function call the stack looks like (starting at `fp`)
    - return value
    - arguments (already pushed by caller)
    - locals

    call_stack: Vec<usize>,     // empty when in root
    frame_ptr: Vec<usize>, // empty when in root

    Op::PrepareCall => {
        self.push_value(Value::Int(0));
        self.frame_ptr.push(self.stack.len());
    }

    Op::Call(address) => {
        // number of args explicit or implicit?
        self.call_stack.push(next_pc);
        next_pc = address;
    }

    Op::Return => {
        Op::StoreL(0);
        let fp = self.frame_ptr.pop().unwrap();
        while self.stack.len() != fp {
            self.stack.pop();
        }
        next_pc = self.call_stack.pop().unwrap();
    }

    Op::LoadL(index) => {
        let fp = *self.frame_ptr.front().unwrap();
        let ptr = self.stack[fp + index];
        self.stack.push(ptr);
    }

    Op::StoreL(index) => {
        let ptr = self.stack.pop().unwrap();
        let fp = *self.frame_ptr.front().unwrap();
        self.stack[fp + index] = ptr;
    }
*/

// TODO: review the public interface of VM

impl VM {
    /// Create a new empty heap.
    pub fn new() -> VM {
        VM {
            heap: vec![],
            stack: vec![],
            top: HashMap::new(),
            free_list: vec![],
        }
    }

    /// Garbage collection of heap
    pub fn collect(&mut self) {
        // the algorithm is a mark-and-sweep using stack and top as roots

        // mark phase uses an explicit stack to follow pointers
        let mut marked = vec![false; self.heap.len()];
        let mut roots = Vec::with_capacity(self.stack.len() + self.top.len());
        self.stack.iter().for_each(|ptr| roots.push(*ptr));
        self.top.values().for_each(|ptr| roots.push(*ptr));
        while let Some(ptr) = roots.pop() {
            if !marked[ptr.0] && self.heap[ptr.0].is_some() {
                marked[ptr.0] = true;
                self.heap[ptr.0].as_ref().unwrap().mark(&mut roots);
            }
        }

        // release a heap entry by setting it to None
        self.free_list.clear();
        for (i, node) in self.heap.iter_mut().enumerate() {
            if !marked[i] {
                self.free_list.push(i);
                *node = None;
            }
        }
    }

    /// Return a currently free slot.
    /// Slot is *not* marked as used!!!
    fn find_free_slot(&mut self) -> usize {
        // attempt to find free heap entry
        if let Some(i) = self.free_list.pop() {
            return i;
        }

        // not found; collect and try again
        self.collect();
        if let Some(i) = self.free_list.pop() {
            return i;
        }

        // if no free entry was found, attempt to grow heap
        // let runtime blow on not-enough-memory conditions :)
        let i = self.heap.len();
        self.heap.push(None);
        i
    }

    /// Store `value` into `self.heap` at `index`.
    fn store_heap(&mut self, index: usize, value: Value) {
        self.heap[index] = Some(value);
    }

    /// Directly push a `HeapPtr` into the stack
    pub fn push(&mut self, ptr: HeapPtr) {
        self.stack.push(ptr);
    }

    /// Allocate a slot for `value` on the heap, and push the result on the stack
    pub fn push_value(&mut self, value: Value) -> HeapPtr {
        let i = self.find_free_slot();
        self.store_heap(i, value);
        self.stack.push(HeapPtr(i));
        HeapPtr(i)
    }

    /// Return a reference to the value of `ptr` on the heap, or an error.
    pub fn get(&self, ptr: HeapPtr) -> Result<&Value> {
        // the first `ok_or` fails if `ptr` is out of range for self.heap
        // the second `ok_or` fails if the heap entry is `None`
        self.heap.get(ptr.0)
            .ok_or(Error::MemoryAccessOutOfRange(ptr))?
            .as_ref()
            .ok_or(Error::InvalidMemoryAccess(ptr))
    }

    /// Return a mutable reference to an entry on the heap
    pub fn get_mut(&mut self, ptr: HeapPtr) -> Result<&mut Value> {
        self.heap.get_mut(ptr.0)
            .ok_or(Error::MemoryAccessOutOfRange(ptr))?
            .as_mut()
            .ok_or(Error::InvalidMemoryAccess(ptr))
    }

    /// Return a clone of an entry on the heap
    pub fn get_clone(&self, ptr: HeapPtr) -> Result<Value> {
        self.get(ptr).map(|v| v.clone())
    }

    /// Return the value at stack[-i] or error
    fn dup(&self, i: usize) -> Result<HeapPtr> {
        if i >= self.stack.len() {
            Err(Error::StackUnderflow)
        } else {
            Ok(self.stack[self.stack.len() - i - 1])
        }
    }

    /// Return a reference to the heap value of the pointer at offset `-i` on the stack
    fn dup_value(&self, i: usize) -> Result<&Value> {
        let ptr = self.dup(i)?;
        self.get(ptr)
    }

    /// Return a *mutable* reference to the heap value of the pointer at offset `-i` on the stack
    fn dup_value_mut(&mut self, i: usize) -> Result<&mut Value> {
        let ptr = self.dup(i)?;
        self.get_mut(ptr)
    }

    /// Pop from stack or error
    fn pop(&mut self) -> Result<HeapPtr> {
        self.stack.pop().ok_or(Error::StackUnderflow)
    }

    /// Pop from stack and return a reference the Value in the Heap.
    fn pop_value(&mut self) -> Result<&Value> {
        let ptr = self.stack.pop().ok_or(Error::StackUnderflow)?;
        self.get(ptr)
    }

    /// Run `code` on the VM, keeping the current memory state from any previous execution (globals).
    pub fn run(&mut self, code: &[Op]) -> Result<()> {
        let mut pc = 0;
        while pc < code.len() {
            let mut next_pc = pc + 1;
            match code[pc].clone() {
                Op::Nop => {
                    // do nothing
                }
                Op::Target(_) => {
                    // jump targets must not happen in the output of compiler
                    return Err(Error::InvalidOpCode(pc));
                }
                Op::PushI(n) => {
                    self.push_value(Value::Int(n));
                }
                Op::PushS(s) => {
                    self.push_value(Value::Str(s.clone()));
                }
                Op::Dup(i) => {
                    self.stack.push(self.dup(i)?);
                }
                Op::Pop => {
                    self.pop()?;
                }
                Op::LoadG(s) => {
                    let ptr = *self.top.get(&s).ok_or_else(|| Error::GlobalNotFound(s.clone()))?;
                    self.stack.push(ptr);
                }
                Op::StoreG(s) => {
                    let ptr = self.dup(0)?;
                    self.top.insert(s.clone(), ptr);
                }
                Op::MoveG(s) => {
                    let ptr = self.pop()?;
                    self.top.insert(s.clone(), ptr);
                }
                Op::MakeList(n) => {
                    let i = self.find_free_slot();
                    let lst = self.stack.split_off(self.stack.len() - n);
                    self.store_heap(i, Value::List(lst));
                    self.stack.push(HeapPtr(i));
                }
                Op::JmpF(target) => {
                    if self.pop_value()?.is_false() {
                        next_pc = target;
                    }
                }
                Op::Jmp(target) => {
                    next_pc = target;
                }
                Op::Native(nargs, native_op) => {
                    // built-in functions handled directly in native code
                    let value = match native_op {
                        Native::Print => {
                            for i in 0 .. nargs {
                                print!("{}", self.dup_value(nargs - i - 1)?.fmt(self, 0)?);
                            }
                            println!();
                            Value::Int(nargs as i64)
                        }
                        Native::Length => {
                            let n = self.dup_value(0)?.length();
                            Value::Int(n as i64)
                        }
                        Native::ToString => {
                            let s = self.dup_value(0)?.fmt(self, 0)?;
                            Value::Str(s)
                        }
                        Native::Append => {
                            let mut to_add = vec![];
                            for i in 1 .. nargs {
                                let ptr = self.dup(nargs - i - 1)?;
                                to_add.push(ptr);
                            }

                            let target = self.dup_value_mut(nargs - 1)?;
                            match target {
                                Value::List(lst) => {
                                    lst.extend_from_slice(&to_add);
                                    Value::Int(lst.len() as i64)
                                }
                                _ => return Err(Error::InvalidAppend(target.clone())),
                            }
                        }
                        Native::DumpStack => {
                            if nargs > 0 {
                                print!("{} ", self.dup_value(0)?.fmt(self, 0)?);
                            } else {
                                print!("STACK> ");
                            }
                            println!("{:?}", self.stack);
                            Value::Int(self.stack.len() as i64)
                        }
                    };

                    // pop all arguments -- even unused ones!
                    for _ in 0 .. nargs {
                        self.pop()?;
                    }

                    // push single return value
                    self.push_value(value);
                }
                Op::Lt => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.cmp(self, b)? < 0;
                    self.push_value(Value::Int(if c { 1 } else { 0 }));
                }
                Op::Lte => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.cmp(self, b)? <= 0;
                    self.push_value(Value::Int(if c { 1 } else { 0 }));
                }
                Op::Gt => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.cmp(self, b)? > 0;
                    self.push_value(Value::Int(if c { 1 } else { 0 }));
                }
                Op::Gte => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.cmp(self, b)? >= 0;
                    self.push_value(Value::Int(if c { 1 } else { 0 }));
                }
                Op::Eq => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.cmp(self, b)? == 0;
                    self.push_value(Value::Int(if c { 1 } else { 0 }));
                }
                Op::Neq => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.cmp(self, b)? != 0;
                    self.push_value(Value::Int(if c { 1 } else { 0 }));
                }
                Op::Add => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.add(b)?;
                    self.push_value(c);
                }
                Op::Sub => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.sub(b)?;
                    self.push_value(c);
                }
                Op::Mul => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.mul(b)?;
                    self.push_value(c);
                }
                Op::Div => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.div(b)?;
                    self.push_value(c);
                }
                Op::Mod => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    let c = a.r#mod(b)?;
                    self.push_value(c);
                }
                Op::Index => {
                    let bptr = self.pop()?;
                    let aptr = self.pop()?;

                    let b = self.get(bptr)?;
                    let a = self.get(aptr)?;
                    match (a, b) {
                        (Value::Str(s), Value::Int(i)) => {
                            let ch = s.chars().nth(*i as usize).ok_or_else(|| Error::IndexOutOfRange(a.clone(), *i as usize))?;
                            self.push_value(Value::Int(ch as i64));
                        }
                        (Value::List(lst), Value::Int(i)) => {
                            let ptr = *lst.get(*i as usize).ok_or_else(|| Error::IndexOutOfRange(a.clone(), *i as usize))?;
                            self.push(ptr);
                        }
                        _ => {
                            return Err(Error::IncompatibleOperands(Op::Index, a.clone(), b.clone()))
                        }
                    }
                }
                Op::IndexStore => {
                    let cptr = self.pop()?;
                    let bptr = self.pop()?;
                    let aptr = self.dup(0)?;

                    let index = {
                        let b = self.get(bptr)?;
                        if let Value::Int(n) = b {
                            *n as usize
                        } else {
                            return Err(Error::IncompatibleOperands(Op::IndexStore, self.get(cptr)?.clone(), b.clone()))
                        }
                    };

                    let c = self.get_mut(cptr)?;
                    match c {
                        Value::List(lst) => {
                            match lst.get_mut(index) {
                                Some(p) => *p = aptr,
                                None => return Err(Error::IndexOutOfRange(c.clone(), index)),
                            }
                        }
                        _ => {
                            return Err(Error::IncompatibleOperands(Op::IndexStore, c.clone(), self.get(bptr)?.clone()))
                        }
                    }
                }
                // _ => {
                //     panic!("not supported: {:?}", code[pc]);
                // }
            }
            pc = next_pc;
        }
        Ok(())
    }
}

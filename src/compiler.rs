use std::collections::HashMap;

use crate::{
    ast::Ast,
    opcodes::{Op, Native},
    errors::{Error, Result},
    token::{Token, Kind},
};

/// The compiler is fed `Ast`'s from the `Parser` and, in the end, output a sequence of `Op` with
/// the instructions that shall be executed by the `VM`.
pub struct Compiler {
    /// Vector of opcodes generated during compilation
    code: Vec<Op>,

    /// List of local jump targets
    target_count: usize,

    /// Name of native function calls, handled directly by the VM
    /// map to (opcodes::Native, min-num-of-args)
    native_calls: HashMap<String, (Native, usize)>,
}

// TODO: scopes
// TODO: actual symbol tables w/ locals, globals, functions, etc...

impl Compiler {
    pub fn new() -> Compiler {
        let native_calls = {
            let mut h = HashMap::new();
            h.insert("print".to_string(), (Native::Print, 0));
            h.insert("length".to_string(), (Native::Length, 1));
            h.insert("to_string".to_string(), (Native::ToString, 1));
            h.insert("append".to_string(), (Native::Append, 2));
            h.insert("dump_stack".to_string(), (Native::DumpStack, 0));
            h
        };

        Compiler {
            code: vec![],
            target_count: 0,
            native_calls,
        }
    }

    /// Return the next jump target ID to use.
    fn next_target(&mut self) -> usize {
        let t = self.target_count;
        self.target_count += 1;
        t
    }

    /// Return the Op to use from a BinOp Kind
    fn op_from_tk(tk: &Token) -> Op {
        match tk.kind {
            Kind::Add => Op::Add,
            Kind::Sub => Op::Sub,
            Kind::Mul => Op::Mul,
            Kind::Div => Op::Div,
            Kind::Mod => Op::Mod,
            Kind::Lt  => Op::Lt,
            Kind::Lte => Op::Lte,
            Kind::Gt  => Op::Gt,
            Kind::Gte => Op::Gte,
            Kind::Eq  => Op::Eq,
            Kind::NotEq => Op::Neq,
            _ => panic!("compiler got invalid binary operator from parser {:?}", tk),
        }
    }

    /// Feed a new `ast` to the compiler.
    ///
    /// This alters the internal state of the compiler to account for new definitions, declarations, etc.
    /// For this to work OK, just feed top-level operations (in the order they are found in source).
    ///
    /// Return the number of opcodes that where written to the code.  Note that the length of the
    /// final `build` may not match the sum of the returns of all `feed`s!
    pub fn feed(&mut self, ast: &Ast) -> Result<usize> {
        let starting = self.code.len();
        match ast {
            Ast::Sttm(ast) => {
                self.feed(ast)?;
                self.code.push(Op::Pop);
            }
            Ast::Int(n, _) => {
                self.code.push(Op::PushI(*n));
            }
            Ast::Str(s, _) => {
                self.code.push(Op::PushS(s.clone()));
            }
            Ast::Lst(lst, _) => {
                for ast in lst.iter() {
                    self.feed(ast)?;
                }
                self.code.push(Op::MakeList(lst.len()));
            }
            Ast::Var(s, _) => {
                // TODO: lookup and check if global or local
                self.code.push(Op::LoadG(s.clone()));
            }
            Ast::BinOp(tk, lhs, rhs) if tk.kind == Kind::Assign => {
                match &**lhs {
                    Ast::Var(name, _) => {
                        // TODO: lookup and check if global or local
                        self.feed(rhs)?;
                        self.code.push(Op::StoreG(name.clone()));
                    }
                    Ast::Index(_, target, index) => {
                        self.feed(rhs)?;
                        self.feed(index)?;
                        self.feed(target)?;
                        self.code.push(Op::IndexStore);
                    }
                    _ => {
                        return Err(Error::InvalidAssignmentTarget(*lhs.clone()))
                    }
                }
            }
            Ast::BinOp(tk, lhs, rhs) => {
                self.feed(lhs)?;
                self.feed(rhs)?;
                self.code.push(Self::op_from_tk(tk));
            }
            Ast::Loop(_, st, cmp, body, up) => {
                if let Some(ast) = st {
                    self.feed(ast)?;
                }
                let loop_start = self.next_target();
                let loop_end = self.next_target();
                self.code.push(Op::Target(loop_start));
                if let Some(ast) = cmp {
                    self.feed(ast)?;
                    self.code.push(Op::JmpF(loop_end));
                }
                self.feed(body)?;
                if let Some(ast) = up {
                    self.feed(ast)?;
                }
                self.code.push(Op::Jmp(loop_start));
                self.code.push(Op::Target(loop_end));
            }
            Ast::IfElse(_, conditional, if_true, if_false) => {
                // target_end is after block, always present
                let target_end = self.next_target();

                // target_false is the target of the if_false,
                // only really present if an else block exists
                let target_false = if let Some(_) = if_false {
                    self.next_target()
                } else {
                    target_end
                };

                // code...
                self.feed(conditional)?;
                self.code.push(Op::JmpF(target_false));
                self.feed(if_true)?;
                if let Some(ast) = if_false {
                    self.code.push(Op::Jmp(target_end));
                    self.code.push(Op::Target(target_false));
                    self.feed(ast)?;
                }

                self.code.push(Op::Target(target_end));
            }
            Ast::Block(_, asts) => {
                for ast in asts {
                    self.feed(ast)?;
                }
            }
            Ast::Index(_, lhs, rhs) => {
                self.feed(lhs)?;
                self.feed(rhs)?;
                self.code.push(Op::Index);
            }
            Ast::Call(_, callee, args) => {
                match &**callee {
                    Ast::Var(name, _) if self.native_calls.contains_key(name) => {
                        let native = self.native_calls.get(name).unwrap();
                        if args.len() < native.1 {
                            return Err(Error::NotEnoughArguments(ast.clone(), name.clone(), args.len(), native.1));
                        }

                        let native = native.clone();
                        for arg in args {
                            self.feed(arg)?;
                        }

                        self.code.push(Op::Native(args.len(), native.0));
                    }
                    _ => {
                        panic!("general calls not implemented");
                    }
                }
            }
        }
        Ok(self.code.len() - starting)
    }

    /// Optimization steps
    fn optimize(&mut self) {
        // TODO: perhaps create a new Vec<Op> and move stuff over is better than in-place?

        let mut i = 0;
        while i < self.code.len() {
            // replace StoreG(x) || Pop by a single MoveG(x)
            if let Op::StoreG(name) = &self.code[i] {
                if i + 1 < self.code.len() && matches!(self.code[i + 1], Op::Pop) {
                    self.code[i] = Op::MoveG(name.clone());
                    self.code.remove(i + 1);
                }
            }
            i += 1;
        }
    }

    /// Replace all jumps to target ID's with actual addresses
    fn expand_targets(mut self) -> Result<Vec<Op>> {
        let mut target = vec![usize::MAX; self.target_count];

        // 1st pass -- store the position of each target
        // note that must account for the fact that all the Op::Target's
        // will be removed from the final version
        let mut i = 0;
        for op in self.code.iter() {
            if let Op::Target(id) = op {
                target[*id] = i;
            } else {
                i += 1;
            }
        }

        // TODO: check if all jumps are covered
        for op in self.code.iter() {
            let target_id = match op {
                Op::Jmp(id) => Some(*id),
                Op::JmpF(id) => Some(*id),
                _ => None
            };

            if let Some(id) = target_id {
                if target[id] == usize::MAX {
                    // TODO: better error reporting... name of target or somesuch...
                    return Err(Error::JumpTargetNotFound(id))
                }
            }
        }

        // 2nd pass -- remove all Op::Target from code
        self.code.retain(|x| !matches!(x, Op::Target(_)));

        // 3rd pass -- rewrite all jumps to use direct address instead of target ID
        for op in self.code.iter_mut() {
            match op {
                Op::Jmp(id) => *id = target[*id],
                Op::JmpF(id) => *id = target[*id],
                _ => (),
            }
        }

        Ok(self.code)
    }

    /// Return the final compiled sequence of `Op` codes.
    pub fn build(mut self) -> Result<Vec<Op>> {
        self.optimize();
        self.expand_targets()
    }
}

use std::ops::Range;
use crate::{
    token::{Token},
};

#[derive(Debug, Clone)]
pub enum Ast {
    /// literal integer
    Int(i64, Token),

    /// literal string
    Str(String, Token),

    /// literal list
    Lst(Vec<Ast>, Token),

    /// variable name
    Var(String, Token),

    /// binary operator
    BinOp(Token, Box<Ast>, Box<Ast>),

    /// loop (keyword, starting, comparison, body, updating)
    /// same node for all looping constructs (while, for)
    Loop(Token, Option<Box<Ast>>, Option<Box<Ast>>, Box<Ast>, Option<Box<Ast>>),

    /// ('if', <conditional>, <if_true>, <if_false>)
    /// for expressions, the <if_false> is required!
    IfElse(Token, Box<Ast>, Box<Ast>, Option<Box<Ast>>),

    /// A block is a sequence of Ast's ('{' '}')
    Block(Token, Vec<Ast>),

    /// expression wrapped as statement
    Sttm(Box<Ast>),

    /// Function call ('(', callee, [parameters])
    Call(Token, Box<Ast>, Vec<Ast>),

    /// Indexing
    Index(Token, Box<Ast>, Box<Ast>),
}

impl Ast {
    pub fn at(&self) -> Range<usize> {
        match self {
            Ast::Int(_, tk) => tk.at.clone(),
            Ast::Str(_, tk) => tk.at.clone(),
            Ast::Var(_, tk) => tk.at.clone(),
            Ast::Lst(lst, tk) => if lst.is_empty() { tk.at.clone() } else { lst.first().unwrap().at().start .. lst.last().unwrap().at().end },
            Ast::BinOp(_, lhs, rhs) => lhs.at().start .. rhs.at().end,
            Ast::Loop(tk, _, _, body, _) => tk.at.start .. body.at().end,
            Ast::Sttm(ast) => ast.at(),
            Ast::Block(tk, lst) => if lst.is_empty() { tk.at.clone() } else { lst.first().unwrap().at().start .. lst.last().unwrap().at().end },
            Ast::Call(tk, callee, args) => callee.at().start .. if args.is_empty() { tk.at.end } else { args.last().unwrap().at().end },
            Ast::Index(_, callee, index) => callee.at().start .. index.at().end,
            Ast::IfElse(tk, _, if_true, if_false) => tk.at.start .. if if_false.is_some() { if_false.as_ref().unwrap().at().end } else { if_true.at().end },
        }
    }

    pub fn pretty(&self) -> String {
        match self {
            Ast::Int(n, _) => n.to_string(),
            Ast::Str(s, _) => format!("{:?}", s),
            Ast::Lst(_, _) => format!("list"),
            Ast::Var(s, _) => s.clone(),
            Ast::BinOp(_, _, _) => format!("binary operator"),
            Ast::Loop(tk, _, _, _, _) => format!("{:?} loop", tk.kind),
            Ast::IfElse(_, _, _, _) => format!("conditional"),
            Ast::Block(_, _) => format!("block"),
            Ast::Sttm(_) => format!("statement"),
            Ast::Call(_, _, _) => format!("function call"),
            Ast::Index(_, _, _) => format!("indexing"),
        }
    }
}

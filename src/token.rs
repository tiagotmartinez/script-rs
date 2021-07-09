use std::ops::Range;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Kind {
    Int,
    Str,
    Id,

    If, Else,
    While,
    For,
    Fun,

    Add, Sub,
    Mul, Div, Mod,

    Lt, Lte,
    Gt, Gte,
    Assign, Eq,
    Not, NotEq,

    LPar, RPar,
    LBraces, RBraces,
    LBracket, RBracket,

    Semi, Comma,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub value: String,
    pub at: Range<usize>,
}

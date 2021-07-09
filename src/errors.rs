use crate::{
    vm::HeapPtr,
    opcodes::Op,
    value::Value,
    token::{Kind, Token},
    ast::Ast,
};

/// Result of a operation on the VM
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {

    // === VM execution errors ===

    /// Stack underflow on Value stack
    StackUnderflow,

    /// Access to an invalid memory location (out of range)
    MemoryAccessOutOfRange(HeapPtr),

    /// Access to a memory location that is currently empty
    InvalidMemoryAccess(HeapPtr),

    /// Global variable not found
    GlobalNotFound(String),

    /// Incompatible operands for operation
    IncompatibleOperands(Op, Value, Value),

    /// Index out of range
    IndexOutOfRange(Value, usize),

    /// An invalid opcode was found on code
    InvalidOpCode(usize),

    /// Attempted to append to a non-list
    InvalidAppend(Value),

    /// Jump to an unknown location
    JumpTargetNotFound(usize),

    // === Script Source errors ===

    /// Syntax error reading script text
    SyntaxError(usize),

    /// Unexpected EOF reading script text
    UnexpectedEOF,

    /// Invalid escape inside a string
    InvalidStringEscape(char, usize),

    /// Error parsing input (unexpected token)
    ParsingError(Token),

    /// Got a token, but was expecting other possibilities
    UnexpectedToken(Token, Vec<Kind>),

    /// Not a valid target for an assignment
    InvalidAssignmentTarget(Ast),

    /// Not enough arguments to a function call
    NotEnoughArguments(Ast, String, usize, usize),
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Error::StackUnderflow => write!(fmt, "Stack Underflow"),
            Error::MemoryAccessOutOfRange(ptr) => write!(fmt, "Memory access out of range at {:?}", ptr),
            Error::InvalidMemoryAccess(ptr) => write!(fmt, "Attempt to access empty memory position at {:?}", ptr),
            Error::GlobalNotFound(name) => write!(fmt, "Global variable '{}' not found", name),
            Error::IncompatibleOperands(op, lhs, rhs) => write!(fmt, "Cannot execute {:?} on {} and {}", op, lhs.type_name(), rhs.type_name()),
            Error::SyntaxError(at) => write!(fmt, "Syntax error at {}", at),
            Error::UnexpectedEOF => write!(fmt, "Unexpected end of source"),
            Error::InvalidStringEscape(c, at) => write!(fmt, "Invalid string escape '{}' at {}", c, at),
            Error::ParsingError(tk) => write!(fmt, "Unexpected token {:?} at {}", tk, tk.at.start),
            Error::UnexpectedToken(tk, possible) => {
                if possible.len() > 1 {
                    write!(fmt, "Unexpected token {:?} at {}, expected one of {:?}", tk, tk.at.start, possible)
                } else {
                    write!(fmt, "Unexpected token {:?} at {}, expected {:?}", tk, tk.at.start, possible[0])
                }
            }
            Error::InvalidAssignmentTarget(ast) => write!(fmt, "{:?} is not a valid target for an assignment", ast),
            Error::IndexOutOfRange(value, index) => write!(fmt, "Index out of range {} of {:?}", index, value),
            Error::InvalidOpCode(index) => write!(fmt, "Invalid opcode at {}", index),
            Error::NotEnoughArguments(_, name, given, expected) => write!(fmt, "Not enough arguments to {}, given {} but expected {}", name, given, expected),
            Error::InvalidAppend(target) => write!(fmt, "Cannot append to {}", target.type_name()),
            Error::JumpTargetNotFound(id) => write!(fmt, "Jump with unknown target {}", id),
        }
    }
}

impl Error {

    /// Extract more precise location information of offset `at` inside `source`.
    ///
    /// Return `(row-number, column-number, row-starting-offset, row-ending-offset)`
    /// where `row-number` and `column-number` are both 1-based.
    fn location(source: &str, at: usize) -> (usize, usize, usize, usize) {
        let mut row = 1;
        let mut column = 1;
        let mut row_start = 0;
        let mut row_end = 0;
        let mut found = false;
        for (i, c) in source.char_indices() {
            if i == at {
                found = true;
            } else if c == '\n' {
                if found {
                    row_end = i;
                    break;
                } else {
                    row += 1;
                    column = 1;
                    row_start = i + 1;
                }
            } else if !found {
                column += 1;
            }
        }

        if row_end == 0 {
            row_end = source.len();
        }

        (row, column, row_start, row_end)
    }

    /// Return two lines, separated with '\n':
    ///
    ///     (<row>, <col>): | <source-line-where-at-is>
    ///                     |        ^  (caret pointing for <col> inside <line>)
    ///
    fn pretty_source_line(source: &str, at: usize) -> String {
        let (row, column, row_start, row_end) = Self::location(source, at);
        let address = format!("({}, {}): ", row, column);
        let line: String = source.chars().skip(row_start).take(row_end - row_start).collect();
        let marker = format!("{}| {}^", " ".repeat(address.len()), " ".repeat(column - 1));
        format!("{}| {}\n{}", address, line, marker)
    }

    pub fn pretty(&self, source: &str) -> String {
        match self {
            Error::SyntaxError(at) =>
                format!("syntax error\n{}", Self::pretty_source_line(source, *at)),
            Error::UnexpectedEOF =>
                format!("unexpected end of file\n{}", Self::pretty_source_line(source, source.len())),
            Error::InvalidStringEscape(ch, at) =>
                format!("invalid escape '{}' inside a string\n{}", ch, Self::pretty_source_line(source, *at)),
            Error::ParsingError(tk) =>
                format!("unexpected input when reading a {:?} with value \"{}\"\n{}", tk.kind, tk.value, Self::pretty_source_line(source, tk.at.start)),
            Error::UnexpectedToken(tk, which) =>
                format!("got a {:?} but expected one of {:?}\n{}", tk.kind, which, Self::pretty_source_line(source, tk.at.start)),
            Error::InvalidAssignmentTarget(ast) =>
                format!("{} is not a valid target for assignment\n{}", ast.pretty(), Self::pretty_source_line(source, ast.at().start)),
            Error::NotEnoughArguments(ast, name, given, expected) =>
                format!("not enough arguments to function '{}' (given {}, expected {})\n{}", name, given, expected, Self::pretty_source_line(source, ast.at().start)),

            // others are internal VM errors that have not a really good printing
            _ => self.to_string(),
        }
    }
}

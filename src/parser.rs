use std::collections::VecDeque;
use crate::{
    lexer::Lexer,
    token::{Kind, Token},
    errors::{Result, Error},
    ast::Ast,
};

/// A `Parser` read `Token`s and return `Ast`s.
#[derive(Debug)]
pub struct Parser {
    source: VecDeque<Token>,
}

impl Parser {

    /// Create a new `Parser` for all `Token`s from `source`.
    ///
    /// Current implementation consumes *all* tokens from `source` and return
    /// an error if `source` has any error.
    ///
    /// `Token`s are stored internally and consumed incrementally as necessary.
    pub fn new(mut source: Lexer) -> Result<Parser> {
        Ok(Parser {
            source: source.collect()?.into_iter().collect(),
        })
    }

    /// `true` if there are no more Ast's to return
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Return a reference to current token, without extracting it
    fn current(&self) -> Result<&Token> {
        self.source.front().ok_or(Error::UnexpectedEOF)
    }

    // fn drop(&mut self) {
    //     self.source.pop_front();
    // }

    /// Extract self.current() from the queue of tokens
    fn pop(&mut self) -> Result<Token> {
        self.source.pop_front().ok_or(Error::UnexpectedEOF)
    }

    /// **Require** self.current() to be one of `what`, error otherwise
    fn expect(&mut self, what: &[Kind]) -> Result<Token> {
        if what.contains(&self.current()?.kind) {
            Ok(self.pop()?)
        } else {
            Err(Error::UnexpectedToken(self.pop()?, what.to_vec()))
        }
    }

    /// *If* current is one of `what` pop it, otherwise return None
    fn check(&mut self, what: &[Kind]) -> Option<Token> {
        if self.one_of(what) {
            self.source.pop_front()
        } else {
            None
        }
    }

    /// `true` if self.current() is not EOF and one of `what`
    fn one_of(&self, what: &[Kind]) -> bool {
        !self.is_empty() && what.contains(&self.current().unwrap().kind)
    }

    /// Generic algorithm for left-associative binary operators
    fn left_associative<F: Fn(&mut Self) -> Result<Ast>>(&mut self, which: &[Kind], previous: F) -> Result<Ast> {
        let mut lhs = previous(self)?;
        while self.one_of(which) {
            let tk = self.pop()?;
            let rhs = previous(self)?;
            lhs = Ast::BinOp(tk, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    /// Int | Str | Var | '(' Expr ')'
    fn atom(&mut self) -> Result<Ast> {
        let tk = self.pop()?;
        match tk.kind {
            Kind::Int => {
                let n = i64::from_str_radix(&tk.value, 10).map_err(|_| Error::ParsingError(tk.clone()))?;
                Ok(Ast::Int(n, tk))
            }
            Kind::Str => {
                Ok(Ast::Str(tk.value.clone(), tk))
            }
            Kind::Id => {
                Ok(Ast::Var(tk.value.clone(), tk))
            }
            Kind::LPar => {
                let e = self.expression()?;
                self.expect(&[Kind::RPar])?;
                Ok(e)
            }
            Kind::LBracket => {
                let v = self.list_of(Self::expression, Kind::Comma, Kind::RBracket)?;
                Ok(Ast::Lst(v, tk))
            }
            _ => {
                Err(Error::UnexpectedToken(tk, [Kind::Int, Kind::Str, Kind::Id, Kind::LPar].to_vec()))
            }
        }
    }

    /// Function call (with '()') or indexing (with '[]').
    fn call_or_index(&mut self) -> Result<Ast> {
        let mut lhs = self.atom()?;
        while self.one_of(&[Kind::LBracket, Kind::LPar]) {
            let tk = self.pop()?;
            if tk.kind == Kind::LBracket {
                let index = self.expression()?;
                lhs = Ast::Index(tk, Box::new(lhs), Box::new(index));
                self.expect(&[Kind::RBracket])?;
            } else if tk.kind == Kind::LPar {
                let args = self.list_of(Self::expression, Kind::Comma, Kind::RPar)?;
                lhs = Ast::Call(tk, Box::new(lhs), args);
            }
        }
        Ok(lhs)
    }

    /// Call_or_index [ '=' Expression ]
    fn assign(&mut self) -> Result<Ast> {
        // assignment is right associative
        let mut lhs = self.call_or_index()?;
        while self.one_of(&[Kind::Assign]) {
            let tk = self.pop()?;
            let rhs = self.expression()?;
            lhs = Ast::BinOp(tk, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    /// Assign [ { '*' | '/' | '%' } Assign ]*
    fn factor(&mut self) -> Result<Ast> {
        self.left_associative(&[Kind::Mul, Kind::Div, Kind::Mod], Self::assign)
    }

    /// Factor [ { '+' | '-' } Factor ]*
    fn term(&mut self) -> Result<Ast> {
        self.left_associative(&[Kind::Add, Kind::Sub], Self::factor)
    }

    /// Comparison operations.
    fn cmp(&mut self) -> Result<Ast> {
        self.left_associative(&[Kind::Lt, Kind::Lte, Kind::Gt, Kind::Gte, Kind::Eq, Kind::NotEq], Self::term)
    }

    /// Expression **always** leave something on the stack.
    fn expression(&mut self) -> Result<Ast> {
        self.cmp()
    }

    /// A `while` loop
    fn while_loop(&mut self) -> Result<Ast> {
        let tk = self.expect(&[Kind::While])?;
        let cmp = self.expression()?;
        let body = self.block()?;
        Ok(Ast::Loop(tk, None, Some(Box::new(cmp)), Box::new(body), None))
    }

    /// The `else` part of a `if_else` can be either a block or another `if`
    fn block_or_if(&mut self) -> Result<Ast> {
        if self.one_of(&[Kind::If]) {
            self.if_else()
        } else if self.one_of(&[Kind::LBraces]) {
            self.block()
        } else {
            Err(Error::UnexpectedToken(self.pop()?, vec![Kind::If, Kind::LBraces]))
        }
    }

    /// If-else conditional.
    fn if_else(&mut self) -> Result<Ast> {
        let tk = self.expect(&[Kind::If])?;
        let conditional = Box::new(self.expression()?);
        let if_true = Box::new(self.block()?);
        let if_false = if self.check(&[Kind::Else]).is_some() {
            Some(Box::new(self.block_or_if()?))
        } else {
            None
        };
        Ok(Ast::IfElse(tk, conditional, if_true, if_false))
    }

    /// Read a list of `previous` separated by `separator` and terminated by `terminator`.
    /// A trailing `separator` is allowed.
    fn list_of<F: Fn(&mut Self) -> Result<Ast>>(&mut self, previous: F, separator: Kind, terminator: Kind) -> Result<Vec<Ast>> {
        let mut v = vec![];

        while !self.check(&[terminator]).is_some() {
            v.push(previous(self)?);
            let tk = self.expect(&[separator, terminator])?;
            if tk.kind == terminator {
                break;
            }
        }

        Ok(v)
    }

    /// Sequence of statements inside '{}'s
    fn block(&mut self) -> Result<Ast> {
        let tk = self.expect(&[Kind::LBraces])?;
        let mut v = vec![];
        while !self.check(&[Kind::RBraces]).is_some() {
            v.push(self.statement()?);
        }
        Ok(Ast::Block(tk, v))
    }

    /// Statement execute and leave nothing on the stack
    fn statement(&mut self) -> Result<Ast> {
        if self.one_of(&[Kind::While]) {
            self.while_loop()
        } else if self.one_of(&[Kind::If]) {
            self.if_else()
        } else if self.one_of(&[Kind::LBraces]) {
            self.block()
        } else {
            // wrap an expression, so a `pop` is inserted
            let e = self.expression()?;
            self.expect(&[Kind::Semi])?;
            Ok(Ast::Sttm(Box::new(e)))
        }
    }

    /// Compute next `Ast` from source `Token`s.
    ///
    /// Return:
    /// * `Ok(Some(ast))` on success
    /// * `Ok(None)` on end of source
    /// * `Err(err)` on error
    pub fn next(&mut self) -> Result<Option<Ast>> {
        if self.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.statement()?))
        }
    }
}

use std::collections::HashMap;
use crate::{
    token::{Kind, Token},
    errors::{Error, Result},
};

/// Lexer for the script language
#[derive(Debug)]
pub struct Lexer {
    source: Vec<char>,
    index: usize,

    keywords: HashMap<String, Kind>,
    operators: Vec<(char, char, Kind, Option<Kind>)>,
}

impl Lexer {
    /// Create a new lexer for a string.
    pub fn new(source: &str) -> Lexer {
        // list of keywords and their token kind
        let keywords = {
            let mut h = HashMap::new();
            h.insert("if".to_string(), Kind::If);
            h.insert("else".to_string(), Kind::Else);
            h.insert("while".to_string(), Kind::While);
            h.insert("for".to_string(), Kind::For);
            h.insert("fun".to_string(), Kind::Fun);
            h
        };

        // list of operators
        // (<first-char>, <optional-second-char>, <first-char-kind>, <optional-second-char-kind>)
        // use '\0' for second-char if not present.
        let operators = vec![
            ('(', '\0', Kind::LPar,     None),
            (')', '\0', Kind::RPar,     None),
            ('{', '\0', Kind::LBraces,  None),
            ('}', '\0', Kind::RBraces,  None),
            ('[', '\0', Kind::LBracket, None),
            (']', '\0', Kind::RBracket, None),
            ('+', '\0', Kind::Add,      None),
            ('-', '\0', Kind::Sub,      None),
            ('/', '\0', Kind::Div,      None),
            ('*', '\0', Kind::Mul,      None),
            ('%', '\0', Kind::Mod,      None),
            (';', '\0', Kind::Semi,     None),
            (',', '\0', Kind::Comma,    None),
            ('<', '=',  Kind::Lt,       Some(Kind::Lte)),
            ('>', '=',  Kind::Gt,       Some(Kind::Gte)),
            ('!', '=',  Kind::Not,      Some(Kind::NotEq)),
            ('=', '=',  Kind::Assign,   Some(Kind::Eq)),
        ];

        Lexer {
            source: source.chars().collect(),
            index: 0,
            keywords,
            operators,
        }
    }

    /// Return `true` if reached end of source.
    pub fn is_empty(&self) -> bool {
        self.index >= self.source.len()
    }

    /// Current char in source of '\0' if EOF
    fn current(&self) -> char {
        self.at(0)
    }

    /// Return the char at `offset` relative to current source index
    fn at(&self, offset: usize) -> char {
        self.source.get(self.index + offset)
            .cloned()
            .unwrap_or('\0')
    }

    /// Skip current char
    fn drop(&mut self) {
        self.index += 1;
    }

    /// Return current and drop
    fn pop(&mut self) -> char {
        let c = self.current();
        self.drop();
        c
    }

    /// Skip all whitespace, return `true` if reached EOF
    fn skip_ws(&mut self) -> Result<bool> {
        while !self.is_empty() {
            if self.current().is_whitespace() {
                // skip whitespace
                self.drop();
            } else if self.current() == '/' && self.at(1) == '/' {
                // skip comment until end of line
                while !self.is_empty() && self.current() != '\n' {
                    self.drop();
                }
            } else {
                break
            }
        }
        Ok(self.is_empty())
    }

    /// `true` if `c` is the first char of an identifier
    fn is_first_id(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '$' || c == '_'
    }

    /// `true` if `c` is the rest (middle) of an identifier
    fn is_rest_id(c: char) -> bool {
        Self::is_first_id(c) || c.is_ascii_digit()
    }

    /// Read next integer from source
    fn next_int(&mut self) -> Result<Token> {
        let start = self.index;
        let mut v = String::new();
        while self.current().is_ascii_digit() {
            v.push(self.pop());
        }

        Ok(Token {
            kind: Kind::Int,
            value: v,
            at: start .. self.index,
        })
    }

    /// Read next identifier or keyword from source
    fn next_id(&mut self) -> Result<Token> {
        let start = self.index;
        let mut v = String::new();
        while Self::is_rest_id(self.current()) {
            v.push(self.pop())
        }

        Ok(Token {
            kind: self.keywords.get(&v).map(|k| k.clone()).unwrap_or(Kind::Id),
            value: v,
            at: start .. self.index,
        })
    }

    /// Read next quoted string from source
    fn next_str(&mut self) -> Result<Token> {
        let start = self.index;
        assert_eq!(self.current(), '"');
        self.drop();
        let mut v = String::new();
        while !self.is_empty() && self.current() != '"' {
            match self.pop() {
                '\\' => {
                    match self.pop() {
                        'n' => v.push('\n'),
                        't' => v.push('\t'),
                        'r' => v.push('\r'),
                        '\\' => v.push('\\'),
                        '"' => v.push('"'),
                        c => return Err(Error::InvalidStringEscape(c, self.index - 1)),
                    }
                }
                c => {
                    v.push(c);
                }
            }
        }

        if self.is_empty() {
            Err(Error::UnexpectedEOF)
        } else {
            self.drop();
            Ok(Token {
                kind: Kind::Str,
                value: v,
                at: start .. self.index,
            })
        }
    }

    /// Read next operator from source
    pub fn next_op(&mut self) -> Result<Token> {
        let start = self.index;
        for (c0, c1, fst, snd) in &self.operators {
            if self.current() == *c0 {
                self.index += 1;
                if snd.is_some() && self.current() == *c1 {
                    self.index += 1;
                    return Ok(Token {
                        kind: snd.clone().unwrap(),
                        value: [*c0, *c1].iter().collect(),
                        at: start .. self.index,
                    })
                } else {
                    return Ok(Token {
                        kind: fst.clone(),
                        value: c0.to_string(),
                        at: start .. self.index,
                    })
                }
            }
        }
        Err(Error::SyntaxError(self.index))
    }

    /// Read next `Token` from source.
    ///
    /// * Return `Ok(Some(tk))` if read a `Token` with success
    /// * Return `Ok(None)` if reached EOF
    /// * Return `Err(err)` in case of lexing error
    pub fn next(&mut self) -> Result<Option<Token>> {
        if self.skip_ws()? {
            Ok(None)
        } else if self.current().is_ascii_digit() {
            Ok(Some(self.next_int()?))
        } else if Self::is_first_id(self.current()) {
            Ok(Some(self.next_id()?))
        } else if self.current() == '"' {
            Ok(Some(self.next_str()?))
        } else {
            Ok(Some(self.next_op()?))
        }
    }

    /// Read all `Token`s from source.
    ///
    /// Return a `Vec` with all `Token`s from source, or the first error.
    pub fn collect(&mut self) -> Result<Vec<Token>> {
        let mut tks = vec![];
        while let Some(tk) = self.next()? {
            tks.push(tk);
        }
        Ok(tks)
    }
}

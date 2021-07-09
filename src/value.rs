use crate::{
    vm::{VM, HeapPtr},
    errors::{Error, Result},
    opcodes::Op,
};

/// Values supported by the script and its VM
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Str(String),
    List(Vec<HeapPtr>),
}

impl Value {
    /// Push into `roots` all HeapPtr accessible from self
    pub fn mark(&self, roots: &mut Vec<HeapPtr>) {
        match self {
            Value::List(values) => {
                values.iter().for_each(|ptr| roots.push(*ptr));
            }
            _ => {
                // no pointers inside, do nothing
            }
        }
    }

    /// `true` if this value presents a falsehood
    pub fn is_false(&self) -> bool {
        match self {
            Value::Int(n) if *n == 0 => true,
            _ => false
        }
    }

    /// Pretty formatting of values
    pub fn fmt(&self, vm: &VM, depth: usize) -> Result<String> {
        // XXX: perhaps move inside VM?
        match self {
            Value::Int(n) => Ok(n.to_string()),
            Value::Str(s) => Ok(s.clone()),
            Value::List(lst) => {
                if depth > 3 {
                    // avoid infinite recursion...
                    Ok("[...]".to_string())
                } else {
                    let mut s = "[".to_string();
                    let mut first = true;
                    for ptr in lst {
                        if !first {
                            s += ", ";
                        }
                        let v = vm.get(*ptr)?;
                        s += &v.fmt(vm, depth + 1)?;
                        first = false;
                    }
                    s += "]";
                    Ok(s)
                }
            }
        }
    }

    /// Return a display name for the type of this value
    pub fn type_name(&self) -> String {
        match self {
            Value::Int(_) => "integer".to_string(),
            Value::Str(_) => "string".to_string(),
            Value::List(_) => "list".to_string(),
        }
    }

    /// Return the "length" of this Value, as should be returned
    /// by the `length` built-in function
    pub fn length(&self) -> usize {
        match self {
            Value::Int(_) => 0,
            Value::Str(s) => s.chars().count(),
            Value::List(lst) => lst.len(),
        }
    }

    /// Compare `self` with `other` executing under `vm`.
    ///
    /// Result:
    /// * -1 if `self` < `other`
    /// * 0 if `self` == `other`
    /// * 1 if `self` > `other`
    ///
    /// Return `Result<i64>` instead of `Result<Value>` to make recursion easier...
    pub fn cmp(&self, vm: &VM, other: &Value) -> Result<i64> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                Ok(if a < b { -1 }
                else if a > b { 1 }
                else { 0 })
            }
            (Value::Str(a), Value::Str(b)) => {
                Ok(if a < b { -1 }
                else if a > b { 1 }
                else { 0 })
            }
            (Value::List(a), Value::List(b)) => {
                // only dereference pointers as long as necessary
                // there should be a nicer built-in API for this...
                let n = a.len().min(b.len());
                let mut i = 0;
                while i < n {
                    let av = vm.get(a[i])?;
                    let bv = vm.get(b[i])?;
                    let c = av.cmp(vm, bv)?;
                    if c != 0 {
                        return Ok(c)
                    }
                    i += 1;
                }
                if a.len() < b.len() { Ok(-1) }
                else if a.len() > b.len() { Ok(1) }
                else { Ok(0) }
            }
            _ => {
                Err(Error::IncompatibleOperands(Op::Lt, self.clone(), other.clone()))
            }
        }
    }

    /// Add `self` to `other`
    pub fn add(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                Ok(Value::Int(a + b))
            }
            (Value::Str(a), Value::Str(b)) => {
                Ok(Value::Str(a.to_owned() + b))
            }
            (Value::List(a), Value::List(b)) => {
                let mut c = a.clone();
                c.extend_from_slice(&b);
                Ok(Value::List(c))
            }
            _ => {
                Err(Error::IncompatibleOperands(Op::Add, self.clone(), other.clone()))
            }
        }
    }

    /// Subtract `other` from `self`
    pub fn sub(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                Ok(Value::Int(a - b))
            }
            _ => {
                Err(Error::IncompatibleOperands(Op::Sub, self.clone(), other.clone()))
            }
        }
    }

    /// Multiply `self` and `other`
    pub fn mul(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                Ok(Value::Int(a * b))
            }
            (Value::Str(a), Value::Int(b)) if *b >= 0 => {
                Ok(Value::Str(a.repeat(*b as usize)))
            }
            (Value::List(a), Value::Int(b)) if *b >= 0 => {
                Ok(Value::List(a.repeat(*b as usize)))
            }
            _ => {
                Err(Error::IncompatibleOperands(Op::Mul, self.clone(), other.clone()))
            }
        }
    }

    /// Divide `self` by `other`
    pub fn div(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                Ok(Value::Int(a / b))
            }
            _ => {
                Err(Error::IncompatibleOperands(Op::Div, self.clone(), other.clone()))
            }
        }
    }

    /// Remainder of `self` by `other`
    pub fn r#mod(&self, other: &Value) -> Result<Value> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => {
                Ok(Value::Int(a % b))
            }
            _ => {
                Err(Error::IncompatibleOperands(Op::Mod, self.clone(), other.clone()))
            }
        }
    }
}

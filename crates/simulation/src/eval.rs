//! Expressions compiled for a run and their evaluation.
//!
//! An [`Expr`] is an [`agq_language::Expression`] whose names have been
//! resolved once, when the run was compiled, to the elements they name.
//! Evaluating it needs an [`Env`] that knows what a path of elements holds in
//! the run (an attribute of the part whose behaviour runs, an accepted
//! payload, a value a scenario received); the evaluator follows the rest of
//! a chain through the fields of items.

use crate::value::{Field, Item, Value};
use agq_language::{BinaryOp, ElementId, UnaryOp};

/// A compiled expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Const(Value),
    /// A name or feature chain, as the elements of its steps, with the text
    /// as written for messages.
    Path {
        steps: Vec<ElementId>,
        text: String,
    },
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `new T(...)`: the type and every field of it, given or defaulted.
    New {
        ty: ElementId,
        type_name: String,
        fields: Vec<(FieldSlot, Option<Expr>)>,
    },
}

/// One field of an item type, as `new` fills it.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldSlot {
    pub feature: ElementId,
    pub aliases: Vec<ElementId>,
    pub name: String,
    /// The value the type gives the field when `new` does not.
    pub default: Option<Value>,
}

/// Why an expression could not be evaluated, in plain words.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalError(pub String);

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the names of a path hold in a run.
pub trait Env {
    /// The value at the start of `steps`, and how many steps that used. The
    /// evaluator follows the remaining steps through item fields.
    fn lookup(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), EvalError>;
}

/// Evaluates an expression.
pub fn eval(expr: &Expr, env: &dyn Env) -> Result<Value, EvalError> {
    match expr {
        Expr::Const(value) => Ok(value.clone()),
        Expr::Path { steps, text } => {
            let (mut value, used) = env.lookup(steps, text)?;
            for step in &steps[used..] {
                value = match value {
                    Value::Item(item) => item
                        .field(*step)
                        .map(|field| field.value.clone())
                        .ok_or_else(|| {
                            EvalError(format!("`{text}`: `{}` has no such field", item.type_name))
                        })?,
                    Value::Null => {
                        return Err(EvalError(format!(
                            "`{text}` reads a field of nothing (null)"
                        )));
                    }
                    other => {
                        return Err(EvalError(format!(
                            "`{text}` reads a field of {}, which has none",
                            other.kind()
                        )));
                    }
                };
            }
            Ok(value)
        }
        Expr::Unary(op, operand) => {
            let value = eval(operand, env)?;
            match (op, value) {
                (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                (UnaryOp::Negate, Value::Int(n)) => n
                    .checked_neg()
                    .map(Value::Int)
                    .ok_or_else(|| EvalError("a whole number overflowed".into())),
                (UnaryOp::Negate, Value::Real(r)) => Ok(Value::Real(-r)),
                (op, value) => Err(EvalError(format!(
                    "`{}` does not apply to {}",
                    op.symbol(),
                    value.kind()
                ))),
            }
        }
        Expr::Binary(op, left, right) => binary(*op, left, right, env),
        Expr::Cond(condition, then, otherwise) => match eval(condition, env)? {
            Value::Bool(true) => eval(then, env),
            Value::Bool(false) => eval(otherwise, env),
            other => Err(EvalError(format!(
                "the condition of `if ? else` is {}, not a Boolean",
                other.kind()
            ))),
        },
        Expr::New {
            ty,
            type_name,
            fields,
        } => {
            let mut values = Vec::with_capacity(fields.len());
            for (slot, given) in fields {
                let value = match given {
                    Some(expr) => eval(expr, env)?,
                    None => slot.default.clone().unwrap_or(Value::Null),
                };
                values.push(Field {
                    feature: slot.feature,
                    aliases: slot.aliases.clone(),
                    name: slot.name.clone(),
                    value,
                });
            }
            Ok(Value::Item(Item {
                ty: *ty,
                type_name: type_name.clone(),
                fields: values,
            }))
        }
    }
}

fn binary(op: BinaryOp, left: &Expr, right: &Expr, env: &dyn Env) -> Result<Value, EvalError> {
    use BinaryOp::*;
    // The logical operators look at their right side only when needed.
    if matches!(op, And | Or | Implies) {
        let a = boolean(op, eval(left, env)?)?;
        let decided = match op {
            And => (!a).then_some(false),
            Or => a.then_some(true),
            _ => (!a).then_some(true),
        };
        if let Some(result) = decided {
            return Ok(Value::Bool(result));
        }
        return Ok(Value::Bool(boolean(op, eval(right, env)?)?));
    }
    let a = eval(left, env)?;
    let b = eval(right, env)?;
    match op {
        Xor => Ok(Value::Bool(boolean(op, a)? != boolean(op, b)?)),
        Equal => Ok(Value::Bool(a.equals(&b))),
        NotEqual => Ok(Value::Bool(!a.equals(&b))),
        Less | LessOrEqual | Greater | GreaterOrEqual => {
            let ordering = match (&a, &b) {
                (Value::Str(x), Value::Str(y)) => x.cmp(y),
                _ => {
                    let (x, y) = numbers(op, &a, &b)?;
                    x.partial_cmp(&y).ok_or_else(|| {
                        EvalError(format!("`{}` cannot compare these numbers", op.symbol()))
                    })?
                }
            };
            Ok(Value::Bool(match op {
                Less => ordering.is_lt(),
                LessOrEqual => ordering.is_le(),
                Greater => ordering.is_gt(),
                _ => ordering.is_ge(),
            }))
        }
        Add | Subtract | Multiply => match (&a, &b) {
            (Value::Str(x), Value::Str(y)) if op == Add => Ok(Value::Str(format!("{x}{y}"))),
            (Value::Int(x), Value::Int(y)) => {
                let result = match op {
                    Add => x.checked_add(*y),
                    Subtract => x.checked_sub(*y),
                    _ => x.checked_mul(*y),
                };
                result
                    .map(Value::Int)
                    .ok_or_else(|| EvalError("a whole number overflowed".into()))
            }
            _ => {
                let (x, y) = numbers(op, &a, &b)?;
                Ok(Value::Real(match op {
                    Add => x + y,
                    Subtract => x - y,
                    _ => x * y,
                }))
            }
        },
        Divide => {
            let (x, y) = numbers(op, &a, &b)?;
            if y == 0.0 {
                return Err(EvalError("division by zero".into()));
            }
            Ok(Value::Real(x / y))
        }
        Remainder => match (&a, &b) {
            (Value::Int(_), Value::Int(0)) => {
                Err(EvalError("remainder of division by zero".into()))
            }
            (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x % y)),
            _ => Err(EvalError(format!(
                "`%` applies to whole numbers, not {} and {}",
                a.kind(),
                b.kind()
            ))),
        },
        And | Or | Implies => unreachable!("handled above"),
    }
}

fn boolean(op: BinaryOp, value: Value) -> Result<bool, EvalError> {
    value.as_bool().ok_or_else(|| {
        EvalError(format!(
            "`{}` applies to Booleans, not {}",
            op.symbol(),
            value.kind()
        ))
    })
}

fn numbers(op: BinaryOp, a: &Value, b: &Value) -> Result<(f64, f64), EvalError> {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(EvalError(format!(
            "`{}` applies to numbers, not {} and {}",
            op.symbol(),
            a.kind(),
            b.kind()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoNames;
    impl Env for NoNames {
        fn lookup(&self, _: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
            Err(EvalError(format!("no `{text}`")))
        }
    }

    fn int(n: i64) -> Box<Expr> {
        Box::new(Expr::Const(Value::Int(n)))
    }

    #[test]
    fn arithmetic_comparison_and_logic() {
        let sum = Expr::Binary(BinaryOp::Add, int(2), int(3));
        assert_eq!(eval(&sum, &NoNames), Ok(Value::Int(5)));
        let ratio = Expr::Binary(BinaryOp::Divide, int(1), int(4));
        assert_eq!(eval(&ratio, &NoNames), Ok(Value::Real(0.25)));
        let less = Expr::Binary(
            BinaryOp::Less,
            int(2),
            Box::new(Expr::Const(Value::Real(2.5))),
        );
        assert_eq!(eval(&less, &NoNames), Ok(Value::Bool(true)));
        let zero = Expr::Binary(BinaryOp::Divide, int(1), int(0));
        assert_eq!(
            eval(&zero, &NoNames),
            Err(EvalError("division by zero".into()))
        );
        // `false and <anything>` never evaluates its right side.
        let path = Expr::Path {
            steps: vec![],
            text: "missing".into(),
        };
        let and = Expr::Binary(
            BinaryOp::And,
            Box::new(Expr::Const(Value::Bool(false))),
            Box::new(path),
        );
        assert_eq!(eval(&and, &NoNames), Ok(Value::Bool(false)));
    }
}

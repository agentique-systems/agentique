//! Expressions (C-50): the part of KerML's `OwnedExpression` that scenarios,
//! behaviour and checks need. Literals, `null`, names and feature chains
//! (`attempts`, `job.code`, `Decision::allow`), unary `-` and `not`, the
//! arithmetic, comparison and logical operators, `if c ? a else b` and
//! `new T(a = 1, b = x)`.
//!
//! A name inside an expression is a [`Reference`] like any other: it is
//! linked to the element it names, keeps pointing at it across renames and
//! moves, and is printed with a name that leads back to it. A named argument
//! of `new T(...)` is held as the chain `T.a`, so it is linked to the feature
//! `a` of `T` by identity too; it is printed as `a`.
//!
//! What an expression means when it runs is decided by the runner that
//! evaluates it (the Simulation part); the language core parses, prints,
//! links and checks it.

use crate::tree::{Literal, Reference};
use std::fmt;

/// `-x`, `not x`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Negate,
    Not,
}

impl UnaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            UnaryOp::Negate => "-",
            UnaryOp::Not => "not",
        }
    }
}

/// A binary operator, from the loosest binding (`implies`) to the tightest
/// (`*`, `/`, `%`). All associate to the left, as in KerML.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Implies,
    Or,
    Xor,
    And,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

impl BinaryOp {
    pub const ALL: [BinaryOp; 15] = [
        BinaryOp::Implies,
        BinaryOp::Or,
        BinaryOp::Xor,
        BinaryOp::And,
        BinaryOp::Equal,
        BinaryOp::NotEqual,
        BinaryOp::Less,
        BinaryOp::LessOrEqual,
        BinaryOp::Greater,
        BinaryOp::GreaterOrEqual,
        BinaryOp::Add,
        BinaryOp::Subtract,
        BinaryOp::Multiply,
        BinaryOp::Divide,
        BinaryOp::Remainder,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            BinaryOp::Implies => "implies",
            BinaryOp::Or => "or",
            BinaryOp::Xor => "xor",
            BinaryOp::And => "and",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::LessOrEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterOrEqual => ">=",
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Remainder => "%",
        }
    }

    pub(crate) fn from_symbol(symbol: &str) -> Option<BinaryOp> {
        BinaryOp::ALL.into_iter().find(|op| op.symbol() == symbol)
    }

    /// How tightly the operator binds; a higher number binds tighter.
    pub fn precedence(self) -> u8 {
        match self {
            BinaryOp::Implies => 2,
            BinaryOp::Or => 3,
            BinaryOp::Xor => 4,
            BinaryOp::And => 5,
            BinaryOp::Equal | BinaryOp::NotEqual => 6,
            BinaryOp::Less
            | BinaryOp::LessOrEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterOrEqual => 7,
            BinaryOp::Add | BinaryOp::Subtract => 8,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 9,
        }
    }
}

/// One named argument of `new T(feature = value)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    /// The feature of the constructed type, held as the chain `T.feature`
    /// so it is linked to that feature by identity.
    pub feature: Reference,
    pub value: Expression,
}

impl Argument {
    /// The feature's name as written.
    pub fn name(&self) -> &str {
        self.feature.last_name()
    }
}

/// An expression. See the module documentation.
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Null,
    /// A name or feature chain.
    Name(Reference),
    Unary(UnaryOp, Box<Expression>),
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    /// `if condition ? then else otherwise`
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),
    /// `new T(a = 1, b = x)`
    New {
        ty: Reference,
        arguments: Vec<Argument>,
    },
}

/// Precedence of a conditional expression, below every binary operator.
const CONDITIONAL: u8 = 1;
/// Unary operators and primaries bind tighter than every binary operator.
const UNARY: u8 = 10;
const PRIMARY: u8 = 11;

impl Expression {
    /// A name as written, such as `job.code` or `Decision::allow`.
    pub fn name(text: &str) -> Expression {
        Expression::Name(Reference::new(text))
    }

    /// `new T(a = value, ...)` from names as written.
    pub fn new_item(ty: &str, arguments: Vec<(&str, Expression)>) -> Expression {
        Expression::New {
            ty: Reference::new(ty),
            arguments: arguments
                .into_iter()
                .map(|(name, value)| Argument {
                    feature: argument_feature(&Reference::new(ty), name),
                    value,
                })
                .collect(),
        }
    }

    pub fn binary(op: BinaryOp, left: Expression, right: Expression) -> Expression {
        Expression::Binary(op, Box::new(left), Box::new(right))
    }

    /// Every name in the expression, depth first, in the order written: a
    /// `new` expression's type before its arguments' features and values.
    pub fn references(&self) -> Vec<&Reference> {
        let mut out = Vec::new();
        self.walk(&mut |e| match e {
            Expression::Name(reference) => out.push(reference),
            Expression::New { ty, arguments } => {
                out.push(ty);
                for argument in arguments {
                    out.push(&argument.feature);
                }
            }
            _ => {}
        });
        out
    }

    /// The references of [`Expression::references`], in the same order.
    pub fn references_mut(&mut self) -> Vec<&mut Reference> {
        let mut out = Vec::new();
        collect_mut(self, &mut out);
        out
    }

    /// Calls `visit` on this expression and then on each of its parts, in
    /// the order written. A `new` expression's argument values are visited
    /// after the `new` expression itself.
    pub fn walk<'a>(&'a self, visit: &mut dyn FnMut(&'a Expression)) {
        visit(self);
        match self {
            Expression::Literal(_) | Expression::Null | Expression::Name(_) => {}
            Expression::Unary(_, operand) => operand.walk(visit),
            Expression::Binary(_, left, right) => {
                left.walk(visit);
                right.walk(visit);
            }
            Expression::Conditional(condition, then, otherwise) => {
                condition.walk(visit);
                then.walk(visit);
                otherwise.walk(visit);
            }
            Expression::New { arguments, .. } => {
                for argument in arguments {
                    argument.value.walk(visit);
                }
            }
        }
    }

    /// How tightly the expression binds, for parentheses when printing.
    pub(crate) fn precedence(&self) -> u8 {
        match self {
            Expression::Conditional(..) => CONDITIONAL,
            Expression::Binary(op, ..) => op.precedence(),
            Expression::Unary(..) => UNARY,
            Expression::Literal(Literal::Integer(text) | Literal::Real(text))
                if text.starts_with('-') =>
            {
                UNARY
            }
            _ => PRIMARY,
        }
    }

    /// Writes the expression, naming each reference with `name` (for
    /// example the printer's name that leads back to the target). Named
    /// arguments are written with the last step of their chain.
    pub fn write(&self, out: &mut String, name: &mut dyn FnMut(&Reference) -> String) {
        match self {
            Expression::Literal(literal) => out.push_str(&literal.to_string()),
            Expression::Null => out.push_str("null"),
            Expression::Name(reference) => out.push_str(&name(reference)),
            Expression::Unary(op, operand) => {
                out.push_str(op.symbol());
                if *op == UnaryOp::Not {
                    out.push(' ');
                }
                operand.write_operand(out, name, UNARY, false);
            }
            Expression::Binary(op, left, right) => {
                left.write_operand(out, name, op.precedence(), false);
                out.push(' ');
                out.push_str(op.symbol());
                out.push(' ');
                right.write_operand(out, name, op.precedence(), true);
            }
            Expression::Conditional(condition, then, otherwise) => {
                out.push_str("if ");
                condition.write_operand(out, name, CONDITIONAL, true);
                out.push_str(" ? ");
                then.write_operand(out, name, CONDITIONAL, true);
                out.push_str(" else ");
                otherwise.write_operand(out, name, CONDITIONAL, false);
            }
            Expression::New { ty, arguments } => {
                out.push_str("new ");
                out.push_str(&name(ty));
                out.push('(');
                for (i, argument) in arguments.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let written = name(&argument.feature);
                    out.push_str(last_step(&written));
                    out.push_str(" = ");
                    argument.value.write(out, name);
                }
                out.push(')');
            }
        }
    }

    /// Writes an operand of an operator of precedence `outer`, in
    /// parentheses when it binds looser (or equally, on the right: every
    /// operator associates to the left).
    fn write_operand(
        &self,
        out: &mut String,
        name: &mut dyn FnMut(&Reference) -> String,
        outer: u8,
        right: bool,
    ) {
        let inner = self.precedence();
        let parens = inner < outer || (right && inner == outer && inner < UNARY);
        if parens {
            out.push('(');
        }
        self.write(out, name);
        if parens {
            out.push(')');
        }
    }
}

fn collect_mut<'a>(expression: &'a mut Expression, out: &mut Vec<&'a mut Reference>) {
    match expression {
        Expression::Literal(_) | Expression::Null => {}
        Expression::Name(reference) => out.push(reference),
        Expression::Unary(_, operand) => collect_mut(operand, out),
        Expression::Binary(_, left, right) => {
            collect_mut(left, out);
            collect_mut(right, out);
        }
        Expression::Conditional(condition, then, otherwise) => {
            collect_mut(condition, out);
            collect_mut(then, out);
            collect_mut(otherwise, out);
        }
        Expression::New { ty, arguments } => {
            out.push(ty);
            // Features first, then values: the order of `references`.
            let mut values = Vec::new();
            for argument in arguments.iter_mut() {
                out.push(&mut argument.feature);
                values.push(&mut argument.value);
            }
            for value in values {
                collect_mut(value, out);
            }
        }
    }
}

/// The last step of a printed feature chain: the text after the last `.`
/// outside a quoted name.
fn last_step(text: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '\'' => quoted = !quoted,
            '.' if !quoted => start = i + 1,
            _ => {}
        }
    }
    &text[start..]
}

/// The chain `T.feature` that holds a named argument of `new T(...)`.
pub(crate) fn argument_feature(ty: &Reference, feature: &str) -> Reference {
    let mut reference = ty.clone();
    reference.steps.truncate(1);
    for step in &mut reference.steps {
        step.target = None;
    }
    reference.steps.push(crate::tree::Step {
        name: crate::tree::QualifiedName::new([feature]),
        target: None,
    });
    reference
}

impl fmt::Display for Expression {
    /// The expression with every name as written.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(&mut out, &mut |reference| reference.to_string());
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(n: &str) -> Expression {
        Expression::Literal(Literal::Integer(n.into()))
    }

    #[test]
    fn prints_the_fewest_parentheses_that_keep_the_meaning() {
        let sum = Expression::binary(BinaryOp::Add, Expression::name("a"), int("1"));
        let product = Expression::binary(BinaryOp::Multiply, sum.clone(), int("2"));
        assert_eq!(product.to_string(), "(a + 1) * 2");
        let left = Expression::binary(BinaryOp::Subtract, sum.clone(), int("2"));
        assert_eq!(left.to_string(), "a + 1 - 2");
        let right = Expression::binary(BinaryOp::Subtract, int("2"), sum);
        assert_eq!(right.to_string(), "2 - (a + 1)");
        let not = Expression::Unary(
            UnaryOp::Not,
            Box::new(Expression::binary(
                BinaryOp::And,
                Expression::name("a"),
                Expression::name("b"),
            )),
        );
        assert_eq!(not.to_string(), "not (a and b)");
    }

    #[test]
    fn a_new_expression_names_its_arguments_by_their_feature() {
        let item = Expression::new_item(
            "Verdict",
            vec![
                ("decision", Expression::name("Decision::allow")),
                (
                    "confidence",
                    Expression::Literal(Literal::Real("0.9".into())),
                ),
            ],
        );
        assert_eq!(
            item.to_string(),
            "new Verdict(decision = Decision::allow, confidence = 0.9)"
        );
        let names: Vec<String> = item.references().iter().map(|r| r.to_string()).collect();
        assert_eq!(
            names,
            [
                "Verdict",
                "Verdict.decision",
                "Verdict.confidence",
                "Decision::allow"
            ]
        );
        let mut copy = item.clone();
        assert_eq!(copy.references_mut().len(), 4);
    }
}

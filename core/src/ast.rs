use crate::err::{IntErr, Interrupt, Never};
use crate::interrupt::test_int;
use crate::num::{Base, FormattingStyle, Number};
use crate::parser::ParseOptions;
use crate::scope::Scope;
use crate::value::Value;
use std::fmt::{Debug, Error, Formatter};

#[derive(Clone)]
pub enum Expr {
    Num(Number),
    Ident(String),
    Parens(Box<Expr>),
    UnaryMinus(Box<Expr>),
    UnaryPlus(Box<Expr>),
    UnaryDiv(Box<Expr>),
    Factorial(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    // Call a function or multiply the expressions
    Apply(Box<Expr>, Box<Expr>),
    // Call a function, or throw an error if lhs is not a function
    ApplyFunctionCall(Box<Expr>, Box<Expr>),
    // Multiply the expressions
    ApplyMul(Box<Expr>, Box<Expr>),

    As(Box<Expr>, Box<Expr>),
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Self::Num(n) => write!(f, "{:?}", n),
            Self::Ident(ident) => write!(f, "{}", ident),
            Self::Parens(x) => write!(f, "({:?})", *x),
            Self::UnaryMinus(x) => write!(f, "(-{:?})", *x),
            Self::UnaryPlus(x) => write!(f, "(+{:?})", *x),
            Self::UnaryDiv(x) => write!(f, "(/{:?})", *x),
            Self::Factorial(x) => write!(f, "{:?}!", *x),
            Self::Add(a, b) => write!(f, "({:?}+{:?})", *a, *b),
            Self::Sub(a, b) => write!(f, "({:?}-{:?})", *a, *b),
            Self::Mul(a, b) => write!(f, "({:?}*{:?})", *a, *b),
            Self::Div(a, b) => write!(f, "({:?}/{:?})", *a, *b),
            Self::Pow(a, b) => write!(f, "({:?}^{:?})", *a, *b),
            Self::Apply(a, b) => write!(f, "({:?} ({:?}))", *a, *b),
            Self::ApplyFunctionCall(a, b) | Self::ApplyMul(a, b) => {
                write!(f, "({:?} {:?})", *a, *b)
            }
            Self::As(a, b) => write!(f, "({:?} as {:?})", *a, *b),
        }
    }
}

pub fn evaluate<I: Interrupt>(
    expr: Expr,
    scope: &mut Scope,
    options: ParseOptions,
    int: &I,
) -> Result<Value, IntErr<String, I>> {
    test_int(int)?;
    let mut evaluate = |expr: Expr| evaluate(expr, scope, options, int);
    Ok(match expr {
        Expr::Num(n) => Value::Num(n),
        Expr::Ident(ident) => resolve_identifier(ident.as_str(), scope, options, int)?,
        Expr::Parens(x) => evaluate(*x)?,
        Expr::UnaryMinus(x) => Value::Num(-evaluate(*x)?.expect_num()?),
        Expr::UnaryPlus(x) => Value::Num(evaluate(*x)?.expect_num()?),
        Expr::UnaryDiv(x) => Value::Num(
            Number::from(1)
                .div(evaluate(*x)?.expect_num()?, int)
                .map_err(IntErr::into_string)?,
        ),
        Expr::Factorial(x) => Value::Num(evaluate(*x)?.expect_num()?.factorial(int)?),
        Expr::Add(a, b) => Value::Num(
            evaluate(*a)?
                .expect_num()?
                .add(evaluate(*b)?.expect_num()?, int)?,
        ),
        Expr::Sub(a, b) => Value::Num(
            evaluate(*a)?
                .expect_num()?
                .sub(evaluate(*b)?.expect_num()?, int)?,
        ),
        Expr::Mul(a, b) => Value::Num(
            evaluate(*a)?
                .expect_num()?
                .mul(evaluate(*b)?.expect_num()?, int)?,
        ),
        Expr::ApplyMul(a, b) => evaluate(*a)?.apply(&evaluate(*b)?, true, true, scope, int)?,
        Expr::Div(a, b) => Value::Num(
            evaluate(*a)?
                .expect_num()?
                .div(evaluate(*b)?.expect_num()?, int)
                .map_err(IntErr::into_string)?,
        ),
        Expr::Pow(a, b) => Value::Num(
            evaluate(*a)?
                .expect_num()?
                .pow(evaluate(*b)?.expect_num()?, int)?,
        ),
        Expr::Apply(a, b) => evaluate(*a)?.apply(&evaluate(*b)?, true, false, scope, int)?,
        Expr::ApplyFunctionCall(a, b) => {
            evaluate(*a)?.apply(&evaluate(*b)?, false, false, scope, int)?
        }
        Expr::As(a, b) => match evaluate(*b)? {
            Value::Num(b) => Value::Num(evaluate(*a)?.expect_num()?.convert_to(b, int)?),
            Value::Format(fmt) => Value::Num(evaluate(*a)?.expect_num()?.with_format(fmt)),
            Value::Dp => Value::Num(
                evaluate(*a)?
                    .expect_num()?
                    .with_format(FormattingStyle::ApproxFloat(10)),
            ),
            Value::Base(base) => Value::Num(evaluate(*a)?.expect_num()?.with_base(base)),
            Value::Func(_) => {
                return Err("Unable to convert value to a function".to_string())?;
            }
        },
    })
}

fn eval<I: Interrupt>(
    input: &'static str,
    scope: &mut Scope,
    int: &I,
) -> Result<Value, IntErr<Never, I>> {
    let options = crate::parser::ParseOptions::default();
    crate::eval::evaluate_to_value(input, options, scope, int).map_err(crate::err::IntErr::unwrap)
}

fn resolve_identifier<I: Interrupt>(
    ident: &str,
    scope: &mut Scope,
    options: ParseOptions,
    int: &I,
) -> Result<Value, IntErr<String, I>> {
    if options.gnu_compatible {
        return Ok(match ident {
            "exp" => Value::Func("exp"),
            "sqrt" => Value::Func("sqrt"),
            "ln" => Value::Func("ln"),
            "log2" => Value::Func("log2"),
            "log10" => Value::Func("log10"),
            "tan" => Value::Func("tan"),
            "asin" => Value::Func("asin"),
            "approx." | "approximately" => Value::Func("approximately"),
            _ => scope.get(ident, int)?,
        });
    }
    Ok(match ident {
        "pi" => eval("approx. 3.141592653589793238", scope, int)?,
        "e" => eval("approx. 2.718281828459045235", scope, int)?,
        "i" => Value::Num(Number::i()),
        // TODO: we want to forward any interrupt, but panic on any other error
        // or statically prove that no other error can occur
        //"c" => eval("299792458 m / s", scope, int)?,
        "sqrt" => Value::Func("sqrt"),
        "cbrt" => Value::Func("cbrt"),
        "abs" => Value::Func("abs"),
        "sin" => Value::Func("sin"),
        "cos" => Value::Func("cos"),
        "tan" => Value::Func("tan"),
        "asin" => Value::Func("asin"),
        "acos" => Value::Func("acos"),
        "atan" => Value::Func("atan"),
        "sinh" => Value::Func("sinh"),
        "cosh" => Value::Func("cosh"),
        "tanh" => Value::Func("tanh"),
        "asinh" => Value::Func("asinh"),
        "acosh" => Value::Func("acosh"),
        "atanh" => Value::Func("atanh"),
        "ln" => Value::Func("ln"),
        "log2" => Value::Func("log2"),
        "log10" => Value::Func("log10"),
        "exp" => Value::Func("exp"),
        "approx." | "approximately" => Value::Func("approximately"),
        "auto" => Value::Format(FormattingStyle::Auto),
        "exact" => Value::Format(FormattingStyle::ExactFloatWithFractionFallback),
        "fraction" => Value::Format(FormattingStyle::ExactFraction),
        "float" => Value::Format(FormattingStyle::ExactFloat),
        "dp" => Value::Dp,
        "base" => Value::Func("base"),
        "decimal" => Value::Base(Base::from_plain_base(10).map_err(|e| e.to_string())?),
        "hex" | "hexadecimal" => Value::Base(Base::from_plain_base(16).map_err(|e| e.to_string())?),
        "binary" => Value::Base(Base::from_plain_base(2).map_err(|e| e.to_string())?),
        "octal" => Value::Base(Base::from_plain_base(8).map_err(|e| e.to_string())?),
        _ => scope.get(ident, int)?,
    })
}

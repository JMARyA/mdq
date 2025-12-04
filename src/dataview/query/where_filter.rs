use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    combinator::map,
    multi::separated_list0,
    sequence::{delimited, preceded, tuple},
    IResult, Parser,
};

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(Value),
    Identifier(String),
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

fn identifier(input: &str) -> IResult<&str, Expr> {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == '.';

    map(take_while1(is_ident), |s: &str| {
        Expr::Identifier(s.to_string())
    })
    .parse(input)
}

fn literal(input: &str) -> IResult<&str, Expr> {
    map(
        alt((
            // Quoted string: "hello"
            delimited(char('"'), take_while1(|c| c != '"'), char('"')),
            // Numbers: integer or float
            take_while1(|c: char| c.is_digit(10) || c == '.' || c == '-'),
            // Boolean or null
            alt((tag("true"), tag("false"), tag("null"))),
        )),
        |txt: &str| {
            // Try parsing number
            if let Ok(n) = txt.parse::<i64>() {
                Expr::Literal(Value::from(n))
            } else if let Ok(f) = txt.parse::<f64>() {
                Expr::Literal(Value::from(f))
            } else if txt == "true" {
                Expr::Literal(Value::Bool(true))
            } else if txt == "false" {
                Expr::Literal(Value::Bool(false))
            } else if txt == "null" {
                Expr::Literal(Value::Null)
            } else {
                Expr::Literal(Value::String(txt.to_string()))
            }
        },
    )
    .parse(input)
}

fn function_call(input: &str) -> IResult<&str, Expr> {
    map(
        tuple((
            take_while1(|c: char| c.is_alphabetic()),
            delimited(
                char('('),
                separated_list0(delimited(multispace0, char(','), multispace0), expr),
                char(')'),
            ),
        )),
        |(name, args)| Expr::FunctionCall {
            name: name.to_string(),
            args,
        },
    )
    .parse(input)
}

fn primary(input: &str) -> IResult<&str, Expr> {
    preceded(
        multispace0,
        alt((
            map(preceded(char('!'), primary), |e| Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(e),
            }),
            function_call,
            literal,
            identifier,
            delimited(char('('), expr, char(')')),
        )),
    )
    .parse(input)
}

fn binary_op(input: &str) -> IResult<&str, BinaryOp> {
    preceded(
        multispace0,
        alt((
            map(tag("AND"), |_| BinaryOp::And),
            map(tag("OR"), |_| BinaryOp::Or),
            map(tag("<="), |_| BinaryOp::Lte),
            map(tag(">="), |_| BinaryOp::Gte),
            map(tag("!="), |_| BinaryOp::Neq),
            map(tag("="), |_| BinaryOp::Eq),
            map(tag("<"), |_| BinaryOp::Lt),
            map(tag(">"), |_| BinaryOp::Gt),
            map(tag("+"), |_| BinaryOp::Add),
            map(tag("-"), |_| BinaryOp::Sub),
            map(tag("*"), |_| BinaryOp::Mul),
            map(tag("/"), |_| BinaryOp::Div),
        )),
    )
    .parse(input)
}

fn comparison(input: &str) -> IResult<&str, Expr> {
    let (input, left) = primary(input)?;

    let mut rest = input;
    let mut node = left;

    while let Ok((next, op)) = binary_op(rest) {
        let (next, right) = primary(next)?;
        node = Expr::BinaryOp {
            left: Box::new(node),
            op,
            right: Box::new(right),
        };
        rest = next;
    }

    Ok((rest, node))
}
fn expr(input: &str) -> IResult<&str, Expr> {
    let (input, left) = comparison(input)?;

    let mut rest = input;
    let mut node = left;

    while let Ok((next, op)) = binary_op(rest) {
        match op {
            BinaryOp::And | BinaryOp::Or => {
                let (next, right) = comparison(next)?;
                node = Expr::BinaryOp {
                    left: Box::new(node),
                    op,
                    right: Box::new(right),
                };
                rest = next;
            }
            _ => break,
        }
    }

    Ok((rest, node))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WhereFilter {
    pub expression: Expr,
}

impl WhereFilter {
    pub fn new(expr: Expr) -> Self {
        Self { expression: expr }
    }

    pub fn parse(input: &str) -> IResult<&str, Self> {
        let (rest, expr) = expr(input)?;
        Ok((rest, Self { expression: expr }))
    }

    pub fn eval(&self, document: &serde_json::Value) -> Result<bool, ()> {
        let val = eval_expr(&self.expression, document);
        if let Ok(Value::Bool(b)) = val {
            return Ok(b);
        }

        Err(())
    }
}

pub fn get_ident(ident: &str, document: &serde_json::Value) -> Option<serde_json::Value> {
    Some(document.as_object()?.get(ident)?.clone())
}

pub enum ExpressionError {
    NoSuchIdent(String),
    General,
}

pub fn eval_expr(
    expr: &Expr,
    document: &serde_json::Value,
) -> Result<serde_json::Value, ExpressionError> {
    println!("matching {:?}", expr);
    match expr {
        Expr::Literal(value) => Ok(value.clone()),
        Expr::Identifier(ident) => {
            let val = get_ident(ident, document); // strict: .ok_or(ExpressionError::NoSuchIdent(ident.clone()))
            if let Some(val) = val {
                Ok(val)
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Expr::FunctionCall { name, args } => todo!(),
        Expr::UnaryOp { op, expr } => todo!(),
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOp::And => todo!(),
            BinaryOp::Or => todo!(),
            BinaryOp::Eq => {
                let (a, b) = (eval_expr(&left, document)?, eval_expr(&right, document)?);
                Ok(json!(a == b))
            }
            BinaryOp::Neq => todo!(),
            BinaryOp::Lt => todo!(),
            BinaryOp::Lte => todo!(),
            BinaryOp::Gt => todo!(),
            BinaryOp::Gte => todo!(),
            BinaryOp::Add => todo!(),
            BinaryOp::Sub => todo!(),
            BinaryOp::Mul => todo!(),
            BinaryOp::Div => todo!(),
        },
    }
}

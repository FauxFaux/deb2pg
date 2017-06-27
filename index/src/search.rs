use std::env;
use std::fmt;
use std::io;

use regex_syntax::Expr;

use errors::*;

type Tri = u32;

use tri;

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Op {
    And(Vec<Op>),
    Or(Vec<Op>),
    Any,
    Lit(Tri),
}

fn render_grams_in(vec: &Vec<Op>) -> String {
    let mut ret = String::with_capacity(vec.len() * 4);
    for item in vec {
        ret.push_str(format!("{} ", item).as_str());
    }
    // remove trailing space if possible
    ret.pop();
    ret
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Op::Lit(tri) => write!(f, "{}", tri::explain_packed(tri as usize)),
            Op::And(ref vec) => write!(f, "and({})", render_grams_in(vec)),
            Op::Or(ref vec) => write!(f, "or({})", render_grams_in(vec)),
            Op::Any => write!(f, "FUUUU"),
        }
    }
}

pub fn analyze(e: &Expr) -> Result<Op> {
    println!("unpacking: {}", e);
    match *e {
        Expr::Group {
            ref e,
            i: _,
            name: _,
        } => {
            println!("group of..");
            analyze(&e)
        }
        Expr::Repeat {
            ref e,
            ref r,
            greedy,
        } => {
            use regex_syntax::Repeater;
            println!("{} repeat of {} ..", greedy, r);
            match *r {
                Repeater::ZeroOrOne |
                Repeater::ZeroOrMore |
                Repeater::Range { min: 0, max: _ } => Ok(Op::Any),
                _ => analyze(&e),
            }
        }
        Expr::Concat(ref exprs) => {
            println!("{} different expressions ..", exprs.len());
            let maybe: Result<Vec<Op>> = exprs.iter().map(analyze).collect();
            Ok(Op::And(maybe?))
        }
        Expr::Alternate(ref exprs) => {
            println!("{} alternate expressions ..", exprs.len());
            let maybe: Result<Vec<Op>> = exprs.iter().map(analyze).collect();
            Ok(Op::Or(maybe?))
        }

        Expr::Literal { ref chars, casei } => {
            let lit = chars.iter().collect::<String>();
            if casei {
                bail!(format!(
                    "unsupported: case insensitive matching on '{}'",
                    lit
                ));
            }
            println!("literal: {} ({})", lit, casei);

            Ok(Op::And(
                tri::trigrams_full(lit.as_str())
                    .into_iter()
                    .map(|gram| Op::Lit(gram as u32))
                    .collect(),
            ))
        }

        ref other => bail!(format!("unimplemented: {}", other)),
    }
}

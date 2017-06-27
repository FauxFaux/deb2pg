use search::analyze;
use regex_syntax::Expr;

fn parse(s: &str) -> Expr {
    Expr::parse(s).expect("valid regex")
}

#[test]
fn analyze_lit() {
    println!("{}", analyze(&parse("hello")).unwrap());
}

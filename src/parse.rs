use std::{iter::Peekable, slice::Iter};

use crate::{
    ast::*,
    common::{TokType, Token},
};

pub fn parse(tokens: Vec<Token>) -> Ast {
    let mut iter = tokens.iter().peekable();
    let mut ast = Ast { 0: Vec::new() };

    // parse external decl
    loop {
        let peek = iter.peek();
        match peek {
            Some(t) if (is_data_type(t)) => {
                let (return_type, name) = parse_declator(&mut iter);
                let ext = match iter.peek() {
                    // parse function
                    Some(t) if t.tok == TokType::ParentOpen => {
                        let (params, cmp_stmt) = parse_func_params_body(&mut iter);
                        ExtDecl::Func(FuncDecl {
                            return_type,
                            name,
                            params,
                            cmp_stmt,
                        })
                    }
                    // parse global function
                    _ => {
                        let expr = match iter.peek() {
                            Some(t) if t.tok == TokType::Assign => {
                                consume_any(&mut iter);
                                Some(parse_expr(&mut iter))
                            }
                            _ => None,
                        };
                        consume(&mut iter, TokType::Semicolon);
                        ExtDecl::Global(GlobalVarDecl(return_type, name, expr))
                    }
                };
                ast.0.push(ext);
            }
            None => break,
            Some(t) => panic!("unexpected {}", t),
        }
    }

    ast
}

/// parse type and name
/// example: int main
fn parse_declator(iter: &mut Peekable<Iter<Token>>) -> (DataType, String) {
    let dt = iter
        .next()
        .and_then(|t| parse_data_type(t))
        .expect("expect data type");
    (dt, parse_id(iter))
}

/// parse function parameters and body (compound statement)
fn parse_func_params_body(iter: &mut Peekable<Iter<Token>>) -> (Vec<ParamDecl>, CmpStmt) {
    // parameters
    consume(iter, TokType::ParentOpen);
    let params = parse_parameters(iter);
    consume(iter, TokType::ParentClose);

    // compound statement
    let cmp_stmt = parse_compound_stmt(iter);

    (params, cmp_stmt)
}

/// parse list of parameters
fn parse_parameters(iter: &mut Peekable<Iter<Token>>) -> Vec<ParamDecl> {
    let mut vec: Vec<ParamDecl> = Vec::new();
    match iter.peek() {
        Some(t) if is_data_type(t) => {
            vec.push(parse_parameter(iter));

            // check if comma
            loop {
                match iter.peek() {
                    Some(t) if t.tok == TokType::Comma => {
                        consume_any(iter);
                        vec.push(parse_parameter(iter));
                    }
                    _ => break,
                }
            }
        }
        _ => (),
    }

    vec
}

fn parse_parameter(iter: &mut Peekable<Iter<Token>>) -> ParamDecl {
    let tp = match iter.next() {
        Some(t) => parse_data_type(t).expect("expected data type"),
        _ => panic!("unexpected EOF"),
    };
    ParamDecl {
        data_type: tp,
        name: parse_id(iter),
    }
}

fn parse_compound_stmt(iter: &mut Peekable<Iter<Token>>) -> CmpStmt {
    consume(iter, TokType::BracketOpen);

    let mut stmts: Vec<Stmt> = Vec::new();

    // parse stmts
    loop {
        if let Some(stmt) = parse_stmt(iter) {
            stmts.push(stmt);
        } else {
            break;
        }
    }

    consume(iter, TokType::BracketClose);

    CmpStmt { stmts }
}

fn parse_stmt(iter: &mut Peekable<Iter<Token>>) -> Option<Stmt> {
    if is_expr(iter) {
        return Some(parse_expr_stmt(iter));
    }

    let stmt = match iter.peek() {
        Some(t) if is_data_type(t) => parse_var_decl_stmt(iter),
        Some(t) if t.tok == TokType::KeywordReturn => parse_return_stmt(iter),
        Some(t) if t.tok == TokType::BracketOpen => Stmt::Compound(parse_compound_stmt(iter)),
        Some(t) if t.tok == TokType::BracketClose => return None,
        Some(t) => panic!("unexpected {}", t),
        _ => panic!("unexpected EOF"),
    };
    Some(stmt)
}

fn parse_var_decl_stmt(iter: &mut Peekable<Iter<Token>>) -> Stmt {
    let decl = parse_var_decl(iter);
    consume(iter, TokType::Semicolon);
    Stmt::VarDecl(decl)
}

fn parse_var_decl(iter: &mut Peekable<Iter<Token>>) -> VarDecl {
    let data_type = parse_data_type(iter.next().unwrap()).expect("expected data type");
    let name: String = parse_id(iter);
    let expr = if is_peek_tok(iter, TokType::Assign) {
        consume(iter, TokType::Assign);
        Some(parse_expr(iter))
    } else {
        None
    };
    VarDecl(data_type, name, expr)
}

fn parse_return_stmt(iter: &mut Peekable<Iter<Token>>) -> Stmt {
    consume(iter, TokType::KeywordReturn);
    let expr: Option<Expr> = if is_expr(iter) {
        Some(parse_expr(iter))
    } else {
        None
    };
    consume(iter, TokType::Semicolon);
    Stmt::Return(expr)
}

/// statement that invoke an expression, i.e function call
fn parse_expr_stmt(iter: &mut Peekable<Iter<Token>>) -> Stmt {
    let e = parse_expr(iter);
    consume(iter, TokType::Semicolon);
    Stmt::Expr(e)
}

fn is_expr(iter: &mut Peekable<Iter<Token>>) -> bool {
    is_int_const_expr(iter) || is_ref(iter)
}

fn parse_expr(iter: &mut Peekable<Iter<Token>>) -> Expr {
    if is_int_const_expr(iter) {
        parse_int_const_expr(iter)
    } else if is_ref(iter) {
        parse_ref_expr(iter)
    } else {
        panic!("expected expression but {:?}", iter.peek())
    }
}

fn is_int_const_expr(iter: &mut Peekable<Iter<Token>>) -> bool {
    match iter.peek() {
        Some(Token {
            tok: TokType::NumInt(_),
            loc: _,
        }) => true,
        _ => false,
    }
}

fn parse_int_const_expr(iter: &mut Peekable<Iter<Token>>) -> Expr {
    match iter.next() {
        Some(Token {
            tok: TokType::NumInt(v),
            loc: _,
        }) => Expr::IntConst(*v as i64),
        Some(t) => panic!("expected int constant but {}", t),
        None => panic!("unexpected EOF"),
    }
}

fn is_ref(iter: &mut Peekable<Iter<Token>>) -> bool {
    match iter.peek() {
        Some(Token {
            tok: TokType::ID(_),
            loc: _,
        }) => true,
        _ => false,
    }
}

/// parse function or variable call
///
/// TODO parse array index
fn parse_ref_expr(iter: &mut Peekable<Iter<Token>>) -> Expr {
    let name = parse_id(iter);
    match iter.peek() {
        Some(t) if t.tok == TokType::ParentOpen => parse_function_call_expr(iter, name),
        _ => Expr::VarRef(name),
    }
}

fn parse_function_call_expr(iter: &mut Peekable<Iter<Token>>, name: String) -> Expr {
    consume(iter, TokType::ParentOpen);
    let args = parse_arguments(iter);
    consume(iter, TokType::ParentClose);
    Expr::FunctionCall(name, args)
}

fn parse_arguments(iter: &mut Peekable<Iter<Token>>) -> Vec<Expr> {
    if is_expr(iter) {
        let mut vec: Vec<Expr> = Vec::new();
        vec.push(parse_expr(iter));
        loop {
            if is_peek_tok(iter, TokType::Comma) {
                consume_any(iter);
                vec.push(parse_expr(iter));
            } else {
                break;
            }
        }
        vec
    } else {
        Vec::with_capacity(0)
    }
}

fn is_data_type(tok: &Token) -> bool {
    has_value(parse_data_type(tok))
}

fn parse_data_type(tok: &Token) -> Option<DataType> {
    match tok.tok {
        TokType::KeywordInt => Some(DataType::Int),
        TokType::KeywordVoid => Some(DataType::Void),
        _ => None,
    }
}

fn parse_id(iter: &mut Peekable<Iter<Token>>) -> String {
    match iter.next() {
        Some(Token {
            tok: TokType::ID(s),
            loc: _,
        }) => s.to_string(),
        Some(t) => panic!("exepcted ID but {}", t),
        _ => panic!("unexpected EOF"),
    }
}

fn is_id(iter: &mut Peekable<Iter<Token>>) -> bool {
    match iter.peek() {
        Some(Token {
            tok: TokType::ID(_),
            loc: _,
        }) => true,
        _ => false,
    }
}

fn has_value<T>(opt: Option<T>) -> bool {
    match opt {
        Some(_) => true,
        _ => false,
    }
}

fn is_peek_tok(iter: &mut Peekable<Iter<Token>>, tok: TokType) -> bool {
    match iter.peek() {
        Some(Token { tok: t, loc: _ }) if *t == tok => true,
        _ => false,
    }
}

fn consume_any(iter: &mut Peekable<Iter<Token>>) {
    let _ = iter.next();
}

fn consume(iter: &mut Peekable<Iter<Token>>, tok: TokType) {
    let item = iter
        .next()
        .expect(format!("expected {} but EOF", tok).as_str());
    match item {
        Token { tok: t, loc: _ } if *t == tok => (),
        t => panic!("expected {} but {}", tok, t),
    }
}

enum ExprRefType {
    FunctionCall,
    ArrayIndex,
    VarRef,
}

#[cfg(test)]
mod test {
    use test_case::test_case;

    use crate::scan;

    use super::parse;

    #[test_case("int main() { return 1; }")]
    #[test_case("int main() { }")]
    #[test_case("void test() { return 1; }")]
    #[test_case("int main() { int a = 100; return 1; }")]
    #[test_case("void test() { int a = 3; return a; }")]
    #[test_case("void foo(int x, int y) {}")]
    #[test_case("void foo() { int a = undefined(x, 3); }")]
    #[test_case("void foo() { undefined(3); }")]
    fn pass_program(src: &str) {
        parse(scan(src));
    }

    #[test_case("main" => panics "unexpected identifier 'main' at 1:1")]
    #[test_case("int main" => panics "expected ; but EOF")]
    #[test_case("int test {" => panics "expected ; but {")]
    #[test_case("int test() {" => panics "unexpected EOF")]
    #[test_case("int main() { return 1 }" => panics "expected ; but }")]
    fn failed_program(src: &str) {
        parse(scan(src));
    }

    #[test_case("int g = 101; void foo() { int a = g;}")]
    #[test_case("int g = 101; void foo() { int g = 2; { int g = 3; }}")]
    fn parse_global(src: &str) {
        parse(scan(src));
    }

    // #[test_case("int main() { int a; a = 1; }")]
    // fn parse_stmt(src: &str) {
    //     parse(scan(src));
    // }
}

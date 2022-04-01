use quote::ToTokens;
use syn::{Expr, parse_quote};
use syn::ItemFn;
use syn::Stmt;

pub(crate) fn nop_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){nop}};
    return nop.block.stmts[0].clone();
}

pub(crate) fn else_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){else_stmt}};
    return nop.block.stmts[0].clone();
}
pub(crate) fn final_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){final_stmt}};
    return nop.block.stmts[0].clone();
}
pub(crate) fn start_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){start_stmt}};
    return nop.block.stmts[0].clone();
}
pub(crate) fn start_node_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){start_node_stmt}};
    return nop.block.stmts[0].clone();
}
pub(crate) fn end_node_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){end_node_stmt}};
    return nop.block.stmts[0].clone();
}

pub(crate) fn semi_token() -> syn::token::Semi {
    return syn::token::Semi::default();
}

fn is_co_expr_path(path: &syn::ExprPath) -> bool {
    let name = get_expr_path_name(path);
    return name == "co_yield" || name == "co_return";
}

fn get_expr_path_name(path: &syn::ExprPath) -> String {
    let res = path.attrs.is_empty()
        && path.qself.is_none()
        && path.path.leading_colon.is_none()
        && path.path.leading_colon.is_none()
        && path.path.segments.len() == 1;
    if res {
        if let Some(name) = path.path.segments.last() {
            let name = name.ident.to_string();
            return name;
        }
    }
    String::new()
}

pub(crate) fn is_co_yield_or_co_return_expr(expr: &syn::Expr) -> bool {
    match expr {
        Expr::Path(expr) => {
            return is_co_expr_path(expr);
        }
        Expr::Call(e) => {
            let res = e.attrs.is_empty();
            if res {
                match e.func.as_ref() {
                    Expr::Path(expr) => {
                        return is_co_expr_path(expr);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    false
}
pub(crate) fn is_yield_or_return(stmt: &syn::Stmt) -> bool {
    match stmt {
        Stmt::Expr(Expr::Path(expr))|Stmt::Semi(Expr::Path(expr), _) => {
            return is_co_expr_path(expr);
        }
        Stmt::Expr(Expr::Call(e))|Stmt::Semi(Expr::Call(e), _) => {
            let res = e.attrs.is_empty();
            if res {
                match e.func.as_ref() {
                    Expr::Path(expr) => {
                        return is_co_expr_path(expr);
                    }
                    _ => {}
                }
            }
        }
        Stmt::Semi(Expr::Return(_), _) => {
            return true;
        }
        Stmt::Expr(Expr::Return(_)) => {
            return true;
        }
        _ => {}
    }
    false
}

pub(crate) fn transform_stmt_to_string(stmt: &syn::Stmt) -> (String, bool) {
    let mut is_yield_or_return = false;
    let stmt_str = match &stmt {
        Stmt::Expr(Expr::Path(expr)) => {
            let name = get_expr_path_name(expr);
            if name == "co_return" || name == "co_yield" {
                // no semi
                return (String::from("return"), true);
            }
            stmt.to_token_stream().to_string()
        }
        Stmt::Semi(Expr::Path(expr), _) => {
            let name = get_expr_path_name(expr);
            if name == "co_return" || name == "co_yield" {
                return (String::from("return;"), true);
            }
            stmt.to_token_stream().to_string()
        }
        Stmt::Expr(Expr::Call(e)) => {
            let res = e.attrs.is_empty();
            if res {
                match e.func.as_ref() {
                    Expr::Path(expr) => {
                        let name = get_expr_path_name(expr);
                        if name == "co_return" || name == "co_yield" {
                            // no semi
                            return (
                                format!(
                                    "return {}",
                                    e.args.last().unwrap().to_token_stream().to_string()
                                ),
                                true,
                            );
                        }
                    }
                    _ => {}
                }
            }
            stmt.to_token_stream().to_string()
        }
        Stmt::Semi(Expr::Call(e), _) => {
            let res = e.attrs.is_empty();
            if res {
                match e.func.as_ref() {
                    Expr::Path(expr) => {
                        let name = get_expr_path_name(expr);
                        if name == "co_return" || name == "co_yield" {
                            return (
                                format!(
                                    "return {};",
                                    e.args.last().unwrap().to_token_stream().to_string()
                                ),
                                true,
                            );
                        }
                    }
                    _ => {}
                }
            }
            stmt.to_token_stream().to_string()
        }
        Stmt::Expr(Expr::Yield(e)) | Stmt::Semi(Expr::Yield(e), _) => {
            is_yield_or_return = true;
            if let Some(expr) = &e.expr {
                format!("{};", expr.to_token_stream().to_string())
            } else {
                format!(";")
            }
        }
        Stmt::Expr(Expr::Return(_))|Stmt::Semi(Expr::Return(_), _) => {
            is_yield_or_return = true;
            stmt.to_token_stream().to_string()
        }
        _ => stmt.to_token_stream().to_string(),
    };
    return (stmt_str, is_yield_or_return);
}

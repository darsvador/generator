use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use quote::ToTokens;
use syn::ItemFn;
use syn::Stmt;
use syn::{parse_quote, Expr};
use petgraph::visit::Dfs;

fn nop_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){;}};
    return nop.block.stmts[0].clone();
}

fn else_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){else_stmt}};
    return nop.block.stmts[0].clone();
}
fn final_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){final_stmt}};
    return nop.block.stmts[0].clone();
}
fn start_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){start_stmt}};
    return nop.block.stmts[0].clone();
}

fn proc_expr(g: &mut Graph<Stmt, Stmt>, expr: &syn::Expr, cur_idx: NodeIndex) -> NodeIndex {
    match expr {
        Expr::If(e) => {
            let end_idx = g.add_node(nop_stmt());
            let mut true_end_idx = g.add_node(nop_stmt());
            g.add_edge(cur_idx, true_end_idx, Stmt::Expr(e.cond.as_ref().clone()));
            for stmt in &e.then_branch.stmts {
                true_end_idx = proc_stmt(g, stmt, true_end_idx);
            }
            g.add_edge(true_end_idx, end_idx, nop_stmt());
            if let Some((_, cond)) = &e.else_branch {
                let mut false_end_idx = g.add_node(nop_stmt());
                g.add_edge(cur_idx, false_end_idx, else_stmt());
                false_end_idx = proc_expr(g, cond.as_ref(), false_end_idx);
                g.add_edge(false_end_idx, end_idx, nop_stmt());
            } else {
                g.add_edge(cur_idx, end_idx, else_stmt());
            }
            return end_idx;
        }
        _ => {
            let idx = g.add_node(Stmt::Expr(expr.clone()));
            g.add_edge(cur_idx, idx, nop_stmt());
            return idx;
        }
    }
}
fn proc_stmt(g: &mut Graph<Stmt, Stmt>, stmt: &syn::Stmt, cur_idx: NodeIndex) -> NodeIndex {
    match stmt {
        Stmt::Local(_) => {
            let idx = g.add_node(stmt.clone());
            g.add_edge(cur_idx, idx, nop_stmt());
            return idx;
        }
        Stmt::Item(_) => {
            panic!("we don't support item for now.");
        }
        Stmt::Expr(e) => {
            return proc_expr(g, &e, cur_idx);
        }
        Stmt::Semi(e, _) => {
            if !stmt.eq(&nop_stmt()) {
                return proc_expr(g, &e, cur_idx);
            } else {
                println!("found nop!");
            }
        }
    }
    cur_idx
}

fn main() {
    let mut g = Graph::<Stmt, Stmt>::new();
    let mut str_g=Graph::<String,String>::new();
    let f: ItemFn = parse_quote! {
    pub fn poll_read_decrypted<R>(
        &mut self,
        ctx: &mut Context<'_>,
        r: &mut R,
        dst: &mut [u8],
    ) -> Poll<io::Result<(usize)>>
    where
        R: AsyncRead + Unpin,
    {
    ;
    if wtf{
    f();
    yield return Poll::Pending;
    g();
    }else{
    let c=p();
    yield return Poll::Ready(c);
    q();
    }
    }
    };
    //println!("stmts len:{}", f.block.stmts.len());
    let mut cur_idx = g.add_node(start_stmt());
    for i in f.block.stmts {
        //println!("{:#?}", i.to_token_stream().to_string());
        cur_idx = proc_stmt(&mut g, &i, cur_idx);
    }
    let final_idx=g.add_node(final_stmt());
    g.add_edge(cur_idx,final_idx,nop_stmt());
    let g2=g.map(|_i,node|{node.to_token_stream().to_string()},|_,e|{e.to_token_stream().to_string()});
    let dot=Dot::with_config(&g2,&[]);
    println!("{:#?}", dot);
}

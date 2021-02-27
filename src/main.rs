use petgraph::dot::Dot;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use quote::ToTokens;
use syn::ItemFn;
use syn::Stmt;
use syn::{parse_quote, Expr};

fn nop_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){nop}};
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
fn start_node_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){start_node_stmt}};
    return nop.block.stmts[0].clone();
}
fn end_node_stmt() -> Stmt {
    let nop: ItemFn = parse_quote! {fn nop(){end_node_stmt}};
    return nop.block.stmts[0].clone();
}

struct LoopLabel {
    pub start_idx: NodeIndex,
    pub end_idx: NodeIndex,
    pub name: String,
}
impl LoopLabel {
    fn new(start_idx: NodeIndex, end_idx: NodeIndex, name: String) -> LoopLabel {
        LoopLabel {
            start_idx,
            end_idx,
            name,
        }
    }
}

pub trait CFG{
    fn add_cfg_edge(&mut self, a: NodeIndex, b: NodeIndex, stmt: Stmt);
}
impl CFG for Graph<Stmt,Stmt>{
    fn add_cfg_edge(&mut self, a: NodeIndex, b: NodeIndex, stmt: Stmt){
        if NodeIndex::end()!=a&&NodeIndex::end()!=b{
            self.add_edge(a, b, stmt);
        }
    }
}

fn proc_expr(
    g: &mut Graph<Stmt, Stmt>,
    expr: &syn::Expr,
    cur_idx: NodeIndex,
    loop_label_node_id: &mut Vec<LoopLabel>,
) -> NodeIndex {
    match expr {
        Expr::If(e) => {
            let end_idx = g.add_node(end_node_stmt());
            let mut true_end_idx = g.add_node(start_node_stmt());
            g.add_cfg_edge(cur_idx, true_end_idx, Stmt::Expr(e.cond.as_ref().clone()));
            for stmt in &e.then_branch.stmts {
                true_end_idx = proc_stmt(g, stmt, true_end_idx, loop_label_node_id);
            }
            g.add_cfg_edge(true_end_idx, end_idx, nop_stmt());
            if let Some((_, cond)) = &e.else_branch {
                let mut false_end_idx = g.add_node(nop_stmt());
                g.add_cfg_edge(cur_idx, false_end_idx, else_stmt());
                false_end_idx = proc_expr(g, cond.as_ref(), false_end_idx, loop_label_node_id);
                g.add_cfg_edge(false_end_idx, end_idx, nop_stmt());
            } else {
                g.add_cfg_edge(cur_idx, end_idx, else_stmt());
            }
            return end_idx;
        }
        Expr::Loop(e) => {
            let true_st_idx = g.add_node(start_node_stmt());
            let false_st_idx = g.add_node(end_node_stmt());
            g.add_cfg_edge(cur_idx, true_st_idx, nop_stmt());
            let mut true_end_idx = true_st_idx;
            if let Some(l) = &e.label {
                let label = l.name.to_token_stream().to_string();
                loop_label_node_id.push(LoopLabel::new(true_st_idx, false_st_idx, label));
            } else {
                loop_label_node_id.push(LoopLabel::new(
                    true_st_idx,
                    false_st_idx,
                    String::from(""),
                ));
            }
            for stmt in &e.body.stmts {
                true_end_idx = proc_stmt(g, stmt, true_end_idx, loop_label_node_id);
            }
            g.add_cfg_edge(true_end_idx, cur_idx, nop_stmt());
            loop_label_node_id.pop();
            return false_st_idx;
        }
        Expr::Continue(e)=>{
            if let Some(l) = &e.label {
                let break_label = l.to_token_stream().to_string();
                for l in loop_label_node_id {
                    if &l.name == &break_label {
                        g.add_cfg_edge(cur_idx, l.start_idx.clone(), nop_stmt());
                    }
                }
            } else {
                let jump_idx = loop_label_node_id.last().unwrap().start_idx;
                g.add_cfg_edge(cur_idx, jump_idx, nop_stmt());
            }
            return NodeIndex::end();
        }
        Expr::Break(e) => {
            if let Some(l) = &e.label {
                let break_label = l.to_token_stream().to_string();
                for l in loop_label_node_id {
                    if &l.name == &break_label {
                        g.add_cfg_edge(cur_idx, l.end_idx.clone(), nop_stmt());
                    }
                }
            } else {
                let jump_idx = loop_label_node_id.last().unwrap().end_idx;
                g.add_cfg_edge(cur_idx, jump_idx, nop_stmt());
            }
            return NodeIndex::end();
        }
        Expr::While(e) => {
            let true_st_idx = g.add_node(start_node_stmt());
            let false_st_idx = g.add_node(end_node_stmt());
            g.add_cfg_edge(cur_idx, true_st_idx, Stmt::Expr(e.cond.as_ref().clone()));
            let mut true_end_idx = true_st_idx;
            if let Some(l) = &e.label {
                let label = l.name.to_token_stream().to_string();
                loop_label_node_id.push(LoopLabel::new(true_st_idx, false_st_idx, label));
            } else {
                loop_label_node_id.push(LoopLabel::new(
                    true_st_idx,
                    false_st_idx,
                    String::from(""),
                ));
            }
            for stmt in &e.body.stmts {
                true_end_idx = proc_stmt(g, stmt, true_end_idx, loop_label_node_id);
            }
            g.add_cfg_edge(true_end_idx, cur_idx, nop_stmt());
            g.add_cfg_edge(cur_idx, false_st_idx, else_stmt());
            loop_label_node_id.pop();
            return false_st_idx;
        }
        Expr::Block(e) => {
            let mut cur_idx = cur_idx;
            for stmt in &e.block.stmts {
                cur_idx = proc_stmt(g, stmt, cur_idx, loop_label_node_id);
            }
            return cur_idx;
        }
        _ => {
            let idx = g.add_node(Stmt::Expr(expr.clone()));
            g.add_cfg_edge(cur_idx, idx, nop_stmt());
            return idx;
        }
    }
}

fn proc_stmt(
    g: &mut Graph<Stmt, Stmt>,
    stmt: &syn::Stmt,
    cur_idx: NodeIndex,
    loop_label_node_id: &mut Vec<LoopLabel>,
) -> NodeIndex {
    match stmt {
        Stmt::Local(_) => {
            let idx = g.add_node(stmt.clone());
            g.add_cfg_edge(cur_idx, idx, nop_stmt());
            return idx;
        }
        Stmt::Item(_) => {
            panic!("we don't support item for now.");
        }
        Stmt::Expr(e) => {
            return proc_expr(g, &e, cur_idx, loop_label_node_id);
        }
        Stmt::Semi(e, _) => {
            return proc_expr(g, &e, cur_idx, loop_label_node_id);
        }
    }
}

fn main() {
    let mut g = Graph::<Stmt, Stmt>::new();
    let mut loop_label_node_id = Vec::new();
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
                if cond1{
                    f();
                    yield return Poll::Pending;
                    g();
                }else{
                    let c=p();
                    yield return Poll::Ready(c);
                    q();
                }
                'outer: loop {
                    println!("Entered the outer loop");

                    'inner: loop {
                        println!("Entered the inner loop");

                        // This would break only the inner loop
                        //break;

                        // This breaks the outer loop
                        break 'outer;
                    }

                    println!("This point will never be reached");
                }
                loop{
                    if cond2{
                        continue;
                    } else{
                        break;
                    }
                }
                loop{
                    if cond3{
                        println!("cond3 is true");
                        continue;
                    } else{
                        break;
                    }
                }
                while not_done{
                    do1();
                    yield return Poll::Ready(c);
                    do2();
                    if cond4{
                        break;
                    }
                }
            }
    };
    //println!("stmts len:{}", f.block.stmts.len());
    let mut cur_idx = g.add_node(start_stmt());
    for i in f.block.stmts {
        //println!("{:#?}", i.to_token_stream().to_string());
        cur_idx = proc_stmt(&mut g, &i, cur_idx, &mut loop_label_node_id);
    }
    let final_idx = g.add_node(final_stmt());
    g.add_cfg_edge(cur_idx, final_idx, nop_stmt());
    let g2 = g.map(
        |_i, node| node.to_token_stream().to_string(),
        |_, e| e.to_token_stream().to_string(),
    );
    let dot = Dot::with_config(&g2, &[]);
    println!("{:#?}", dot);
}

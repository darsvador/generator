use petgraph::dot::Dot;
use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;
use petgraph::Graph;
use quote::ToTokens;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
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

fn semi_token() -> syn::token::Semi {
    return syn::token::Semi::default();
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

pub trait CFG {
    fn add_cfg_edge(&mut self, a: NodeIndex, b: NodeIndex, stmt: Stmt);
}
impl CFG for Graph<Stmt, Stmt> {
    fn add_cfg_edge(&mut self, a: NodeIndex, b: NodeIndex, stmt: Stmt) {
        if NodeIndex::end() != a && NodeIndex::end() != b {
            let mut incoming_neighbors =
                self.neighbors_directed(a, petgraph::EdgeDirection::Incoming);
            // 0 is start statement
            if incoming_neighbors.next().is_some() || a == 0.into() {
                self.add_edge(a, b, stmt);
            }
        }
    }
}

fn proc_expr(
    g: &mut Graph<Stmt, Stmt>,
    expr: &syn::Expr,
    cur_idx: NodeIndex,
    loop_label_node_id: &mut Vec<LoopLabel>,
    is_semi: bool,
) -> NodeIndex {
    let mut ret_idx = NodeIndex::end();
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
                false_end_idx =
                    proc_expr(g, cond.as_ref(), false_end_idx, loop_label_node_id, false);
                g.add_cfg_edge(false_end_idx, end_idx, nop_stmt());
            } else {
                g.add_cfg_edge(cur_idx, end_idx, else_stmt());
            }
            ret_idx = end_idx;
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
            ret_idx = false_st_idx;
        }
        Expr::Continue(e) => {
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
            ret_idx = false_st_idx;
        }
        Expr::Block(e) => {
            let mut cur_idx = cur_idx;
            for stmt in &e.block.stmts {
                cur_idx = proc_stmt(g, stmt, cur_idx, loop_label_node_id);
            }
            ret_idx = cur_idx;
        }
        _ => {
            let idx;
            if is_semi {
                idx = g.add_node(Stmt::Semi(expr.clone(), semi_token()));
            } else {
                idx = g.add_node(Stmt::Expr(expr.clone()));
            }
            g.add_cfg_edge(cur_idx, idx, nop_stmt());
            ret_idx = idx;
        }
    }
    ret_idx
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
            return proc_expr(g, &e, cur_idx, loop_label_node_id, false);
        }
        Stmt::Semi(e, _) => {
            return proc_expr(g, &e, cur_idx, loop_label_node_id, true);
        }
    }
}

fn figure_out_projections(g: &Graph<Stmt, Stmt>) -> HashMap<usize, usize> {
    let mut project_to_state: HashMap<usize, usize> = HashMap::new();
    let mut global_state = 0usize;
    let mut dfs = Dfs::new(g, 0.into());
    // project to a state
    // indegree >1
    // yield node's outgoing node
    // incoming edges have a edge which is not 'nop'
    project_to_state.insert(0, global_state);
    while let Some(node) = dfs.next(g) {
        let mut indegree = 0;
        let mut in_edge_not_nop = false;
        let mut parent_idx = usize::MAX;
        for succ in g.neighbors_directed(node, petgraph::EdgeDirection::Incoming) {
            indegree += 1;
            parent_idx = succ.index();
            if let Some(e) = g.find_edge(succ, node) {
                if g[e] != nop_stmt() {
                    in_edge_not_nop = true;
                }
            }
        }
        if indegree == 0 {
            // dead node or start_stmt node(0)
            continue;
        }
        if in_edge_not_nop || indegree > 1 {
            if !project_to_state.contains_key(&node.index()) {
                global_state += 1;
                project_to_state.insert(node.index(), global_state);
            }
        } else if indegree == 1 {
            if !project_to_state.contains_key(&node.index()) {
                project_to_state.insert(
                    node.index(),
                    project_to_state.get(&parent_idx).unwrap().clone(),
                );
            }
        }
        let mut is_yield_or_return = false;
        match &g[node] {
            Stmt::Expr(Expr::Yield(_))
            | Stmt::Semi(Expr::Yield(_), _)
            | Stmt::Semi(Expr::Return(_), _) => {
                is_yield_or_return = true;
            }
            _ => {}
        };
        if is_yield_or_return {
            for succ in g.neighbors(node) {
                if !project_to_state.contains_key(&succ.index()) {
                    global_state += 1;
                    project_to_state.insert(succ.index(), global_state);
                }
            }
        }
    }
    project_to_state
}

fn gen_state_machines(g: &Graph<Stmt, Stmt>, final_idx: NodeIndex) -> Box<syn::Block>{
    let mut set: HashSet<String> = HashSet::new();
    let state_name = String::from("state");
    set.insert(String::from("nop"));
    set.insert(String::from("start_stmt"));
    set.insert(String::from("final_stmt"));
    set.insert(String::from("start_node_stmt"));
    set.insert(String::from("end_node_stmt"));
    let else_stmt = String::from("else_stmt");
    let project_to_state: HashMap<usize, usize> = figure_out_projections(g);
    let mut discovered: HashSet<usize> = HashSet::new();
    let mut dfs = Dfs::new(g, 0.into());
    let mut loops = String::from(format!(
        "{{'genloop: loop{{ \nmatch {}\n{{ \n_=>{{\nbreak 'genloop;\n",
        state_name
    ));
    while let Some(node) = dfs.next(g) {
        let mut is_yield_or_return = false;
        let cur_state = project_to_state.get(&node.index()).unwrap();
        if !discovered.contains(&cur_state) {
            discovered.insert(cur_state.clone());
            loops.push_str(&format!("}}\n{}=>{{", cur_state));
        }
        let stmt_str: String = match &g[node] {
            Stmt::Expr(Expr::Yield(e)) | Stmt::Semi(Expr::Yield(e), _) => {
                is_yield_or_return = true;
                if let Some(expr) = &e.expr {
                    format!("{};", expr.to_token_stream().to_string())
                } else {
                    g[node].to_token_stream().to_string()
                }
            }
            Stmt::Semi(Expr::Return(_), _) => {
                is_yield_or_return = true;
                g[node].to_token_stream().to_string()
            }
            _ => g[node].to_token_stream().to_string(),
        };
        if !set.contains(&stmt_str) && !is_yield_or_return {
            loops.push_str(&stmt_str);
        } else if node == final_idx {
            // out of the loop
            loops.push_str(&format!("{}={};", state_name, g.node_count() + 1));
        }
        for succ in g.neighbors(node) {
            if let Some(e) = g.find_edge(node, succ) {
                let next_state = project_to_state.get(&succ.index()).unwrap();
                if g[e] != nop_stmt() {
                    let cond = g[e].to_token_stream().to_string();
                    if cond != else_stmt {
                        // if cond{state=next_state;continue 'genloop;}
                        loops.push_str(&format!(
                            "if {}{{ {}={};continue 'genloop;}}",
                            cond, state_name, next_state
                        ));
                    } else {
                        // state=next_state;
                        loops.push_str(&format!("{}={};", state_name, next_state));
                    }
                } else if is_yield_or_return {
                    loops.push_str(&format!("{}={};", state_name, next_state));
                    loops.push_str(&stmt_str);
                }
            }
        }
    }
    loops.push_str(&format!("}}}}}}}}"));
    Box::new(syn::parse_str(&loops).unwrap())
}

fn main() {
    let mut g = Graph::<Stmt, Stmt>::new();
    let mut loop_label_node_id = Vec::new();
    let mut f: ItemFn = parse_quote! {
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
                    yield return Poll::Ready(Ok(c));
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
                    let c=do1();
                    yield return Poll::Ready(Ok(c));
                    do2();
                    if cond4{
                        break;
                    }
                }
            }
    };
    let mut cur_idx = g.add_node(start_stmt());
    for i in &f.block.stmts {
        cur_idx = proc_stmt(&mut g, &i, cur_idx, &mut loop_label_node_id);
    }
    let final_idx = g.add_node(final_stmt());
    g.add_cfg_edge(cur_idx, final_idx, nop_stmt());
    let states = figure_out_projections(&g);
    let g2 = g.map(
        |_i, node| node.to_token_stream().to_string(),
        |_, e| e.to_token_stream().to_string(),
    );
    let g3 = g.map(
        |i, _node| {
            if let Some(&v) = states.get(&i.index()) {
                return v.to_string();
            }
            String::from("invalid")
        },
        |_, e| e.to_token_stream().to_string(),
    );
    f.block=gen_state_machines(&g, final_idx);
    println!("{}", f.to_token_stream().to_string());
    // generate contrl flow graph
    let dot = Dot::with_config(&g2, &[]);
    let mut cfg_dot = fs::File::create("cfg.dot").unwrap();
    cfg_dot.write_all(format!("{:#?}", dot).as_bytes()).unwrap();
    let dot = Dot::with_config(&g3, &[]);
    let mut cfg_dot = fs::File::create("cfg_state.dot").unwrap();
    cfg_dot.write_all(format!("{:#?}", dot).as_bytes()).unwrap();
}

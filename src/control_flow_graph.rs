use crate::stmt::{
    else_stmt, end_node_stmt, final_stmt, is_yield_or_return, nop_stmt, semi_token,
    start_node_stmt, start_stmt,
};
use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;
use petgraph::Graph;
use quote::ToTokens;
use std::collections::HashMap;
use syn::Expr;
use syn::Stmt;
pub struct LoopLabel {
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
    fn new_cfg_graph() -> (Self, NodeIndex)
    where
        Self: Sized;
    fn add_cfg_edge(&mut self, a: NodeIndex, b: NodeIndex, stmt: Stmt);
    fn proc_stmt(
        &mut self,
        stmt: &syn::Stmt,
        cur_idx: NodeIndex,
        final_idx: NodeIndex,
        loop_label_node_id: &mut Vec<LoopLabel>,
    ) -> NodeIndex;
    fn proc_expr(
        &mut self,
        expr: &syn::Expr,
        cur_idx: NodeIndex,
        final_idx: NodeIndex,
        loop_label_node_id: &mut Vec<LoopLabel>,
        is_semi: bool,
    ) -> NodeIndex;
    fn figure_out_projections(&self) -> HashMap<usize, usize>;
}
impl CFG for Graph<Stmt, Stmt> {
    fn new_cfg_graph() -> (Self, NodeIndex) {
        let mut g = Graph::<Stmt, Stmt>::new();
        g.add_node(start_stmt());
        let final_idx = g.add_node(final_stmt());
        (g, final_idx)
    }

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
    fn proc_stmt(
        &mut self,
        stmt: &syn::Stmt,
        cur_idx: NodeIndex,
        final_idx: NodeIndex,
        loop_label_node_id: &mut Vec<LoopLabel>,
    ) -> NodeIndex {
        match stmt {
            Stmt::Local(_) => {
                let idx = self.add_node(stmt.clone());
                self.add_cfg_edge(cur_idx, idx, nop_stmt());
                return idx;
            }
            Stmt::Item(_) => {
                panic!("we don't support item for now.");
            }
            Stmt::Expr(e) => {
                return self.proc_expr(&e, cur_idx, final_idx, loop_label_node_id, false);
            }
            Stmt::Semi(e, _) => {
                return self.proc_expr(&e, cur_idx, final_idx, loop_label_node_id, true);
            }
        }
    }

    fn proc_expr(
        &mut self,
        expr: &syn::Expr,
        cur_idx: NodeIndex,
        final_idx: NodeIndex,
        loop_label_node_id: &mut Vec<LoopLabel>,
        is_semi: bool,
    ) -> NodeIndex {
        let mut ret_idx = NodeIndex::end();
        match expr {
            Expr::If(e) => {
                let end_idx = self.add_node(end_node_stmt());
                let mut true_end_idx = self.add_node(start_node_stmt());
                self.add_cfg_edge(cur_idx, true_end_idx, Stmt::Expr(e.cond.as_ref().clone()));
                for stmt in &e.then_branch.stmts {
                    true_end_idx =
                        self.proc_stmt(stmt, true_end_idx, final_idx, loop_label_node_id);
                }
                self.add_cfg_edge(true_end_idx, end_idx, nop_stmt());
                if let Some((_, cond)) = &e.else_branch {
                    let mut false_end_idx = self.add_node(nop_stmt());
                    self.add_cfg_edge(cur_idx, false_end_idx, else_stmt());
                    false_end_idx = self.proc_expr(
                        cond.as_ref(),
                        false_end_idx,
                        final_idx,
                        loop_label_node_id,
                        false,
                    );
                    self.add_cfg_edge(false_end_idx, end_idx, nop_stmt());
                } else {
                    self.add_cfg_edge(cur_idx, end_idx, else_stmt());
                }
                ret_idx = end_idx;
            }
            Expr::Loop(e) => {
                let before_enter_loop_idx = self.add_node(nop_stmt());
                let true_st_idx = self.add_node(start_node_stmt());
                let false_st_idx = self.add_node(end_node_stmt());
                self.add_cfg_edge(cur_idx, before_enter_loop_idx, nop_stmt());
                self.add_cfg_edge(before_enter_loop_idx, true_st_idx, nop_stmt());
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
                    true_end_idx =
                        self.proc_stmt(stmt, true_end_idx, final_idx, loop_label_node_id);
                }
                self.add_cfg_edge(true_end_idx, before_enter_loop_idx, nop_stmt());
                loop_label_node_id.pop();
                ret_idx = false_st_idx;
            }
            Expr::Continue(e) => {
                if let Some(l) = &e.label {
                    let break_label = l.to_token_stream().to_string();
                    for l in loop_label_node_id {
                        if &l.name == &break_label {
                            self.add_cfg_edge(cur_idx, l.start_idx.clone(), nop_stmt());
                        }
                    }
                } else {
                    let jump_idx = loop_label_node_id.last().unwrap().start_idx;
                    self.add_cfg_edge(cur_idx, jump_idx, nop_stmt());
                }
            }
            Expr::Break(e) => {
                if let Some(l) = &e.label {
                    let break_label = l.to_token_stream().to_string();
                    for l in loop_label_node_id {
                        if &l.name == &break_label {
                            self.add_cfg_edge(cur_idx, l.end_idx.clone(), nop_stmt());
                        }
                    }
                } else {
                    let jump_idx = loop_label_node_id.last().unwrap().end_idx;
                    self.add_cfg_edge(cur_idx, jump_idx, nop_stmt());
                }
            }
            Expr::While(e) => {
                let before_enter_while_idx = self.add_node(nop_stmt());
                let true_st_idx = self.add_node(start_node_stmt());
                let false_st_idx = self.add_node(end_node_stmt());
                self.add_cfg_edge(cur_idx, before_enter_while_idx, nop_stmt());
                self.add_cfg_edge(
                    before_enter_while_idx,
                    true_st_idx,
                    Stmt::Expr(e.cond.as_ref().clone()),
                );
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
                    true_end_idx =
                        self.proc_stmt(stmt, true_end_idx, final_idx, loop_label_node_id);
                }
                self.add_cfg_edge(true_end_idx, before_enter_while_idx, nop_stmt());
                self.add_cfg_edge(before_enter_while_idx, false_st_idx, else_stmt());
                loop_label_node_id.pop();
                ret_idx = false_st_idx;
            }
            Expr::Return(_) => {
                let idx;
                if is_semi {
                    idx = self.add_node(Stmt::Semi(expr.clone(), semi_token()));
                } else {
                    idx = self.add_node(Stmt::Expr(expr.clone()));
                }
                self.add_cfg_edge(cur_idx, idx, nop_stmt());
                self.add_cfg_edge(idx, final_idx, nop_stmt());
            }
            Expr::Block(e) => {
                let mut cur_idx = cur_idx;
                for stmt in &e.block.stmts {
                    cur_idx = self.proc_stmt(stmt, cur_idx, final_idx, loop_label_node_id);
                }
                ret_idx = cur_idx;
            }
            _ => {
                let idx;
                if is_semi {
                    idx = self.add_node(Stmt::Semi(expr.clone(), semi_token()));
                } else {
                    idx = self.add_node(Stmt::Expr(expr.clone()));
                }
                self.add_cfg_edge(cur_idx, idx, nop_stmt());
                ret_idx = idx;
            }
        }
        ret_idx
    }

    fn figure_out_projections(&self) -> HashMap<usize, usize> {
        let mut project_to_state: HashMap<usize, usize> = HashMap::new();
        let mut global_state = 0usize;
        let mut dfs = Dfs::new(self, 0.into());
        // project to a state
        // indegree >1
        // yield node's outgoing node
        // incoming edges have a edge which is not 'nop'
        project_to_state.insert(0, global_state);
        while let Some(node) = dfs.next(self) {
            let mut indegree = 0;
            let mut in_edge_not_nop = false;
            let mut parent_idx = usize::MAX;
            for succ in self.neighbors_directed(node, petgraph::EdgeDirection::Incoming) {
                indegree += 1;
                parent_idx = succ.index();
                if let Some(e) = self.find_edge(succ, node) {
                    if self[e] != nop_stmt() {
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
            let is_yield_or_return = is_yield_or_return(&self[node]);
            if is_yield_or_return {
                for succ in self.neighbors(node) {
                    if !project_to_state.contains_key(&succ.index()) {
                        global_state += 1;
                        project_to_state.insert(succ.index(), global_state);
                    }
                }
            }
        }
        project_to_state
    }
}

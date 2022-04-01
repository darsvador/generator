use crate::stmt::{else_stmt, end_node_stmt, final_stmt, is_co_yield_or_co_return_expr, is_yield_or_return, nop_stmt, semi_token, start_node_stmt, start_stmt};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::Expr;
use syn::Stmt;

pub struct LoopLabel {
    pub start_idx: u32,
    pub end_idx: u32,
    pub name: String,
}
impl LoopLabel {
    pub(crate) fn new(start_idx: u32, end_idx: u32, name: String) -> LoopLabel {
        LoopLabel {
            start_idx,
            end_idx,
            name,
        }
    }
}

pub trait CFG {
    fn new_cfg_graph() -> (Self, u32)
    where
        Self: Sized;
    fn add_cfg_edge(&mut self, a: u32, b: u32, stmt: Stmt);
    fn proc_stmt(
        &mut self,
        stmt: &syn::Stmt,
        cur_idx: u32,
        final_idx: u32,
        loop_label_node_id: &mut Vec<LoopLabel>,
    ) -> u32;
    fn proc_expr(
        &mut self,
        expr: &syn::Expr,
        cur_idx: u32,
        final_idx: u32,
        loop_label_node_id: &mut Vec<LoopLabel>,
        is_semi: bool,
    ) -> u32;
    fn figure_out_projections(&self) -> HashMap<usize, usize>;
}

pub struct Node {
    pub(crate) val: Stmt,
    pub(crate) h: u32,
}

pub struct InDegree {
    d: u32,
    input_edges_contain_not_no_nop_stmt: bool,
}

impl InDegree {
    pub fn new() -> InDegree {
        InDegree {
            d: 0,
            input_edges_contain_not_no_nop_stmt: false,
        }
    }
}

pub struct CFGraph {
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Stmt>,
    pub(crate) e: Vec<u32>,
    pub(crate) ne: Vec<u32>,
    pub(crate) in_degree: Vec<InDegree>,
}

impl CFGraph {
    pub fn new() -> CFGraph {
        CFGraph {
            nodes: vec![],
            edges: vec![],
            e: vec![],
            ne: vec![],
            in_degree: vec![],
        }
    }
    pub fn add_node(&mut self, node: Stmt) -> u32 {
        let tmp = self.nodes.len();
        self.nodes.push(Node {
            val: node,
            h: u32::MAX,
        });
        self.in_degree.push(InDegree::new());
        tmp as u32
    }

    pub fn add_edge(&mut self, start_node: u32, end_node: u32, weight: Stmt) -> u32 {
        if !(start_node < self.nodes.len() as u32 && end_node < self.nodes.len() as u32) {
            return u32::MAX;
        }
        let tmp = self.edges.len() as u32;
        self.e.push(end_node);
        self.ne.push(self.nodes[start_node as usize].h);
        self.nodes[start_node as usize].h = tmp;
        self.in_degree[end_node as usize].d += 1;
        if weight != nop_stmt() {
            self.in_degree[end_node as usize].input_edges_contain_not_no_nop_stmt = true;
        }
        self.edges.push(weight);
        tmp
    }
}

impl CFG for CFGraph {
    fn new_cfg_graph() -> (Self, u32) {
        let mut g = CFGraph::new();
        g.add_node(start_stmt());
        let final_idx = g.add_node(final_stmt());
        (g, final_idx)
    }

    fn add_cfg_edge(&mut self, a: u32, b: u32, stmt: Stmt) {
        if u32::MAX != a && u32::MAX != b {
            if a == 0 || self.in_degree[a as usize].d != 0 {
                self.add_edge(a, b, stmt);
            }
        }
    }
    fn proc_stmt(
        &mut self,
        stmt: &syn::Stmt,
        cur_idx: u32,
        final_idx: u32,
        loop_label_node_id: &mut Vec<LoopLabel>,
    ) -> u32 {
        #[cfg(feature = "co_await")]
        {
            if crate::stmt::is_co_await_stmt(stmt){
                let new_stmt = crate::stmt::transform_co_await_stmt(stmt);
                return self.proc_stmt(&new_stmt,cur_idx,final_idx,loop_label_node_id);
            }
        }
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
        cur_idx: u32,
        final_idx: u32,
        loop_label_node_id: &mut Vec<LoopLabel>,
        is_semi: bool,
    ) -> u32 {
        let mut ret_idx = u32::MAX;
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
                if is_co_yield_or_co_return_expr(expr) {
                    let idx;
                    if is_semi {
                        idx = self.add_node(Stmt::Semi(expr.clone(), semi_token()));
                    } else {
                        idx = self.add_node(Stmt::Expr(expr.clone()));
                    }
                    self.add_cfg_edge(cur_idx, idx, nop_stmt());
                    let end_st_idx = self.add_node(end_node_stmt());
                    self.add_cfg_edge(idx, end_st_idx, nop_stmt());
                    ret_idx = end_st_idx;
                } else {
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
        }
        ret_idx
    }

    fn figure_out_projections(&self) -> HashMap<usize, usize> {
        let mut project_to_state: HashMap<usize, usize> = HashMap::new();
        let mut global_state = 0usize;
        let mut q = Vec::new();
        // project to a state
        // indegree >1
        // yield node's outgoing node
        // incoming edges have a edge which is not 'nop'
        project_to_state.insert(0, global_state);
        q.push((0u32, 0u32));
        let mut visit_set = HashSet::new();
        while let Some((cur, par)) = q.pop() {
            if !visit_set.contains(&cur) {
                visit_set.insert(cur);
                let cur = cur as usize;
                let par = par as usize;
                if self.in_degree[cur].d > 1
                    || self.in_degree[cur].input_edges_contain_not_no_nop_stmt
                {
                    if !project_to_state.contains_key(&cur) {
                        global_state += 1;
                        project_to_state.insert(cur, global_state);
                    }
                } else if self.in_degree[cur].d == 1 {
                    if !project_to_state.contains_key(&cur) {
                        project_to_state.insert(cur, project_to_state.get(&par).unwrap().clone());
                    }
                }
                let is_yield_or_return = is_yield_or_return(&self.nodes[cur].val);
                let mut i = self.nodes[cur].h as usize;
                while i as u32 != u32::MAX {
                    let next_node = self.e[i] as usize;
                    if is_yield_or_return && !project_to_state.contains_key(&next_node) {
                        global_state += 1;
                        project_to_state.insert(next_node, global_state);
                    }
                    q.push((next_node as u32, cur as u32));
                    i = self.ne[i] as usize;
                }
            }
            let mut i = self.nodes[cur as usize].h as usize;
            while i as u32 != u32::MAX {
                let next_node = self.e[i];
                if !visit_set.contains(&next_node) {
                    q.push((next_node as u32, cur as u32));
                }
                i = self.ne[i] as usize;
            }
        }
        project_to_state
    }
}

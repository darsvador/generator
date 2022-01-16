#![allow(dead_code)]
use crate::control_flow_graph::CFGraph;
use crate::control_flow_graph::CFG;
use crate::stmt::{nop_stmt, transform_stmt_to_string};
use quote::ToTokens;
use std::collections::HashMap;
use std::collections::HashSet;
use syn::ItemFn;

pub struct Generator {
    cfg_graph: CFGraph,
    final_node_idx: u32,
    state_projections: HashMap<usize, usize>,
    predefined_stmt: HashSet<String>,
}

impl Generator {
    pub fn new() -> Generator {
        let (g, final_idx) = CFGraph::new_cfg_graph();
        let mut predefined_stmt: HashSet<String> = HashSet::new();
        predefined_stmt.insert(String::from("nop"));
        predefined_stmt.insert(String::from("start_stmt"));
        predefined_stmt.insert(String::from("final_stmt"));
        predefined_stmt.insert(String::from("start_node_stmt"));
        predefined_stmt.insert(String::from("end_node_stmt"));
        Generator {
            cfg_graph: g,
            final_node_idx: final_idx,
            state_projections: HashMap::new(),
            predefined_stmt,
        }
    }

    pub fn gen_state_machines_tokenstream(
        &mut self,
        mut function: ItemFn,
        state_name: &str,
        return_default_value: &str,
    ) -> proc_macro2::TokenStream {
        let mut cur_idx = 0u32;
        let mut loop_label_node_id = Vec::new();
        for i in &function.block.stmts {
            cur_idx =
                self.cfg_graph
                    .proc_stmt(&i, cur_idx, self.final_node_idx, &mut loop_label_node_id);
        }
        self.cfg_graph
            .add_cfg_edge(cur_idx, self.final_node_idx, nop_stmt());
        println!("add cfg edge successful!");

        self.state_projections = self.cfg_graph.figure_out_projections();
        function.block = self.gen_state_machines(state_name, return_default_value);
        function.to_token_stream()
    }

    pub fn get_cfg_state_graph(&self) -> String {
        if self.state_projections.is_empty() {
            return String::new();
        }
        let mut dot_string = String::new();
        use std::fmt::Write;
        writeln!(dot_string, "digraph{{").unwrap();
        for (idx, node) in self.cfg_graph.nodes.iter().enumerate() {
            writeln!(
                dot_string,
                "{} [label=\"state:{}, {} \"]",
                idx,
                if let Some(v) = self.state_projections.get(&idx) {
                    v.to_string()
                } else {
                    String::from("invalid")
                },
                node.val.to_token_stream().to_string().replace("\"", "\\\"")
            )
            .unwrap();
        }
        for (idx, node) in self.cfg_graph.nodes.iter().enumerate() {
            let mut i = node.h as usize;
            while i as u32 != u32::MAX {
                let next_node = self.cfg_graph.e[i];
                let e = &self.cfg_graph.edges[i];
                writeln!(
                    dot_string,
                    "{} -> {} [label=\"{}\"]",
                    idx,
                    next_node,
                    e.to_token_stream().to_string().replace("\"", "\\\"")
                )
                .unwrap();
                i = self.cfg_graph.ne[i] as usize;
            }
        }
        writeln!(dot_string, "}}").unwrap();
        dot_string
    }

    fn gen_state_machines(&self, state_name: &str, return_default_value: &str) -> Box<syn::Block> {
        let state_name = format!("{}", state_name);
        let else_stmt = String::from("else_stmt");
        let project_to_state: &HashMap<usize, usize> = &self.state_projections;
        let mut discovered: HashSet<usize> = HashSet::new();
        let mut loops = String::from(format!(
            "{{'genloop: loop{{ \nmatch {}\n{{ \n{}=>{{\nbreak 'genloop;\n",
            state_name,
            self.cfg_graph.nodes.len() + 1
        ));
        let mut q = Vec::new();
        q.push(0u32);
        let mut visit_set = HashSet::new();
        while let Some(node) = q.pop() {
            if !visit_set.contains(&node) {
                visit_set.insert(node);
                let node = node as usize;
                let (stmt_str, is_yield_or_return) =
                    transform_stmt_to_string(&self.cfg_graph.nodes[node].val);
                let cur_state = project_to_state.get(&node).unwrap();
                if !discovered.contains(&cur_state) {
                    discovered.insert(cur_state.clone());
                    loops.push_str(&format!("}}\n{}=>{{", cur_state));
                }
                let is_predefined_stmt: bool = self.predefined_stmt.contains(&stmt_str);
                if !is_predefined_stmt && !is_yield_or_return {
                    loops.push_str(&stmt_str);
                } else if node as u32 == self.final_node_idx {
                    // out of the loop
                    loops.push_str(&format!(
                        "{}={};",
                        state_name,
                        self.cfg_graph.nodes.len() + 1
                    ));
                }
                let mut i = self.cfg_graph.nodes[node].h;
                while i != u32::MAX {
                    let next_node = self.cfg_graph.e[i as usize] as usize;
                    let e = self.cfg_graph.edges[i as usize].clone();
                    let next_state = project_to_state.get(&next_node).unwrap();
                    if e != nop_stmt() {
                        assert_ne!(next_state, cur_state);
                        let cond = e.to_token_stream().to_string();
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
                    } else if next_state != cur_state {
                        loops.push_str(&format!("{}={};", state_name, next_state));
                    }
                    i = self.cfg_graph.ne[i as usize];
                }
            }
            let mut i = self.cfg_graph.nodes[node as usize].h;
            while i != u32::MAX {
                let next_node = self.cfg_graph.e[i as usize];
                if !visit_set.contains(&next_node) {
                    q.push(next_node);
                }
                i = self.cfg_graph.ne[i as usize];
            }
        }
        if return_default_value.is_empty() {
            loops.push_str(&format!("}} _=>{{ break 'genloop;}} }}}}}}"));
        } else {
            loops.push_str(&format!(
                "}} _=>{{ break 'genloop;}} }}}} return {};}}",
                return_default_value
            ));
        }
        Box::new(syn::parse_str(&loops).unwrap())
    }
}

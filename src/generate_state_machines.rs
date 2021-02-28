#![allow(dead_code)]
use crate::control_flow_graph::CFG;
use crate::stmt::{nop_stmt, transform_stmt_to_string};
use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;
use petgraph::Graph;
use quote::ToTokens;
use std::collections::HashMap;
use std::collections::HashSet;
use syn::{ItemFn, Stmt};

pub struct Generator {
    cfg_graph: Graph<Stmt, Stmt>,
    final_node_idx: NodeIndex,
    state_projections: HashMap<usize, usize>,
    predefined_stmt: HashSet<String>,
}

impl Generator {
    pub fn new() -> Generator {
        let (g, final_idx) = Graph::<Stmt, Stmt>::new_cfg_graph();
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
        return_default_value:&str
    ) -> proc_macro2::TokenStream {
        let mut cur_idx = 0.into();
        let mut loop_label_node_id = Vec::new();
        for i in &function.block.stmts {
            cur_idx =
                self.cfg_graph
                    .proc_stmt(&i, cur_idx, self.final_node_idx, &mut loop_label_node_id);
        }
        self.cfg_graph
            .add_cfg_edge(cur_idx, self.final_node_idx, nop_stmt());
        self.state_projections = self.cfg_graph.figure_out_projections();
        function.block = self.gen_state_machines(state_name,return_default_value);
        function.to_token_stream()
    }

    pub fn get_cfg_graph(&self) -> Graph<String, String> {
        self.cfg_graph.map(
            |_i, node| node.to_token_stream().to_string(),
            |_, e| e.to_token_stream().to_string(),
        )
    }
    pub fn get_cfg_state_graph(&self) -> Graph<String, String> {
        if self.state_projections.is_empty() {
            return Graph::new();
        }
        self.cfg_graph.map(
            |i, _node| {
                if let Some(&v) = self.state_projections.get(&i.index()) {
                    return v.to_string();
                }
                String::from("invalid")
            },
            |_, e| e.to_token_stream().to_string(),
        )
    }

    fn gen_state_machines(&self, state_name: &str,return_default_value:&str) -> Box<syn::Block> {
        let state_name = format!("self.{}", state_name);
        let else_stmt = String::from("else_stmt");
        let project_to_state: &HashMap<usize, usize> = &self.state_projections;
        let mut discovered: HashSet<usize> = HashSet::new();
        let mut dfs = Dfs::new(&self.cfg_graph, 0.into());
        let mut loops = String::from(format!(
            "{{'genloop: loop{{ \nmatch {}\n{{ \n{}=>{{\nbreak 'genloop;\n",
            state_name,
            self.cfg_graph.node_count() + 1
        ));
        while let Some(node) = dfs.next(&self.cfg_graph) {
            let (stmt_str, is_yield_or_return) = transform_stmt_to_string(&self.cfg_graph[node]);
            let cur_state = project_to_state.get(&node.index()).unwrap();
            if !discovered.contains(&cur_state) {
                discovered.insert(cur_state.clone());
                loops.push_str(&format!("}}\n{}=>{{", cur_state));
            }
            if !self.predefined_stmt.contains(&stmt_str) && !is_yield_or_return {
                loops.push_str(&stmt_str);
            } else if node == self.final_node_idx {
                // out of the loop
                loops.push_str(&format!(
                    "{}={};",
                    state_name,
                    self.cfg_graph.node_count() + 1
                ));
            }
            for succ in self.cfg_graph.neighbors(node) {
                if let Some(e) = self.cfg_graph.find_edge(node, succ) {
                    let next_state = project_to_state.get(&succ.index()).unwrap();
                    if self.cfg_graph[e] != nop_stmt() {
                        let cond = self.cfg_graph[e].to_token_stream().to_string();
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
                }
            }
        }
        if return_default_value.is_empty(){
            loops.push_str(&format!("}} _=>{{ break 'genloop;}} }}}}}}"));
        } else {
            loops.push_str(&format!("}} _=>{{ break 'genloop;}} }}}} return {};}}",return_default_value));
        }
        Box::new(syn::parse_str(&loops).unwrap())
    }
}

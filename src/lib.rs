mod attr;
mod control_flow_graph;
mod generate_state_machines;
mod stmt;
mod test;

use crate::attr::GentianAttr;
use generate_state_machines::Generator;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::ItemFn;

#[proc_macro_attribute]
pub fn gentian(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let expanded = transform_to_state_machine(input);
    TokenStream::from(expanded)
}

fn transform_to_state_machine(mut input: syn::ItemFn) -> proc_macro2::TokenStream {
    let mut attrs: Option<GentianAttr> = None;
    if !input.attrs.is_empty() {
        if let Ok(a) = GentianAttr::try_from_attributes(&input.attrs) {
            attrs = a;
        }
    }
    let state_name: String;
    let ret_val: String;
    if let Some(attr) = &attrs {
        state_name = attr.get_state_name();
        ret_val = attr.get_ret_val();
    } else {
        state_name = "self.state".to_string();
        ret_val = String::new();
    }
    input.attrs.clear();
    if ret_val.is_empty() {
        println!("[gentian] found function state name: {}.", state_name);
    } else {
        println!(
            "[gentian] found function state name: {}, function default return value is: {}.",
            state_name, ret_val
        );
    }
    let mut generator = Generator::new();
    let expanded = generator.gen_state_machines_tokenstream(input, &state_name, &ret_val);
    return expanded;
}

mod control_flow_graph;
mod generate_state_machines;
mod stmt;
mod test;

use generate_state_machines::Generator;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::ItemFn;

#[proc_macro_attribute]
pub fn state_machine_generator(args: TokenStream, input: TokenStream) -> TokenStream {
    if args.is_empty() {
        return input;
    }
    let input = parse_macro_input!(input as ItemFn);
    let mut generator = Generator::new();
    let mut iter = args.into_iter();
    let state_name = iter.next().unwrap().to_string();
    let args: Vec<String> = iter.skip(1).map(|x| x.to_string()).collect();
    let expanded =
        generator.gen_state_machines_tokenstream(input, state_name.as_str(), &args.join(""));
    TokenStream::from(expanded)
}

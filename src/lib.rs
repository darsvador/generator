//! gentian is a proc macro that transforms generators to state machines.
//! Currently it supports loop statements, while statements, if statements, and the extended syntax for using `co_yield` and `co_return` and `return` in these statements.
//! # `gentian_attr` attribute of a function
//!  It has two kinds of parameters,
//!  * `state` represents the state currently used to maintain the automaton.
//!  * `ret_val` represents the default return value of the function, which is usually used for the result returned by calling again after the state machine ends.
//!
//! # `co_yield` or `co_return` statement
//!  This divides into three logical steps:
//!  * `co_yield` or `co_return` save the current state of the coroutine.
//!  * The resume point is defined immediately following the statement.
//!  * Same as rust `return` semantics, it returns from the function immediately.
//!
//! # `co_await` statement
//!  It's a syntax sugar for `co_yield` or `co_return` with `std::task::Poll`.
//!  ````ignore
//!  co_await(some_poll_func());
//!  ````
//!  is equivalent to
//!  ````ignore
//!  loop{
//!    let tmp=some_poll_func();
//!    if tmp.is_ready(){
//!        break;
//!    }
//!    co_yield(Poll::Pending);
//!  }
//!  ````
//!  This divides into two logical steps:
//!  * `co_await` save the current state of the coroutine.
//!  * The resume point is defined immediately following the statement and if and only if the waited poll function is ready.
//!
//! # `return` statement
//!  This type of statement divides into two logical steps:
//!  * `return` sets the coroutine state to indicate termination.
//!  * Same as rust `return` semantics, it returns from the function immediately.
//!  * When the function is called again, it returns the default return value (`ret_val`), or does nothing which means the function has no return value.
//!
//! # Example
//! The following code demonstrates the use of generators with and without a return value.
//! ````rust
//! use gentian::gentian;
//!
//! #[cfg(test)]
//! struct MyGenerator {
//!     my_state_1: usize,
//!     pub my_state_2: usize,
//!     pub num: u32,
//!     pub num1: u32,
//! }
//!
//! #[cfg(test)]
//! impl MyGenerator {
//!     pub fn new() -> MyGenerator {
//!         MyGenerator {
//!             my_state_1: 0,
//!             my_state_2: 0,
//!             num: 0,
//!             num1: 0,
//!         }
//!     }
//!
//!     #[gentian]
//!     #[gentian_attr(state=self.my_state_1)]
//!     pub fn test_simple(&mut self) {
//!         loop {
//!             println!("Hello, ");
//!             //co_yield;
//!             while self.num1 < 99 {
//!                 println!("Generator{}", self.num1);
//!                 self.num1 += 1;
//!                 co_yield;
//!             }
//!             return;
//!         }
//!     }
//!
//!     // state_name , return_default_value
//!     #[gentian]
//!     #[gentian_attr(state=self.my_state_2,ret_val=0u32)]
//!     pub fn get_odd(&mut self) -> u32 {
//!         loop {
//!             if self.num % 2 == 1 {
//!                 co_yield(self.num);
//!             }
//!             self.num += 1;
//!         }
//!     }
//! }
//!
//! #[test]
//! fn test_generator_proc_macro() {
//!     let mut gen = MyGenerator::new();
//!     gen.test_simple(); // print Hello,
//!     for _ in 0..200 {
//!         gen.test_simple(); // only print 99 times `Generator`
//!     }
//!     gen.test_simple(); // print nothing
//!     assert_eq!(gen.num1, 99);
//!     for i in (1u32..1000).step_by(2) {
//!         assert_eq!(gen.get_odd(), i);
//!     }
//! }
//!
//! ````
//!
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
        println!(
            "[gentian] found function `{}` state name: {}.",
            input.sig.ident, state_name
        );
    } else {
        println!(
            "[gentian] found function `{}` state name: {}, function default return value is: {}.",
            input.sig.ident, state_name, ret_val
        );
    }
    let mut generator = Generator::new();
    let expanded = generator.gen_state_machines_tokenstream(input, &state_name, &ret_val);
    return expanded;
}

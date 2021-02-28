#[test]
fn test_generation() {
    use crate::generate_state_machines::Generator;
    use petgraph::dot::Dot;
    use std::fs;
    use std::io::Write;
    use syn::parse_quote;
    use syn::ItemFn;
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
                co_yield;
                co_return(Poll::Pending);
                if cond1{
                    f();
                    co_return(Poll::Pending);
                    g();
                }else{
                    let c=p();
                    co_return(Poll::Ready(Ok(c)));
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
                        return Poll::Ready();
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
                    co_return(Poll::Ready(Ok(c)));
                    do2();
                    if cond4{
                        break;
                    }
                }
            }
    };
    let mut generator = Generator::new();
    println!(
        "{}",
        generator
            .gen_state_machines_tokenstream(f, "state")
            .to_string()
    );
    let cfg_graph = generator.get_cfg_graph();
    let cfg_state_graph = generator.get_cfg_state_graph();
    let dot = Dot::with_config(&cfg_graph, &[]);
    let mut cfg_dot = fs::File::create("cfg.dot").unwrap();
    cfg_dot.write_all(format!("{:#?}", dot).as_bytes()).unwrap();
    let dot = Dot::with_config(&cfg_state_graph, &[]);
    let mut cfg_dot = fs::File::create("cfg_state.dot").unwrap();
    cfg_dot.write_all(format!("{:#?}", dot).as_bytes()).unwrap();
}

#[test]
fn test_is_co_yield_or_co_return() {
    fn co_yield() -> syn::Stmt {
        let nop: ItemFn = parse_quote! {fn nop(){co_yield;}};
        return nop.block.stmts[0].clone();
    }
    fn co_return_no_arg() -> syn::Stmt {
        let nop: ItemFn = parse_quote! {fn nop(){co_return;}};
        return nop.block.stmts[0].clone();
    }
    fn co_return_with_arg() -> syn::Stmt {
        let nop: ItemFn = parse_quote! {fn nop(){co_return(wtf);}};
        return nop.block.stmts[0].clone();
    }
    let stmt = co_yield();
    assert_eq!(is_yield_or_return(&stmt), true);
    let stmt = co_return_no_arg();
    assert_eq!(is_yield_or_return(&stmt), true);
    let stmt = co_return_with_arg();
    assert_eq!(is_yield_or_return(&stmt), true);
}

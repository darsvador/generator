extern crate generator;
use generator::state_machine_generator;

#[cfg(test)]
struct MyGenerator {
    my_state_1: usize,
    pub my_state_2: usize,
    pub num:u32
}

#[cfg(test)]
impl MyGenerator {
    pub fn new() -> MyGenerator {
        MyGenerator {
            my_state_1: 0,
            my_state_2: 0,
            num:1
        }
    }

    #[state_machine_generator(my_state_1)]
    pub fn test_simple(&mut self) {
        loop {
            println!("Hello, ");
            co_yield;
            println!("Generator");
            return;
        }
    }

    // state_name , return_default_value
    #[state_machine_generator(my_state_2,0u32)]
    pub fn get_odd(&mut self)->u32{
        loop{
            co_yield(self.num);
            self.num+=2;
        }
    }
}

#[test]
fn test_generator_proc_macro() {
    let mut gen = MyGenerator::new();
    gen.test_simple(); // print Hello,
    gen.test_simple(); // print Generator
    gen.test_simple(); // print nothing
    for i in (1u32..1000).step_by(2){
        assert_eq!(gen.get_odd(),i);
    }
}

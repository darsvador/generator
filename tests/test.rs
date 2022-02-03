use gentian::gentian;

#[cfg(test)]
struct MyGenerator {
    my_state_1: usize,
    pub my_state_2: usize,
    pub num: u32,
    pub num1: u32,
}

#[cfg(test)]
impl MyGenerator {
    pub fn new() -> MyGenerator {
        MyGenerator {
            my_state_1: 0,
            my_state_2: 0,
            num: 0,
            num1: 0,
        }
    }

    #[gentian]
    #[gentian_attr(state=self.my_state_1)]
    pub fn test_simple(&mut self) {
        loop {
            println!("Hello, ");
            //co_yield;
            while self.num1 < 99 {
                println!("Generator{}", self.num1);
                self.num1 += 1;
                co_yield;
            }
            return;
        }
    }

    // state_name , return_default_value
    #[gentian]
    #[gentian_attr(state=self.my_state_2,ret_val=0u32)]
    pub fn get_odd(&mut self) -> u32 {
        loop {
            if self.num % 2 == 1 {
                co_yield(self.num);
            }
            self.num += 1;
        }
    }
}

#[test]
fn test_generator_proc_macro() {
    let mut gen = MyGenerator::new();
    gen.test_simple(); // print Hello,
    for _ in 0..200 {
        gen.test_simple(); // only print 99 times `Generator`
    }
    gen.test_simple(); // print nothing
    assert_eq!(gen.num1, 99);
    for i in (1u32..1000).step_by(2) {
        assert_eq!(gen.get_odd(), i);
    }
}

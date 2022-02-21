use gentian::gentian;
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProtocolType {
    SS,
    TLS,
    VMESS,
    WS,
    TROJAN,
    DIRECT,
}

pub trait Protocol {
    fn protocol_type(&self) -> ProtocolType;
}

struct ChainStreamBuilderProtocolTypeIter<'a> {
    builders: &'a Vec<Box<dyn Protocol>>,
    ty: Option<ProtocolType>,
    pos: usize,
    state: u32,
}
impl<'a> ChainStreamBuilderProtocolTypeIter<'a> {
    fn new(
        builders: &'a Vec<Box<dyn Protocol>>,
        last_builder: &'a Option<Box<dyn Protocol>>,
    ) -> Self {
        let mut ty = None;
        if let Some(b) = last_builder {
            ty = Some(b.protocol_type());
        }
        Self {
            builders,
            ty,
            pos: builders.len(),
            state: 0,
        }
    }
}

impl<'a> Iterator for ChainStreamBuilderProtocolTypeIter<'a> {
    type Item = ProtocolType;

    #[gentian]
    #[gentian_attr(ret_val=None)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ty.is_some() {
            co_yield(self.ty);
        }
        if self.pos == 0 {
            return None;
        }
        while self.pos != 0 {
            self.pos -= 1;
            co_yield(Some(self.builders[self.pos].protocol_type()));
        }
        return None;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.ty.is_some() {
            return (0, Some(self.pos + 1));
        }
        (0, Some(self.pos))
    }
}
macro_rules! impl_protocol {
    ($name:tt,$type:expr) => {
        struct $name;
        impl Protocol for $name {
            fn protocol_type(&self) -> ProtocolType {
                $type
            }
        }
    };
}
impl_protocol!(Trojan, ProtocolType::TROJAN);
impl_protocol!(Shadowsocks, ProtocolType::SS);
impl_protocol!(Vmess, ProtocolType::VMESS);
impl_protocol!(Direct, ProtocolType::DIRECT);
impl_protocol!(Tls, ProtocolType::TLS);
impl_protocol!(Ws, ProtocolType::WS);

#[test]
fn test_iter_impl() {
    use ProtocolType::{DIRECT, SS, TLS, TROJAN, VMESS, WS};
    let builders: Vec<Box<dyn Protocol>> = vec![
        Box::new(Vmess),
        Box::new(Tls),
        Box::new(Trojan),
        Box::new(Shadowsocks),
        Box::new(Ws),
        Box::new(Vmess),
        Box::new(Direct),
    ];
    let last_builder: Option<Box<dyn Protocol>> = Some(Box::new(Trojan));
    let expected = vec![TROJAN, DIRECT, VMESS, WS, SS, TROJAN, TLS, VMESS];
    let my_iter = ChainStreamBuilderProtocolTypeIter::new(&builders, &last_builder);
    for (real, expected) in my_iter.zip(expected.into_iter()) {
        assert_eq!(real, expected);
    }
    let last_builder = None;
    let expected = vec![DIRECT, VMESS, WS, SS, TROJAN, TLS, VMESS];
    let my_iter = ChainStreamBuilderProtocolTypeIter::new(&builders, &last_builder);
    for (real, expected) in my_iter.zip(expected.into_iter()) {
        assert_eq!(real, expected);
    }
}

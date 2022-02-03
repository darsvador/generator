use bae::FromAttributes;
use quote::ToTokens;

#[derive(Debug, Eq, PartialEq, FromAttributes)]
pub struct GentianAttr {
    pub state: Option<syn::Expr>,
    pub ret_val: Option<syn::Expr>,
}

impl GentianAttr {
    pub fn get_state_name(&self) -> String {
        if let Some(n) = &self.state {
            return n.to_token_stream().to_string();
        }
        String::from("self.state")
    }

    pub fn get_ret_val(&self) -> String {
        if let Some(n) = &self.ret_val {
            return n.to_token_stream().to_string();
        }
        String::new()
    }
}

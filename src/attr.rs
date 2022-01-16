use bae::FromAttributes;
use quote::ToTokens;

#[derive(
Debug,
Eq,
PartialEq,
FromAttributes,
)]
pub struct FsaAttr {
    pub state: Option<syn::Expr>,
    pub ret_val: Option<syn::Expr>,
}

impl FsaAttr{
    pub fn get_state_name(&self)->String{
        if let Some(n)=&self.state{
            return n.to_token_stream().to_string();
        }
        String::from("this.state")
    }

    pub fn get_ret_val(&self)->String{
        if let Some(n)=&self.ret_val{
            return n.to_token_stream().to_string();
        }
        String::new()
    }
}
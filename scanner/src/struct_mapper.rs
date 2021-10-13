use syn::DeriveInput;

#[proc_macro_derive(Struct)]
pub fn ygopro_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = input.ident;
    let struct_name = struct_ident.to_string();
    let _type = match struct_name {
        _ if struct_name.starts_with("CTOS") => "CTOS",
        _ if struct_name.starts_with("STOC") => "STOC",
        _ if struct_name.starts_with("SRVPRU") => "SRVPRU",
        _ if struct_name.starts_with("GM") => "GM",
        _ => "SRVPRU"
    };
    let enum_name = if struct_name.starts_with(_type) { &struct_name[_type.len()..] } else { struct_name.as_str() };
    let type_ident = quote::format_ident!("{}", _type);
    let enum_class_ident = quote::format_ident!("{}MessageType", _type);
    let enum_ident = quote::format_ident!("{}", enum_name);
        
    let expand = quote! {
        impl Struct for #struct_ident {}
        impl MappedStruct for #struct_ident {
            fn message() -> MessageType {
                return MessageType::#type_ident(#enum_class_ident::#enum_ident);
            }
        }
    };
    TokenStream::from(expand)
}

// ===========================================
// srvpru_handler!(CTOSMessageType::xxxxx, |context| { false });
// srvpru_handler!(CTOSJoinGame, |context| { });
// srvpru_handler!(_, ATTACHMENT, |context| {  });
// ===========================================
use syn::Token;

struct RegisterHandlerInput {
    priority: Option<syn::ExprLit>,
    parameters: Vec<syn::ExprPath>,
    block: syn::ExprClosure,
}

impl Parse for RegisterHandlerInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut parameters = Vec::new();
        let mut priority = None;
        let block: syn::ExprClosure;
        loop {
            if let Ok(_block) = input.parse::<syn::ExprClosure>()  { block = _block; break; }
            if let Ok(_parameter) = input.parse::<syn::ExprPath>() { parameters.push(_parameter); }
            if let Ok(_lit) = input.parse::<syn::ExprLit>()        { priority = Some(_lit); }
            input.parse::<Token![,]>()?;
        }
        Ok(RegisterHandlerInput { priority, parameters, block })
    }
}

// Author think extra traits are too slow to compile
// Also even you add it, to_string don't work
// https://github.com/dtolnay/syn/issues/220
fn expr_to_string(path: syn::ExprPath) -> String {
    quote!(#path).to_string()
}

#[proc_macro]
pub fn srvpru_handler(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as RegisterHandlerInput);
    let mut attachment: Option<syn::ExprPath> = None;
    let mut is_always_trigger = false;
    let name: String;
    let size = input.parameters.len();
    match size {
        0 => { name = "_".to_string(); }
        1 => { name = expr_to_string(input.parameters.pop().unwrap()); }
        2 => { attachment = Some(input.parameters.pop().unwrap()); name = expr_to_string(input.parameters.pop().unwrap()); }
        _ => { return TokenStream::from(quote!(compile_error!(too much parameters))); }
    }
    if name == "_" { is_always_trigger = true; }
    let name_path: proc_macro2::TokenStream = name.parse().unwrap();
    let is_enum = name.contains("MessageType") || is_always_trigger;
    let block = input.block;
    let block_inputs = block.inputs;
    let block_body = block.body;
    let priority_ident: proc_macro2::TokenStream = if let Some(lit) = input.priority { quote!(#lit) } else { quote!(100) };

    let attachment_statement = attachment.map(|func| quote!(let mut attachment = #func(context);)).unwrap_or(quote!());
    let message_type = if is_enum {
        let direction = quote::format_ident!("{}", name[0..name.find("MessageType").unwrap()].to_string());
        quote!(crate::ygopro::message::MessageType::#direction(#name_path)) 
    } else { quote!(#name_path::message()) };
    let handler_condition = if is_always_trigger { quote! { |_| true } } else { quote!{|context| context.message_type == Some(#message_type) } };
    let extra_return = if can_add_return_in_last(&block_body) { quote!{ return Ok(false); } } else { quote!() };

    let expand = if is_enum {
        quote!(
            crate::srvpru::Handler::new(#priority_ident, "", #handler_condition, |#block_inputs| Box::pin(async move {
                #attachment_statement
                #block_body
                #extra_return
            }))
        )
    } else {
        quote!(
            crate::srvpru::Handler::follow_message::<#name_path, _>(#priority_ident, "", |#block_inputs| Box::pin(async move {
                #attachment_statement
                #block_body
                #extra_return
            }))
        )
    };
    TokenStream::from(expand)
}


#[proc_macro]
pub fn srvpru_handler_debug(input: TokenStream) -> TokenStream {
    let result = srvpru_handler(input);
    let stream = proc_macro2::TokenStream::from(result);
    println!("{}", stream.to_string());
    return TokenStream::from(stream);
}


fn can_add_return_in_last(block: &Box<syn::Expr>) -> bool {
    let block = *block.clone();
    if let syn::Expr::Block(expr) = block {
        let block = expr.block;
        let last_stmt = block.stmts.last();
        if last_stmt.is_none() { return true; }
        let last_stmt = last_stmt.unwrap();
        match last_stmt {
            syn::Stmt::Expr(_) => false,
            syn::Stmt::Semi(semi, _) => match semi {
                syn::Expr::Return(_) => false,
                _ => true
            },
            _ => true
        }
    }
    else { false }
}
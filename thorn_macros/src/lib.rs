#![allow(clippy::single_char_add_str)]

extern crate proc_macro;
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Spacing, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

#[proc_macro]
pub fn sql2(input: TokenStream) -> TokenStream {
    syn::parse_macro_input!(input with do_parse).into()
}

mod lit;
//mod punct;

use syn::{ext::IdentExt, parse::ParseStream, Expr, Ident, Token};

mod kw {
    syn::custom_keyword!(INTO);
    syn::custom_keyword!(FROM);
    syn::custom_keyword!(AS);

    syn::custom_keyword!(join);
}

#[derive(Default)]
struct State {
    buffer: String,
    stack: String,
    exports: indexmap::IndexSet<Ident>,
    /// When working within a table
    current_table: Option<Ident>,
    depth: usize,
}

impl State {
    fn ident(&mut self, ident: &Ident) -> &str {
        use std::fmt::Write;

        self.buffer.clear();
        write!(self.buffer, "{}", ident).unwrap();
        &self.buffer
    }

    fn rewrite_spacing(&mut self) {
        let Some(start) = self.stack.len().checked_sub(2) else { return; };

        let mut chars = self.stack.chars().rev();

        let token = chars.next().unwrap();
        let maybe_ws = chars.next().unwrap();

        if maybe_ws.is_whitespace() && matches!(token, ',' | ')' | ']') {
            self.stack.truncate(start);
            self.stack.push(token);
        }
    }

    fn push(&mut self, tokens: impl ToTokens) {
        use std::fmt::Write;

        for t in tokens.into_token_stream() {
            let old_len = self.stack.len();
            write!(self.stack, "{}", t).unwrap();

            // single-byte token, check for punctuation rewrite
            if old_len + 1 == self.stack.len() {
                self.rewrite_spacing();
            }

            match t {
                TokenTree::Punct(ref p) if p.spacing() == Spacing::Joint => {}
                _ => self.stack.push_str(" "),
            }
        }
    }

    fn push_str(&mut self, token: impl AsRef<str>) {
        let token = token.as_ref();
        self.stack.push_str(token);
        if token.len() == 1 {
            self.rewrite_spacing();
        }
        if !matches!(token, "(" | "[") {
            self.stack.push_str(" ");
        }
    }

    fn flush(&mut self, out: &mut TokenStream2) {
        if !self.stack.is_empty() {
            let mut stack = std::mem::take(&mut self.stack);
            if !stack.ends_with(' ') {
                stack.push_str(" ");
            }
            out.extend(quote::quote! { __out.write_str(#stack); });
        }
    }
}

fn do_parse(input: ParseStream) -> syn::Result<TokenStream2> {
    let mut state = State::default();
    let mut tokens = state.parse(input)?;

    state.flush(&mut tokens);

    Ok(quote::quote! {
        #[allow(unused_braces)]
        { #tokens }
    })
}

impl State {
    fn parse_nested(&mut self, input: ParseStream) -> syn::Result<TokenStream2> {
        self.depth += 1;
        let mut res = self.parse(input);
        if let Ok(ref mut out) = res {
            self.flush(out);
        }
        self.depth -= 1;
        res
    }

    fn parse(&mut self, input: ParseStream) -> syn::Result<TokenStream2> {
        let mut out = TokenStream2::new();

        while !input.is_empty() {
            match () {
                // AS @Name
                _ if input.peek(kw::AS) && input.peek2(Token![@]) && input.peek3(Ident) => {
                    let _as_token: kw::AS = input.parse()?;
                    let _at_token: Token![@] = input.parse()?;
                    let fork_at_name = input.fork();
                    let export_name = input.parse()?;

                    if !self.exports.insert(export_name) {
                        return Err(fork_at_name.error("Duplicate export"));
                    }
                }

                // AS Table.Column
                _ if input.peek(kw::AS) && input.peek2(Ident) && input.peek3(Token![.]) => {
                    let as_token: kw::AS = input.parse()?;
                    let table: Ident = input.parse()?;
                    let _dot: Token![.] = input.parse()?;
                    let column: Ident = input.parse()?;

                    self.push(as_token);
                    self.flush(&mut out);

                    out.extend(quote::quote! { __out.write_column_name(#table::#column)?; });
                }

                _ if is_macro(input) => {
                    input.parse::<syn::Stmt>()?.to_tokens(&mut out);
                }

                _ if input.peek(Token![match]) => {
                    self.flush(&mut out);
                    parse_match(input, self)?.to_tokens(&mut out);
                }

                _ if input.peek(Token![if]) => {
                    self.flush(&mut out);
                    parse_if(input, self)?.to_tokens(&mut out);
                }

                _ if (input.peek(Token![for]) || input.peek(kw::join))
                    || (input.peek(syn::Lifetime)
                        && input.peek2(Token![:])
                        && (input.peek3(Token![for]) || input.peek3(kw::join))) =>
                {
                    self.flush(&mut out);
                    parse_for(input, self)?.to_tokens(&mut out);
                }

                _ if input.peek(Ident) => {
                    let ident: Ident = input.parse()?;

                    match () {
                        _ if KEYWORDS.contains(self.ident(&ident)) => self.push(ident),
                        //_ if input.peek(kw::AS) && input.peek2(Ident) => {
                        //
                        //}
                        _ if input.peek(Token![.]) && input.peek2(Ident) => {
                            let _dot: Token![.] = input.parse()?;
                            let column: Ident = input.parse()?;

                            let table_name = ident.to_string().to_snake_case();

                            self.flush(&mut out);
                            out.extend(quote::quote! { __out.write_column(#ident::#column, #table_name)?; });
                        }
                        // functions
                        _ if input.peek(syn::token::Paren) => {}
                        _ => {
                            self.flush(&mut out);
                            out.extend(quote::quote! { __out.write_table::<#ident>()?; });
                        }
                    }
                }

                // SQL literals
                _ if input.peek(syn::Lit) => {
                    lit::push_lit(lit::parse_lit(input)?, self);
                }

                // parameters #{&value as Type::INT4}
                _ if input.peek(Token![#]) && input.peek2(syn::token::Brace) => {
                    let _pound_token: Token![#] = input.parse()?;

                    let inner;
                    syn::braced!(inner in input);
                    let syn::ExprCast { expr, ty, .. } = inner.parse::<syn::ExprCast>()?;

                    self.flush(&mut out);
                    out.extend(quote::quote! { __out.param(#expr, #ty.into()); });
                }

                // statements
                _ if is_stmt(input) => {
                    input.parse::<syn::Stmt>()?.to_tokens(&mut out);
                }

                // deny other Rust keywords, if they aren't part of built-in syntax or arbitrary statements
                _ if is_rust_keyword(input) => {
                    return Err(input.error("Unexpected Rust keyword"));
                }

                // { ... }, runtime literals
                _ if input.peek(syn::token::Brace) => {
                    let expr = input.parse::<syn::Block>()?;

                    self.flush(&mut out);
                    out.extend(quote::quote! { __out.write_literal(#expr)?; });
                }

                // (...)
                _ if input.peek(syn::token::Paren) => {
                    let inner;
                    syn::parenthesized!(inner in input);

                    self.push_str("(");
                    self.parse(&inner)?.to_tokens(&mut out);
                    self.push_str(")");
                }

                // [...]
                _ if input.peek(syn::token::Bracket) => {
                    let inner;
                    syn::bracketed!(inner in input);

                    self.push_str("[");
                    self.parse(&inner)?.to_tokens(&mut out);
                    self.push_str("]");
                }

                // detect trailing commas
                _ if input.peek(Token![,]) => {
                    let fork = input.fork();
                    let comma: Token![,] = input.parse()?;

                    if input.is_empty() || input.peek(kw::FROM) {
                        return Err(fork.error("Trailing commas are not allowed in SQL"));
                    }

                    self.push(comma);
                }

                // passthrough as text
                _ => self.push(input.parse::<proc_macro2::TokenTree>()?),
            }
        }

        Ok(out)
    }
}

fn is_rust_keyword(input: ParseStream) -> bool {
    input.peek(Ident::peek_any) && !input.peek(Ident)
}

fn is_macro(input: ParseStream) -> bool {
    if input.peek(Ident::peek_any) {
        if input.peek2(Token![!]) {
            return true;
        }

        let fork = input.fork();
        if fork.peek2(Token![::]) && syn::Path::parse_mod_style(&fork).is_ok() && fork.peek(Token![!]) {
            return true;
        }
    }

    false
}

fn is_stmt(input: ParseStream) -> bool {
    if input.peek(Token![let])
        || input.peek(Token![const])
        || input.peek(Token![use])
        || input.peek(Token![continue])
        || input.peek(Token![break])
        || is_macro(input)
    {
        return true;
    }

    false
}

struct Match {
    match_token: Token![match],
    expr: Box<Expr>,
    brace_token: syn::token::Brace,
    arms: Vec<Arm>,
}

struct Arm {
    pat: syn::Pat,
    guard: Option<(Token![if], Box<syn::Expr>)>,
    fat_arrow_token: Token![=>],
    brace_token: syn::token::Brace,
    body: TokenStream2,
    comma: Option<Token![,]>,
}

enum Else {
    If(If),
    Block((syn::token::Brace, TokenStream2)),
}

struct If {
    if_token: Token![if],
    cond: Box<Expr>,
    brace_token: syn::token::Brace,
    then_branch: TokenStream2,
    else_branch: Option<(Token![else], Box<Else>)>,
}

struct For {
    label: Option<syn::Label>,
    for_token: Token![for],
    joiner: Option<syn::LitStr>,
    pat: Box<syn::Pat>,
    in_token: Token![in],
    expr: Box<Expr>,
    brace_token: syn::token::Brace,
    body: TokenStream2,
}

fn parse_match(input: ParseStream, state: &mut State) -> syn::Result<Match> {
    let match_token = input.parse()?;
    let expr = Expr::parse_without_eager_brace(input)?;

    let content;
    let brace_token = syn::braced!(content in input);
    let mut arms = Vec::new();

    while !content.is_empty() {
        arms.push(parse_arm(&content, state)?);
    }

    Ok(Match {
        match_token,
        expr: Box::new(expr),
        brace_token,
        arms,
    })
}

impl ToTokens for Match {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Match {
            match_token,
            expr,
            brace_token,
            arms,
        } = self;

        match_token.to_tokens(tokens);
        expr.to_tokens(tokens);

        brace_token.surround(tokens, |tokens| {
            for arm in arms {
                arm.to_tokens(tokens);
            }
        });
    }
}

fn parse_arm(input: ParseStream, state: &mut State) -> syn::Result<Arm> {
    let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
    let guard = if input.peek(Token![if]) { Some((input.parse()?, input.parse()?)) } else { None };
    let fat_arrow_token = input.parse()?;
    let body;
    let brace_token = syn::braced!(body in input);
    let body = state.parse_nested(&body)?;
    let comma = input.parse()?;

    Ok(Arm {
        pat,
        guard,
        fat_arrow_token,
        brace_token,
        body,
        comma,
    })
}

impl ToTokens for Arm {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.pat.to_tokens(tokens);
        if let Some((ref if_token, ref cond)) = self.guard {
            if_token.to_tokens(tokens);
            cond.to_tokens(tokens);
        }
        self.fat_arrow_token.to_tokens(tokens);
        self.brace_token.surround(tokens, |tokens| {
            self.body.to_tokens(tokens);
        });
        self.comma.to_tokens(tokens);
    }
}

fn parse_if(input: ParseStream, state: &mut State) -> syn::Result<If> {
    let if_token = input.parse()?;
    let cond = Expr::parse_without_eager_brace(input)?;
    let then;
    let brace_token = syn::braced!(then in input);
    let then_branch = state.parse_nested(&then)?;

    let else_branch = if input.peek(Token![else]) {
        let else_token = input.parse()?;

        let else_branch = if input.peek(Token![if]) {
            Else::If(parse_if(input, state)?)
        } else {
            Else::Block({
                let inner;
                let brace_token = syn::braced!(inner in input);

                (brace_token, state.parse_nested(&inner)?)
            })
        };

        Some((else_token, Box::new(else_branch)))
    } else {
        None
    };

    Ok(If {
        if_token,
        cond: Box::new(cond),
        brace_token,
        then_branch,
        else_branch,
    })
}

impl ToTokens for If {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let If {
            if_token,
            cond,
            brace_token,
            then_branch,
            else_branch,
        } = self;

        if_token.to_tokens(tokens);
        cond.to_tokens(tokens);

        brace_token.surround(tokens, |tokens| {
            then_branch.to_tokens(tokens);
        });

        if let Some((else_token, else_branch)) = else_branch {
            else_token.to_tokens(tokens);

            match **else_branch {
                Else::Block(ref block) => {
                    let (ref brace, ref block) = *block;
                    brace.surround(tokens, |tokens| block.to_tokens(tokens));
                }
                Else::If(ref else_if) => else_if.to_tokens(tokens),
            }
        }
    }
}

fn parse_for(input: ParseStream, state: &mut State) -> syn::Result<For> {
    let label = if input.peek(syn::Lifetime) { Some(input.parse()?) } else { None };

    let mut joiner = None;
    let for_token: Token![for] = if input.peek(kw::join) {
        let join_token = input.parse::<kw::join>()?;

        joiner = Some(if input.peek(syn::token::Paren) {
            let joiner_input;
            syn::parenthesized!(joiner_input in input);

            joiner_input.parse()?
        } else {
            syn::LitStr::new(",", join_token.span)
        });

        Token![for](join_token.span)
    } else {
        input.parse()?
    };

    let pat = syn::Pat::parse_multi_with_leading_vert(input)?;

    let in_token = input.parse()?;
    let expr = Expr::parse_without_eager_brace(input)?;

    let inner;
    let brace_token = syn::braced!(inner in input);
    let body = state.parse_nested(&inner)?;

    Ok(For {
        label,
        for_token,
        joiner,
        pat: Box::new(pat),
        in_token,
        expr: Box::new(expr),
        brace_token,
        body,
    })
}

impl ToTokens for For {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let For {
            label,
            for_token,
            joiner,
            pat,
            in_token,
            expr,
            brace_token,
            body,
        } = self;

        if joiner.is_some() {
            tokens.extend(quote::quote! {
                let mut __first = true;
            });
        }

        label.to_tokens(tokens);
        for_token.to_tokens(tokens);
        pat.to_tokens(tokens);
        in_token.to_tokens(tokens);
        expr.to_tokens(tokens);

        brace_token.surround(tokens, |tokens| {
            if let Some(ref joiner) = joiner {
                tokens.extend(quote::quote! {
                    if !__first { __out.write_str(#joiner); }
                    __first = false;
                });
            }

            body.to_tokens(tokens);
        });
    }
}

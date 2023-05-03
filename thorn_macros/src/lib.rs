#![allow(clippy::single_char_add_str)]

extern crate proc_macro;

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Spacing, Span, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

#[proc_macro]
pub fn __isql2(input: TokenStream) -> TokenStream {
    syn::parse_macro_input!(input with do_parse).into()
}

fn do_parse(input: ParseStream) -> syn::Result<TokenStream2> {
    let writer = input.parse()?;

    let mut state = State {
        writer,
        buffer: Default::default(),
        stack: Default::default(),
        exports: Default::default(),
        current_table: Default::default(),
        depth: Default::default(),
    };

    let mut tokens = state.parse(input, &mut 0)?;

    state.flush(&mut tokens);

    if let Some(first) = state.exports.swap_remove_index(0) {
        let mut buffer = state.buffer;

        let accessors = std::iter::once(&first).chain(&state.exports).map(|export| {
            let snake = buffer.ident(export).to_snake_case();

            let name = Ident::new(&snake, export.span());

            quote::quote! {
                #[inline(always)]
                pub fn #name<'a, T>(&'a self) -> Result<T, PgError>
                where T: FromSql<'a>
                { self.try_get(ColumnIndices::#export as usize) }
            }
        });

        let exports = state.exports.iter();

        tokens.extend(quote::quote! {
            #[allow(clippy::enum_variant_names)]
            #[repr(usize)]
            enum ColumnIndices {
                #first = 0,
                #(#exports,)*
            }

            impl Columns {
                #(#accessors)*
            }
        });
    }

    Ok(tokens)
}

mod lit;
//mod punct;

use syn::{
    ext::IdentExt,
    parse::ParseStream,
    token::{Brace, Bracket, Paren},
    Error, Expr, Ident, LitStr, Token,
};

mod kw {
    syn::custom_keyword!(INTO);
    syn::custom_keyword!(FROM);
    syn::custom_keyword!(AS);

    syn::custom_keyword!(join);
}

#[derive(Default)]
struct IdentBuffer {
    buffer: String,
}

impl IdentBuffer {
    fn ident(&mut self, ident: &Ident) -> &str {
        use std::fmt::Write;

        self.buffer.clear();
        write!(self.buffer, "{}", ident).unwrap();
        &self.buffer
    }
}

struct State {
    writer: Ident,
    buffer: IdentBuffer,
    stack: String,
    exports: indexmap::IndexSet<Ident>,
    /// When working within a table
    current_table: Option<Ident>,
    depth: usize,
}

impl State {
    fn ident(&mut self, ident: &Ident) -> &str {
        self.buffer.ident(ident)
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
        if !matches!(token, "(" | "[" | "::") {
            self.stack.push_str(" ");
        }
    }

    fn flush(&mut self, out: &mut TokenStream2) {
        if !self.stack.is_empty() {
            let mut stack = std::mem::take(&mut self.stack);
            if !(stack.ends_with(&[' ', '(', '[']) || stack.ends_with("::")) {
                stack.push_str(" ");
            }
            let writer = &self.writer;
            out.extend(quote::quote! { #writer.write_str(#stack); });
        }
    }

    fn parse_nested(&mut self, input: ParseStream) -> syn::Result<TokenStream2> {
        self.depth += 1;
        let mut res = self.parse(input, &mut 0);
        if let Ok(ref mut out) = res {
            self.flush(out);
        }
        self.depth -= 1;
        res
    }

    fn assert_export_top(&self, span: Span) -> syn::Result<()> {
        if self.depth != 0 {
            return Err(Error::new(span, "Exports may only be defined in the top SQL scope"));
        }

        Ok(())
    }

    fn parse_ident_sequence(&mut self, input: ParseStream, out: &mut TokenStream2) -> syn::Result<()> {
        let ident: Ident = input.parse()?;

        match () {
            _ if KEYWORDS.contains(self.ident(&ident)) => self.push(ident),
            //_ if input.peek(kw::AS) && input.peek2(Ident) => {
            //
            //}

            // Table.Column
            _ if input.peek(Token![.]) && input.peek2(Ident) => {
                let _dot: Token![.] = input.parse()?;
                let column: Ident = input.parse()?;

                let table_name = self.ident(&ident).to_snake_case();

                self.flush(out);
                let writer = &self.writer;
                out.extend(quote::quote! { #writer.write_column(#ident::#column, #table_name)?; });

                // Table.Column AS @_
                //
                // Combines the table name and column name for an automatic export name
                if input.peek(kw::AS) && input.peek2(Token![@]) && input.peek3(Token![_]) {
                    let as_token: kw::AS = input.parse()?;
                    let at_token: Token![@] = input.parse()?;
                    let _: Token![_] = input.parse()?;

                    self.assert_export_top(as_token.span)?;

                    let column_name = self.ident(&column).to_snake_case();

                    self.exports.insert(Ident::new(&format!("{table_name}_{column_name}"), at_token.span));
                }
            }

            // Table./Column syntax for unqualified column names
            _ if input.peek(Token![.]) && input.peek2(Token![/]) && input.peek3(Ident) => {
                let _dot: Token![.] = input.parse()?;
                let _slash: Token![/] = input.parse()?;
                let column: Ident = input.parse()?;

                self.flush(out);
                let writer = &self.writer;
                out.extend(quote::quote! { #writer.write_column_name(#ident::#column)?; });
            }

            // Ident(...), function calls, just let through unchanged,
            // parens will be handled elsewhere
            _ if input.peek(Paren) => {
                self.push(ident);
            }

            // Table
            _ => {
                self.flush(out);
                let writer = &self.writer;
                out.extend(quote::quote! { #writer.write_table::<#ident>()?; });

                // Table AS Alias
                //
                // This adds on `AS "renamed"` and declared a type alias for other chunks
                // of code to reference
                if input.peek(kw::AS) && input.peek2(Ident) {
                    let as_token: kw::AS = input.parse()?;
                    let alias: Ident = input.parse()?;

                    let alias_name = self.ident(&alias).to_snake_case();

                    self.push(as_token);
                    self.push(LitStr::new(&alias_name, alias.span()));

                    out.extend(quote::quote! { type #alias = #ident; });
                }
            }
        }

        Ok(())
    }

    fn parse(&mut self, input: ParseStream, comma_counter: &mut usize) -> syn::Result<TokenStream2> {
        let mut out = TokenStream2::new();

        while !input.is_empty() {
            match () {
                _ if input.peek(kw::AS) => {
                    let as_token: kw::AS = input.parse()?;

                    match () {
                        // AS @Name
                        _ if input.peek(Token![@]) && input.peek2(Ident) => {
                            let _: Token![@] = input.parse()?;
                            let export_name: Ident = input.parse()?;

                            self.assert_export_top(export_name.span())?;

                            if let Some(prev) = self.exports.replace(export_name.clone()) {
                                let mut err = Error::new(export_name.span(), "Duplicate export");
                                err.combine(Error::new(prev.span(), "Previously defined here"));

                                return Err(err);
                            }
                        }
                        // AS Table.Column
                        _ if input.peek(Ident) && input.peek2(Token![.]) && input.peek3(Ident) => {
                            let table: Ident = input.parse()?;
                            let _dot: Token![.] = input.parse()?;
                            let column: Ident = input.parse()?;

                            self.push(as_token);
                            self.flush(&mut out);

                            let writer = &self.writer;
                            out.extend(quote::quote! { #writer.write_column_name(#table::#column)?; });
                        }
                        _ => return Err(Error::new(as_token.span, "Unexpected AS")),
                    }
                }

                // .func(1, 2, 3)
                _ if input.peek(Token![.]) && input.peek2(Ident) && input.peek3(Paren) => {
                    let _: Token![.] = input.parse()?;
                    let ident: Ident = input.parse()?;

                    let args;
                    let parens = syn::parenthesized!(args in input);

                    let has_args = !args.is_empty();
                    let mut num_commas = 0;

                    self.flush(&mut out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { #writer.write_func::<#ident>(); });

                    // like () handling, but counts the commas in the subtree
                    self.push_str("(");
                    self.depth += 1;
                    self.parse(&args, &mut num_commas)?.to_tokens(&mut out);
                    self.depth -= 1;
                    self.push_str(")");

                    // if we have arguments, commas + 1 is the number of arguments
                    num_commas += has_args as usize;

                    // <func>::func((), (), ()) but with correct span for parenthesis
                    out.extend(quote::quote!(<#ident>::#ident));
                    parens.surround(&mut out, |tokens| {
                        let empty = quote::quote!(());
                        let empties = (0..num_commas).map(|_| &empty);
                        tokens.extend(quote::quote!(#(#empties,)*));
                    });
                    out.extend(quote::quote!(;));
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
                    parse_for(input, self)?.to_tokens(&mut out, self);
                }

                _ if input.peek(Ident) => {
                    self.parse_ident_sequence(input, &mut out)?;
                }

                // SQL literals
                _ if input.peek(syn::Lit) => {
                    lit::push_lit(lit::parse_lit(input)?, self);
                }

                // parameters #{&value as Type::INT4}
                _ if input.peek(Token![#]) && input.peek2(Brace) => {
                    let _pound_token: Token![#] = input.parse()?;

                    let inner;
                    syn::braced!(inner in input);
                    let syn::ExprCast { expr, ty, .. } = inner.parse::<syn::ExprCast>()?;

                    self.flush(&mut out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { #writer.param(#expr, #ty.into()); });
                }

                _ if input.peek(Token![@]) && input.peek2(Brace) => {
                    let _at_token: Token![@] = input.parse()?;
                    let block: syn::Block = input.parse()?;

                    self.flush(&mut out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { write!(#writer, "{}", #block)?; });
                }

                // arbitrary Rust expressions ${x += 1;}
                _ if input.peek(Token![$]) && input.peek2(Brace) => {
                    let _bang: Token![$] = input.parse()?;
                    input.parse::<syn::Block>()?.to_tokens(&mut out);
                }

                // SQL type casting
                _ if input.peek(Token![::]) => {
                    let _colon: Token![::] = input.parse()?;
                    self.push_str("::");

                    if input.peek(Ident) {
                        let ident: Ident = input.parse()?;

                        let mut ty = self.ident(&ident).to_uppercase();

                        // convert _TYPE to TYPE_ARRAY
                        if let Some(array_ty) = ty.strip_prefix('_') {
                            ty = format!("{array_ty}_ARRAY");
                        }

                        let ty_ident = Ident::new(&ty, ident.span());

                        self.flush(&mut out);
                        let writer = &self.writer;
                        out.extend(quote::quote! { #writer.write_str(pg::Type::#ty_ident.name()); });

                        // cheat lack of space after this write_str by pushing an empty string
                        self.push_str("");
                    } else if input.peek(Brace) {
                        let block: syn::Block = input.parse()?;

                        self.flush(&mut out);
                        let writer = &self.writer;
                        out.extend(quote::quote! { write!(#writer, "{} ", pg::Type::from(#block))?; });
                    }
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
                _ if input.peek(Brace) => {
                    let expr = input.parse::<syn::Block>()?;

                    self.flush(&mut out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { #writer.write_literal(#expr)?; });
                }

                // (...)
                _ if input.peek(Paren) => {
                    let inner;
                    syn::parenthesized!(inner in input);

                    self.push_str("(");
                    self.depth += 1;
                    self.parse(&inner, &mut 0)?.to_tokens(&mut out);
                    self.depth -= 1;
                    self.push_str(")");
                }

                // [...]
                _ if input.peek(Bracket) => {
                    let inner;
                    syn::bracketed!(inner in input);

                    self.push_str("[");
                    self.depth += 1;
                    self.parse(&inner, &mut 0)?.to_tokens(&mut out);
                    self.depth -= 1;
                    self.push_str("]");
                }

                // detect trailing commas
                _ if input.peek(Token![,]) => {
                    let comma: Token![,] = input.parse()?;

                    if input.is_empty() || input.peek(kw::FROM) {
                        return Err(Error::new(comma.span, "Trailing commas are not allowed in SQL"));
                    }

                    self.push(comma);
                    *comma_counter += 1;
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

// Supported subset of statements
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
    brace_token: Brace,
    arms: Vec<Arm>,
}

struct Arm {
    pat: syn::Pat,
    guard: Option<(Token![if], Box<syn::Expr>)>,
    fat_arrow_token: Token![=>],
    brace_token: Brace,
    body: TokenStream2,
    comma: Option<Token![,]>,
}

enum Else {
    If(If),
    Block((Brace, TokenStream2)),
}

struct If {
    if_token: Token![if],
    cond: Box<Expr>,
    brace_token: Brace,
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
    brace_token: Brace,
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

impl If {
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
    let label = input.parse()?;

    let mut joiner = None;
    let for_token: Token![for] = if input.peek(kw::join) {
        let join_token = input.parse::<kw::join>()?;

        joiner = Some(if input.peek(Paren) {
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

impl For {
    fn to_tokens(&self, tokens: &mut TokenStream2, state: &State) {
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
            tokens.extend(quote::quote! { let mut __first = true; });
        }

        label.to_tokens(tokens);
        for_token.to_tokens(tokens);
        pat.to_tokens(tokens);
        in_token.to_tokens(tokens);
        expr.to_tokens(tokens);

        brace_token.surround(tokens, |tokens| {
            if let Some(ref joiner) = joiner {
                let writer = &state.writer;
                tokens.extend(quote::quote! {
                    if !__first { #writer.write_str(#joiner); }
                    __first = false;
                });
            }

            body.to_tokens(tokens);
        });
    }
}

use crate::core::node::{Selector, SelectorGroup, SelectorPart};
use proc_macro::{Delimiter, Span, TokenStream, TokenTree};

use std::collections::HashMap;
use std::iter::Peekable;
pub fn parse_selector_group<I: 'static>(
    read_tokens: Vec<TokenTree>,
    tokens: &mut Peekable<I>,
) -> Option<(SelectorGroup, TokenStream)>
where
    I: Iterator<Item = TokenTree>,
{
    let mut tokens = read_tokens.into_iter().chain(tokens).peekable();

    let mut selectors = Vec::<Selector>::new();

    while let Some(current) = tokens.peek().cloned() {
        match current {
            TokenTree::Group(ref group) if group.delimiter() == Delimiter::Brace => {
                return Some((selectors, group.stream()));
            }
            TokenTree::Punct(ref punct) if punct.as_char() == ',' => {
                tokens.next();
            }
            _ => {
                if let Some(selector) = parse_selector(&mut tokens) {
                    selectors.push(selector);
                }
            }
        };
    }

    None
}

fn parse_selector<I>(tokens: &mut Peekable<I>) -> Option<Selector>
where
    I: Iterator<Item = TokenTree>,
{
    let mut selector_parts = Vec::<SelectorPart>::new();
    let mut last_part_span: Option<Span> = None;
    let mut ignore_token = false;

    let construct_selector = |selector_parts: Vec<SelectorPart>| {
        let filters = {
            let mut map = HashMap::<&str, Box<dyn Fn(&&SelectorPart) -> bool>>::with_capacity(2);
            map.insert(
                "pseudo class",
                Box::new(|part| match part {
                    SelectorPart::PseudoClass { .. } => true,
                    _ => false,
                }),
            );
            map.insert(
                "pseudo element",
                Box::new(|part| match part {
                    SelectorPart::PseudoElement { .. } => true,
                    _ => false,
                }),
            );
            map.insert(
                "element",
                Box::new(|part| match part {
                    SelectorPart::Element { .. } => true,
                    _ => false,
                }),
            );
            map
        };
        if !selector_parts.is_empty() {
            let mut duplicate_detected = false;
            for (name, filter) in filters {
                let filtered = selector_parts
                    .iter()
                    .filter(filter)
                    .collect::<Vec<&SelectorPart>>();

                if filtered.len() >= 2 {
                    for part in filtered {
                        if let Some(span) = part.span() {
                            span.error(format!("Use only one {}", name)).emit();
                            duplicate_detected = true;
                        }
                    }
                }
            }

            if duplicate_detected {
                None
            } else {
                Some(Selector {
                    parts: selector_parts
                        .into_iter()
                        .rev()
                        .skip_while(|part| *part == SelectorPart::Spacing)
                        .map(|part| part.clone())
                        .collect::<Vec<SelectorPart>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<SelectorPart>>(),
                })
            }
        } else {
            None
        }
    };

    while let Some(current) = tokens.peek().cloned() {
        if ignore_token {
            match current {
                TokenTree::Group(ref group) if group.delimiter() == Delimiter::Brace => {
                    last_part_span
                        .unwrap_or(group.span())
                        .warning("Parse failed because of before error(s)")
                        .emit();
                    return None;
                }
                TokenTree::Punct(ref punct) if punct.as_char() == ',' => {
                    if let Some(last_part_span) = last_part_span {
                        last_part_span.error("Not parsable selectors").emit();
                    }
                    return None;
                }
                _ => {
                    last_part_span = last_part_span
                        .unwrap_or(current.span())
                        .join(current.span());
                }
            }

            tokens.next();

            continue;
        }
        match current {
            TokenTree::Group(ref token) if token.delimiter() == Delimiter::Brace => {
                return construct_selector(selector_parts);
            }
            TokenTree::Punct(ref token) if token.as_char() == ',' => {
                return construct_selector(selector_parts);
            }
            _ => {
                if let Some(result) = parse_selector_part(&current, tokens) {
                    if let (Some(last_part_span), Some(span)) = (last_part_span, result.span()) {
                        if last_part_span.end() != span.start() {
                            selector_parts.push(SelectorPart::Spacing);
                        }
                    }
                    last_part_span = result.span().or(last_part_span);

                    selector_parts.push(result);
                } else {
                    current.span().error("Not parsable selector").emit();
                    ignore_token = true;
                    last_part_span = Some(current.span());
                }
            }
        };
    }

    None
}

fn parse_selector_part<I>(current: &TokenTree, tokens: &mut Peekable<I>) -> Option<SelectorPart>
where
    I: Iterator<Item = TokenTree>,
{
    match current {
        TokenTree::Punct(ref punct) if punct.as_char() == '&' => {
            tokens.next();
            Some(SelectorPart::Itself {
                span: current.span(),
            })
        }
        TokenTree::Punct(ref punct) if punct.as_char() == '.' => {
            tokens.next();
            let result = parse_identifier(Some(current.span()), tokens);
            if let Some((ident, span)) = result {
                let span = current.span().join(span).expect("In the same file");
                Some(SelectorPart::Class { span, name: ident })
            } else {
                current
                    .span()
                    .error("Expected identifier but no identifier received")
                    .emit();
                None
            }
        }
        TokenTree::Punct(ref punct) if punct.as_char() == ':' => {
            tokens.next();
            let is_pseudo_element = if let Some(TokenTree::Punct(ref punct)) = tokens.peek() {
                if punct.as_char() == ':' {
                    tokens.next();
                    true
                } else {
                    false
                }
            } else {
                false
            };

            let result = parse_identifier(Some(current.span()), tokens);
            if let Some((ident, span)) = result {
                let span = current.span().join(span).expect("In the same file");
                if is_pseudo_element {
                    Some(SelectorPart::PseudoElement { span, name: ident })
                } else {
                    // todo: parse parameter
                    Some(SelectorPart::PseudoClass {
                        span,
                        name: ident,
                        parameter: None,
                    })
                }
            } else {
                current
                    .span()
                    .error("Expected identifier but no identifier received")
                    .emit();
                None
            }
        }
        TokenTree::Punct(ref punct) if punct.as_char() == '#' => {
            tokens.next();
            let result = parse_identifier(Some(current.span()), tokens);
            if let Some((ident, span)) = result {
                let span = current.span().join(span).expect("In the same file");
                Some(SelectorPart::Id { span, name: ident })
            } else {
                current
                    .span()
                    .error("Expected identifier but no identifier received")
                    .emit();
                None
            }
        }

        TokenTree::Punct(ref punct) if punct.as_char() == '*' => {
            tokens.next();
            Some(SelectorPart::Universal {
                span: current.span(),
                namespace: None,
            })
        }
        _ => {
            let result = parse_identifier(None, tokens);
            if let Some((ident, span)) = result {
                let span = current.span().join(span).expect("In the same file");
                // todo: css namespace support (e.g. `svg|a`, `|a`, `*|a`)
                // ? check required: should we filter identifier by html-element-set?
                Some(SelectorPart::Element {
                    span: span,
                    namespace: None,
                    name: ident,
                })
            } else {
                None
            }
        }
    }
    //? S = Q(CQ)*
    //? C = + | > | ~
    //? Q = (a|p|n)*
    //? a = '[' T ident (('^=' | '$=' | '*=' | '=' | '~=' | '|=') (ident | String))? ']'
    //? p = ':'{1,2} ident ('('expr')')?
    //? n = ':not(' (t|u|h|c|a|p) ')'
}

fn parse_identifier<I>(span: Option<Span>, tokens: &mut Peekable<I>) -> Option<(String, Span)>
where
    I: Iterator<Item = TokenTree>,
{
    let mut result = String::new();
    let mut span = span;

    while let Some(token) = tokens.peek().cloned() {
        if let Some(span) = span {
            if span.end() != token.span().start() {
                break;
            }
        }
        match token {
            TokenTree::Group(_) => {
                break;
            }
            TokenTree::Punct(ref punct)
                if punct.as_char() == '.' || punct.as_char() == '#' || punct.as_char() == ':' =>
            {
                break;
            }
            _ => {
                result.push_str(&token.to_string());
                span = span.map_or(Some(token.span()), |span| span.join(token.span()));
                tokens.next();
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        span.map(|span| (result, span))
    }
}

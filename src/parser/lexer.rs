use std::iter::{from_fn, once};

#[derive(Debug, Clone)]
pub enum Token {
    Symbol(String),
    OpenParen,
    CloseParen,
    Lambda,
    Dot,
    Pipe,
    Eof,
}

fn match_reserved_token(c: char) -> Option<Token> {
    match c {
        '(' => Some(Token::OpenParen),
        ')' => Some(Token::CloseParen),
        'λ' | '@' | '\\' => Some(Token::Lambda),
        '.' => Some(Token::Dot),
        '|' => Some(Token::Pipe),
        _ => None,
    }
}

/// Create a Token iterator from &str
pub fn lexer(input: &str) -> impl Iterator<Item = Token> {
    input
        .split_ascii_whitespace()
        .flat_map(|token| {
            let mut chars = token.chars().peekable();
            from_fn(move || {
                let c = chars.peek()?;
                match match_reserved_token(*c) {
                    Some(token) => {
                        chars.next(); // Consume
                        Some(token)
                    }
                    // No reserved token, it means we are parsing variable name
                    None => {
                        let mut variable_name = String::new();
                        loop {
                            match chars.peek() {
                                Some(c) => match match_reserved_token(*c) {
                                    Some(_) => break,
                                    None => {}
                                },
                                None => break,
                            }
                            let ch = chars.next().unwrap(); // Consume
                            variable_name.push(ch);
                        }
                        Some(Token::Symbol(variable_name))
                    }
                }
            })
        })
        .chain(once(Token::Eof))
}

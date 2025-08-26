use std::iter::{from_fn, once};

#[derive(Debug, Clone)]
pub enum Token {
    Symbol(String),
    OpenParen,
    CloseParen,
    Lambda,
    Dot,
    Pipe,
    With,
    In,
    Colon,
    Eof,
}

fn match_single_char_token(c: char) -> Option<Token> {
    match c {
        '(' => Some(Token::OpenParen),
        ')' => Some(Token::CloseParen),
        'λ' | '@' | '\\' => Some(Token::Lambda),
        '.' => Some(Token::Dot),
        '|' => Some(Token::Pipe),
        ':' => Some(Token::Colon),
        _ => None,
    }
}

/// Create a Token iterator from &str
pub fn lexer(input: &str) -> impl Iterator<Item = Token> {
    input
        .split_ascii_whitespace()
        .flat_map(|word| {
            let mut chars = word.chars().peekable();
            from_fn(move || {
                let c = chars.peek()?;
                match match_single_char_token(*c) {
                    Some(token) => {
                        chars.next(); // Consume
                        Some(token)
                    }
                    // No reserved token, it means we are parsing variable name
                    None => {
                        let mut variable_name = String::new();
                        while let Some(c) = chars.peek() {
                            if match_single_char_token(*c).is_some() {
                                break;
                            }
                            let ch = chars.next().unwrap(); // Consume
                            variable_name.push(ch);
                        }
                        Some(Token::Symbol(variable_name))
                    }
                }
            })
        })
        .map(|token| match token {
            Token::Symbol(name) if name == "with" => Token::With,
            Token::Symbol(name) if name == "in" => Token::In,
            _ => token,
        })
        .chain(once(Token::Eof))
}

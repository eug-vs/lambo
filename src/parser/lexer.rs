use std::iter::{from_fn, once};

#[derive(Debug, Clone)]
pub enum Token {
    Symbol(String),
    Quoted(String),
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
    let mut chars = input.chars().peekable();

    from_fn(move || {
        // Skip whitespace
        while let Some(_) = chars.next_if(|c| c.is_ascii_whitespace()) {}

        let c = chars.peek()?;

        // Check for single-char tokens
        if let Some(token) = match_single_char_token(*c) {
            chars.next(); // Consume
            return Some(token);
        }

        // Handle quoted strings
        if *c == '"' {
            chars.next(); // Consume opening quote
            let mut string_content = String::new();

            while let Some(ch) = chars.next() {
                if ch == '"' {
                    // Found closing quote
                    return Some(Token::Quoted(string_content));
                }
                if ch == '\\' {
                    // Handle escape sequences
                    if let Some(escaped) = chars.next() {
                        match escaped {
                            'n' => string_content.push('\n'),
                            't' => string_content.push('\t'),
                            'r' => string_content.push('\r'),
                            '\\' => string_content.push('\\'),
                            '"' => string_content.push('"'),
                            _ => {
                                string_content.push('\\');
                                string_content.push(escaped);
                            }
                        }
                    }
                } else {
                    string_content.push(ch);
                }
            }
            // Unclosed string - return what we have
            return Some(Token::Quoted(string_content));
        }

        // Parse variable name
        let mut variable_name = String::new();
        while let Some(c) = chars.next_if(|&c| {
            match_single_char_token(c).is_none() && !c.is_ascii_whitespace() && c != '"'
        }) {
            variable_name.push(c);
        }

        if variable_name.is_empty() {
            None
        } else {
            Some(Token::Symbol(variable_name))
        }
    })
    .map(|token| match token {
        Token::Symbol(name) if name == "with" || name == "let" => Token::With,
        Token::Symbol(name) if name == "in" => Token::In,
        _ => token,
    })
    .chain(once(Token::Eof))
}

use crate::lexers::token::Token;

pub struct Lexer;

impl Lexer {
    pub fn tokenize(input: &str) -> Vec<Token> {
        input
            .split_whitespace()
            .map(|word| match word {
                "|" => Token::Pipe,
                ";" => Token::Semicolon,
                "&" => Token::Ampersand,
                "(" => Token::LeftParen,
                ")" => Token::RightParen,
                _ => Token::Word(word.to_string()),
            })
            .collect()
    }
}

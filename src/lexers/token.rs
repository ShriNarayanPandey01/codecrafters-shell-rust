#[derive(Debug, PartialEq, Eq, Clone, PartialEq, Eq)]

pub enum Token {
    Word(String),
    Pipe,
    Semicolon,
    Ampersand,
    LeftParen,
    RightParen,
}
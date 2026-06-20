#[derive(Debug, PartialEq, Eq, Clone)]

pub enum Token {
    Word(String),
    RedirectStdout,
    RedirectStdoutAppend,
    RedirectStderr,
    Pipe,
    Semicolon,
    Ampersand,
    LeftParen,
    RightParen,
}

impl Token {
    pub fn as_word(&self) -> Option<&str> {
        match self {
            Token::Word(word) => Some(word.as_str()),
            _ => None,
        }
    }
}

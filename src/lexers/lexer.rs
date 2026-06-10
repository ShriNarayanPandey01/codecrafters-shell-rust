pub struct Lexer;

impl Lexer {
    pub fn tokenize(input: &str) -> Vec<Token>{
        let mut tokens = Vec::new();

        for word in input.split_whitespace() {
            match word {
                "|" => tokens.push(Token::Pipe),
                ";" => tokens.push(Token::Semicolon),
                "&" => tokens.push(Token::Background),
                "<" => tokens.push(Token::RedirectIn),
                ">" => tokens.push(Token::RedirectOut),
                _ => tokens.push(Token::Word(word.to_string())),
            }
        }
        tokens
    }
}
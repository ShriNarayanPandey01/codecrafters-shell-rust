use crate::lexers::token::Token;

pub struct Lexer;

impl Lexer {
    pub fn tokenize(input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_word = String::new();
        let mut chars = input.chars().peekable();
        let mut building_word = false;

        while let Some(ch) = chars.next() {
            match ch {
                '\'' => {
                    building_word = true;
                    for quoted_char in chars.by_ref() {
                        if quoted_char == '\'' {
                            break;
                        }

                        current_word.push(quoted_char);
                    }
                }
                '"' => {
                    building_word = true;
                    for quoted_char in chars.by_ref() {
                        if quoted_char == '"' {
                            break;
                        }

                        current_word.push(quoted_char);
                    }
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if building_word {
                        tokens.push(Token::Word(std::mem::take(&mut current_word)));
                        building_word = false;
                    }
                }
                '|' | ';' | '&' | '(' | ')' => {
                    if building_word {
                        tokens.push(Token::Word(std::mem::take(&mut current_word)));
                        building_word = false;
                    }

                    tokens.push(match ch {
                        '|' => Token::Pipe,
                        ';' => Token::Semicolon,
                        '&' => Token::Ampersand,
                        '(' => Token::LeftParen,
                        ')' => Token::RightParen,
                        _ => unreachable!(),
                    });
                }
                _ => {
                    building_word = true;
                    current_word.push(ch);
                }
            }
        }

        if building_word {
            tokens.push(Token::Word(current_word));
        }

        tokens
    }
}

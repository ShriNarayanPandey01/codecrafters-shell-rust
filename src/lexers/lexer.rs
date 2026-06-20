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
                '2' if !building_word && chars.peek() == Some(&'>') => {
                    chars.next();
                    tokens.push(Token::RedirectStderr);
                }
                '1' if !building_word && chars.peek() == Some(&'>') => {
                    chars.next();
                    tokens.push(Token::RedirectStdout);
                }
                '>' => {
                    if building_word {
                        tokens.push(Token::Word(std::mem::take(&mut current_word)));
                        building_word = false;
                    }

                    tokens.push(Token::RedirectStdout);
                }
                '\\' => {
                    if let Some(escaped_char) = chars.next() {
                        building_word = true;
                        current_word.push(escaped_char);
                    } else {
                        building_word = true;
                        current_word.push('\\');
                    }
                }
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
                    Self::consume_double_quoted(&mut chars, &mut current_word);
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

    fn consume_double_quoted(
        chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
        current_word: &mut String,
    ) {
        while let Some(quoted_char) = chars.next() {
            match quoted_char {
                '"' => break,
                '\\' => match chars.peek() {
                    Some('"') | Some('\\') => {
                        if let Some(escaped_char) = chars.next() {
                            current_word.push(escaped_char);
                        }
                    }
                    Some(_) => {
                        current_word.push('\\');
                    }
                    None => current_word.push('\\'),
                },
                _ => current_word.push(quoted_char),
            }
        }
    }
}

use crate::lexers::token::Token;
use crate::parser::ast::{ASTNode, RedirectStream};

pub struct Parser;

impl Parser {
    pub fn parse(tokens: Vec<Token>) -> Result<ASTNode, String> {
        if tokens.is_empty() {
            return Err("no tokens to parse".to_string());
        }

        Self::parse_pipeline(&tokens)
    }

    fn parse_pipeline(tokens: &[Token]) -> Result<ASTNode, String> {
        let segments = tokens.split(|token| matches!(token, Token::Pipe));
        let mut commands = Vec::new();

        for segment in segments {
            if segment.is_empty() {
                return Err("invalid null command".to_string());
            }

            commands.push(Self::parse_command(segment)?);
        }

        let mut nodes = commands.into_iter();
        let mut ast = nodes.next().ok_or_else(|| "no command found".to_string())?;

        for node in nodes {
            ast = ASTNode::Pipe {
                left: Box::new(ast),
                right: Box::new(node),
            };
        }

        Ok(ast)
    }

    fn parse_command(tokens: &[Token]) -> Result<ASTNode, String> {
        let mut words = Vec::new();
        let mut redirects = Vec::new();
        let mut index = 0;

        let mut last_is_background = false;
        if tokens.last() == Some(&Token::Ampersand) {
            if tokens.len() == 1 {
                return Err("expected command before &".to_string());
            }
            last_is_background = true;
        }

        while index < tokens.len() {
            if last_is_background && index == tokens.len() - 1 {
                break;
            }

            match &tokens[index] {
                Token::Word(word) => words.push(word.clone()),
                Token::RedirectStdout => {
                    let file = tokens
                        .get(index + 1)
                        .and_then(Token::as_word)
                        .ok_or_else(|| "expected file after redirection".to_string())?;

                    redirects.push((RedirectStream::Stdout, file.to_string()));
                    index += 2;
                    continue;
                }
                Token::RedirectStdoutAppend => {
                    let file = tokens
                        .get(index + 1)
                        .and_then(Token::as_word)
                        .ok_or_else(|| "expected file after redirection".to_string())?;

                    redirects.push((RedirectStream::StdoutAppend, file.to_string()));
                    index += 2;
                    continue;
                }
                Token::RedirectStderr => {
                    let file = tokens
                        .get(index + 1)
                        .and_then(Token::as_word)
                        .ok_or_else(|| "expected file after redirection".to_string())?;

                    redirects.push((RedirectStream::Stderr, file.to_string()));
                    index += 2;
                    continue;
                }
                Token::RedirectStderrAppend => {
                    let file = tokens
                        .get(index + 1)
                        .and_then(Token::as_word)
                        .ok_or_else(|| "expected file after redirection".to_string())?;

                    redirects.push((RedirectStream::StderrAppend, file.to_string()));
                    index += 2;
                    continue;
                }
                Token::Semicolon => return Err("sequences are not supported yet".to_string()),
                Token::Ampersand => {
                    return Err("background jobs are only supported at end of command".to_string());
                }
                Token::LeftParen | Token::RightParen => {
                    return Err("subshells are not supported yet".to_string());
                }
                Token::Pipe => return Err("unexpected pipe in command".to_string()),
            }

            index += 1;
        }

        let name = words
            .first()
            .cloned()
            .ok_or_else(|| "expected command name".to_string())?;
        let args = words.into_iter().skip(1).collect();

        let mut command = ASTNode::Command { name, args };
        for (stream, file) in redirects {
            command = ASTNode::Redirect {
                command: Box::new(command),
                file,
                stream,
            };
        }

        if last_is_background {
            command = ASTNode::Background {
                command: Box::new(command),
            };
        }

        Ok(command)
    }
}

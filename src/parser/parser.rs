use crate::lexers::token::Token;
use crate::parser::ast::ASTNode;

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

        for token in tokens {
            match token {
                Token::Word(word) => words.push(word.clone()),
                Token::Semicolon => return Err("sequences are not supported yet".to_string()),
                Token::Ampersand => return Err("background jobs are not supported yet".to_string()),
                Token::LeftParen | Token::RightParen => {
                    return Err("subshells are not supported yet".to_string())
                }
                Token::Pipe => return Err("unexpected pipe in command".to_string()),
            }
        }

        let name = words
            .first()
            .cloned()
            .ok_or_else(|| "expected command name".to_string())?;
        let args = words.into_iter().skip(1).collect();

        Ok(ASTNode::Command { name, args })
    }
}

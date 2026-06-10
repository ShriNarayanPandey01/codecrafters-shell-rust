#[derive(Debug, PartialEq, Eq, Clone, PartialEq, Eq)]

pub enum ASTNode {
    Command{
        name: String,
        args:Vec<String>
    },
    Pipe{
        left : Box<ASTNode>,
        right : Box<ASTNode>
    },
    Redirect{
        Command : Box<ASTNode>,
        file : String,
    }
}
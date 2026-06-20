#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RedirectStream {
    Stdout,
    Stderr,
}

#[derive(Debug, PartialEq, Eq, Clone)]

pub enum ASTNode {
    Command {
        name: String,
        args: Vec<String>,
    },
    Pipe {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    Redirect {
        command: Box<ASTNode>,
        file: String,
        stream: RedirectStream,
    },
}

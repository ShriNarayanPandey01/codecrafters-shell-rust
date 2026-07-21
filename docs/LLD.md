# Low-Level Design: `codecrafters-shell-rust`

## 1. Overview

This project is a custom shell written in Rust. It provides a REPL that reads
user input, tokenizes it, parses it into an AST, expands shell variables, and
executes either built-in commands or external programs.

The implementation is organized around these responsibilities:

- `main.rs`: REPL lifecycle and high-level orchestration
- `lexers/*`: input tokenization
- `parser/*`: AST construction
- `commands/*`: built-in commands and command execution logic
- `shell/*`: runtime shell state and autocomplete support
- `registry/*`: built-in command registration and lookup

## 2. Runtime Flow

Each input line follows this execution path:

1. Prompt user with `$ `
2. Read input using `rustyline`
3. Tokenize raw input with `Lexer::tokenize`
4. Parse tokens into `ASTNode` with `Parser::parse`
5. Expand `$VAR` and `${VAR}` using shell variables
6. Save command into in-memory history and `rustyline` history
7. Execute AST through the command execution layer
8. Update exit code and reap completed background jobs
9. Repeat until EOF or `exit`

## 3. Module Design

### 3.1 `src/main.rs`

`main.rs` owns the top-level shell loop:

- creates `CommandRegistry`
- creates `CompletionRegistry`
- creates mutable `ShellContext`
- loads history from `HISTFILE` if present
- configures `rustyline`
- runs the REPL loop
 - runs the REPL loop

It also supports a small set of command-line options for the shell process itself, for example `--path <dir>` to inject a custom directory into the runtime `PATH` so locally-built executables can be discovered during a session.
It also contains variable expansion helpers:

- `expand_variable_in_string`
- `expand_variables_in_ast`

This keeps parsing and execution independent of shell variable substitution.

### 3.2 `src/lexers`

#### `token.rs`

Defines token types used by the parser, including:

- `Word`
- `Pipe`
- `RedirectStdout`
- `RedirectStdoutAppend`
- `RedirectStderr`
- `RedirectStderrAppend`
- `Ampersand`
- `Semicolon`
- `LeftParen`
- `RightParen`

#### `lexer.rs`

`Lexer::tokenize(input: &str) -> Vec<Token>` converts raw input into tokens.

Supported lexical behavior:

- whitespace-separated words
- escaped characters with `\`
- single-quoted strings
- double-quoted strings with limited escaping
- `>`, `>>`, `1>`, `1>>`
- `2>`, `2>>`
- `|`
- `&`
- `;`
- `(`
- `)`

Design note:

- The lexer is intentionally simple and single-pass.
- It does not try to perform semantic validation.
- Unsupported constructs are still tokenized and later rejected by the parser.

### 3.3 `src/parser`

#### `ast.rs`

Defines the AST used by the executor.

```rust
ASTNode::Command { name, args }
ASTNode::Pipe { left, right }
ASTNode::Redirect { command, file, stream }
ASTNode::Background { command }
```

`RedirectStream` distinguishes:

- stdout overwrite
- stdout append
- stderr overwrite
- stderr append

#### `parser.rs`

`Parser::parse(tokens)` converts tokens into an AST.

Core behavior:

- pipelines are split on `|`
- each segment becomes a command subtree
- redirections wrap the command node
- trailing `&` wraps the command in `ASTNode::Background`

Current parser constraints:

- command sequences with `;` are rejected
- subshells `(...)` are rejected
- `&` is only allowed at the end
- empty pipeline segments are rejected

### 3.4 `src/registry`

#### `command_registry.rs`

`CommandRegistry` stores built-in commands in a `HashMap<String, Box<dyn BuiltInCommand>>`.

Responsibilities:

- register built-ins during shell startup
- expose `get_builtin(name)` lookup

Currently registered built-ins:

- `cat`
- `cd`
- `complete`
- `declare`
- `echo`
- `exit`
- `history`
- `jobs`
- `ls`
- `mkdir`
- `pwd`
- `rm`
- `touch`

`type` is handled as a special built-in-like branch in the execution layer.

### 3.5 `src/commands`

This folder has two categories:

- built-in command implementations
- execution helpers for built-in and external commands

#### Built-in command files

Each built-in implements the `BuiltInCommand` trait:

```rust
fn execute(
    &self,
    args: Vec<String>,
    context: &mut ShellContext,
    stdout: &mut dyn Write,
) -> Result<(), String>;
```

Built-in behavior summary:

- `cd`: changes current working directory and refreshes shell state
- `pwd`: prints current directory from `ShellContext`
- `echo`: prints arguments
- `exit`: terminates the shell process
- `declare`: creates or prints shell variables
- `history`: shows, reads, writes, and appends shell history
- `jobs`: shows tracked background jobs
- `complete`: registers, removes, or prints completion scripts
- `cat`: prints the contents of files to stdout
- `ls`: lists directory contents (basic listing; advanced flags are limited)
- `mkdir`: creates directories (supports `-p`-like behavior)
- `rm`: removes files or directories (supports `-r` for recursive removal)
- `touch`: creates an empty file or updates file timestamps

#### `execution.rs`

This module owns AST execution.

Key responsibilities:

- flatten command wrappers into an execution plan
- open redirection files
- dispatch built-ins
- dispatch external commands
- support background jobs
- support pipeline execution
- print completion notifications for finished jobs

Primary functions:

- `execute_ast`
- `flatten_command_execution`
- `reap_and_print_done_jobs`

Internal execution helpers:

- `execute_pipe`
- `execute_pipeline_stages`
- `execute_two_stage_pipeline`
- `execute_multi_stage_pipeline`
- `execute_command`
- `run_type_command`
- `open_redirect_file`

#### `external.rs`

This module isolates external program behavior:

- locate executable from `PATH`
- run foreground external commands
- spawn background jobs
- support pipe-connected external command execution

Primary functions:

- `find_command_in_path`
- `run_external_command`
- `run_external_command_background`
- `spawn_external_command`
- `execute_external_command_with_stdin`

### 3.6 `src/shell`

#### `shell_context.rs`

`ShellContext` stores mutable runtime state shared across commands.

Fields:

- `current_dir: String`
- `previous_exit_code: i32`
- `completions: CompletionRegistry`
- `background_jobs: Vec<BackgroundJob>`
- `history: Vec<String>`
- `last_saved_history_index: usize`
- `variables: HashMap<String, String>`

Responsibilities:

- track current working directory
- maintain command history
- manage background jobs
- store shell variables
- load/save history files

Related types:

- `BackgroundJob`
- `BackgroundJobStatus`

#### `built_in_command.rs`

Defines the common trait for all built-ins.

#### `completion_registry.rs`

Stores completion specs and disabled completion entries.

Implementation details:

- uses `Rc<RefCell<...>>`
- clonable across shell components
- supports dynamic registration/removal at runtime

#### `autocomplete.rs`

Integrates with `rustyline` completion APIs.

Completion sources:

- shell built-ins
- executables discovered from `PATH`
- file system paths
- user-registered completion scripts

Behavior summary:

- if cursor is at command position, suggest built-ins and executables
- otherwise resolve completion using command-specific script if registered
- fallback to file path completion

## 4. Core Data Structures

### 4.1 AST

The AST represents command structure after parsing and before execution.

- `Command` is the base executable unit
- `Redirect` decorates a command
- `Background` decorates a command
- `Pipe` combines commands into a pipeline

### 4.2 CommandExecution

`CommandExecution` is an internal flattened execution model used by the executor.

It separates:

- the final command node
- stdout redirection
- stderr redirection
- background execution flag

This allows the executor to process wrappers without deeply branching on the AST
at every stage.

### 4.3 ShellContext

`ShellContext` acts as the runtime state container for a shell session.

It is passed mutably into built-ins and execution helpers so command behavior can
update state in one place.

## 5. Execution Behavior

### 5.1 Built-in Commands

Built-ins are resolved through `CommandRegistry`. They execute in-process and can
mutate shell state directly.

This is required for commands like:

- `cd`
- `declare`
- `history`
- `complete`

### 5.2 External Commands

External commands are resolved from `PATH` and spawned as OS processes.

Foreground execution:

- uses `Command::status()`
- optionally redirects stdout/stderr to files

Background execution:

- uses `Command::spawn()`
- stores `Child` in `ShellContext.background_jobs`
- prints job id and process id

### 5.3 Pipelines

Pipelines are parsed into left-associated `ASTNode::Pipe` trees.

Execution behavior:

- single command pipelines fall back to normal execution
- two-stage pipelines support built-in/external combinations
- multi-stage pipelines currently support external commands only

Platform note:

- pipe execution is implemented under `#[cfg(unix)]`
- non-Unix builds reject pipeline execution

Note: the shell provides POSIX-like behavior but not all flags/constructs are supported on Windows. Certain command flags (e.g., `ls -la`), `echo -e` escape handling, and subshell/background constructs may behave differently or be unavailable on non-Unix builds. For full POSIX semantics run the shell under WSL, Git Bash, or a Unix-like host.

### 5.4 Redirection

Redirection is modeled in the AST, then flattened before execution.

Supported modes:

- `>` overwrite stdout
- `>>` append stdout
- `2>` overwrite stderr
- `2>>` append stderr

Redirection files are opened lazily during execution.

### 5.5 Variables

The shell supports user-defined variables through `declare`.

Expansion support:

- `$VAR`
- `${VAR}`

Expansion happens after parsing and before execution.

Unset variables expand to empty strings.

## 6. History and Job Management

### 6.1 History

History is tracked in memory in `ShellContext.history`.

Persistence behavior:

- optional load from `HISTFILE` at startup
- optional save to `HISTFILE` at shutdown
- built-in `history` supports read/write/append operations

### 6.2 Background Jobs

Background jobs are tracked as active `Child` processes with shell-level job ids.

Shell behavior:

- job ids are generated incrementally
- completed jobs are detected using `try_wait`
- finished jobs are printed before and after command execution

## 7. Error Handling Strategy

The project currently uses `Result<(), String>` heavily in the shell layers.

Advantages:

- simple control flow
- easy to print user-facing errors

Trade-off:

- limited structure for machine-readable errors
- some errors are user-facing and internal-facing at the same time

## 8. Current Limitations

As implemented today, the shell has these known constraints:

- no command sequences using `;`
- no subshell support
- no advanced shell grammar
- no environment variable export semantics
- no stdin-aware built-ins in pipelines
- multi-stage pipelines do not support built-ins
- pipeline execution is Unix-only
- no unit/integration design described for test coverage yet

## 9. Suggested Future Refactoring

The current structure is already improved by moving execution logic out of
`main.rs`, but the next cleanups could be:

- move variable expansion into its own module such as `src/shell/expansion.rs`
- move REPL setup into `src/shell/repl.rs`
- introduce structured error enums instead of raw `String`
- add test modules for lexer, parser, and executor behavior
- split pipeline execution from general execution into a dedicated module

## 10. Summary

This shell follows a classic command interpreter design:

- lexical analysis
- parsing into AST
- lightweight semantic transformation
- execution dispatch
- mutable shell session state

The codebase is small, understandable, and modular enough to evolve toward a
more complete shell implementation without changing its overall architecture.

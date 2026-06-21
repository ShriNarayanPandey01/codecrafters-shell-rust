# 🐚 BYOShell — Build Your Own Shell

A fully-featured, POSIX-inspired shell written from scratch in **Rust**. It includes a custom lexer, parser, AST-based execution engine, built-in commands, pipelines, I/O redirection, background jobs, tab completion, variable expansion, and persistent command history.

---

## ✨ Features

| Category | Details |
|---|---|
| **Custom Lexer & Parser** | Tokenizes raw input and builds an AST supporting commands, pipes, redirects, and background execution |
| **Built-in Commands** | `echo`, `cd`, `pwd`, `exit`, `type`, `history`, `jobs`, `declare`, `complete` |
| **External Commands** | Resolves and executes any program found in `$PATH` |
| **Pipelines** | Chain commands with `\|` — supports 2-stage and multi-stage pipelines |
| **I/O Redirection** | `>`, `>>`, `2>`, `2>>` for stdout and stderr redirection to files |
| **Background Jobs** | Run commands in the background with `&`, manage with `jobs` |
| **Variable Expansion** | `$VAR` and `${VAR}` syntax with `declare` for setting variables |
| **Tab Completion** | Built-in autocomplete via `rustyline` with custom completion scripts (`complete -C`) |
| **Command History** | Persistent history via `$HISTFILE`, with `history` command and `-r`, `-w`, `-a` flags |
| **Quoting** | Single quotes, double quotes, and escape character handling in the lexer |

---

## 🚀 Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.95 or later)

### Build

```bash
git clone https://github.com/ShriNarayanPandey01/codecrafters-shell-rust.git
cd codecrafters-shell-rust
cargo build --release
```

### Run

```bash
cargo run
```

You'll be greeted with the shell prompt:

```
$
```

### Showcase

A showcase script (`showcase.sh`) is included to quickly demonstrate the shell's core features. 

**On Linux / macOS / Git Bash:**
```bash
cargo run < showcase.sh
```

**On Windows PowerShell:**
```powershell
Get-Content showcase.sh | cargo run
```

---

## 📖 Usage

### Built-in Commands

#### `echo` — Print text to stdout
```bash
$ echo Hello, world!
Hello, world!
```

#### `pwd` — Print the current working directory
```bash
$ pwd
/home/user/projects
```

#### `cd` — Change directory
```bash
$ cd /tmp
$ cd ~          # Go to home directory
$ cd -          # Go to previous directory
```

#### `type` — Show whether a command is a built-in or an external program
```bash
$ type echo
echo is a shell builtin
$ type ls
ls is /usr/bin/ls
$ type nonexistent
nonexistent: not found
```

#### `exit` — Exit the shell
```bash
$ exit 0
```

#### `history` — View and manage command history
```bash
$ history          # Show all history
$ history 5        # Show last 5 entries
$ history -r file  # Read history from file
$ history -w file  # Write history to file
$ history -a file  # Append new history to file
```

Set the `HISTFILE` environment variable to persist history across sessions:
```bash
export HISTFILE=~/.byoshell_history
```

#### `declare` — Set and inspect shell variables
```bash
$ declare MY_VAR=hello
$ echo $MY_VAR
hello
$ declare -p MY_VAR
declare -- MY_VAR="hello"
```

#### `jobs` — List background jobs
```bash
$ sleep 10 &
[1] 12345
$ jobs
[1]+  Running                  sleep 10 &
```

#### `complete` — Manage tab-completion scripts
```bash
$ complete -C /path/to/script my_command   # Register completion script
$ complete -p my_command                    # Print completion spec
$ complete -r my_command                    # Remove completion spec
```

---

### Pipelines

Chain commands together — output of one feeds into the next:

```bash
$ echo "hello world" | cat
hello world

$ ls | grep ".rs" | head -5
```

Supports multi-stage pipelines with any number of external commands.

---

### I/O Redirection

```bash
$ echo "hello" > output.txt         # Write stdout to file
$ echo "more" >> output.txt         # Append stdout to file
$ ls nonexistent 2> errors.txt      # Redirect stderr to file
$ ls nonexistent 2>> errors.txt     # Append stderr to file
```

---

### Background Execution

```bash
$ sleep 30 &
[1] 54321
$ jobs
[1]+  Running                  sleep 30 &
```

Completed background jobs are automatically reported when the shell is ready for the next command.

---

### Variable Expansion

```bash
$ declare greeting=hello
$ echo $greeting
hello
$ echo ${greeting}_world
hello_world
```

---

## 🏗️ Architecture

```
src/
├── main.rs                     # Entry point, REPL loop, AST execution engine
├── commands/                   # Built-in command implementations
│   ├── cd.rs                   #   cd — change directory
│   ├── complete.rs             #   complete — manage tab completions
│   ├── declare.rs              #   declare — shell variables
│   ├── echo.rs                 #   echo — print to stdout
│   ├── exit.rs                 #   exit — terminate the shell
│   ├── history.rs              #   history — command history management
│   ├── jobs.rs                 #   jobs — background job listing
│   └── pwd.rs                  #   pwd — print working directory
├── lexers/
│   ├── lexer.rs                # Tokenizer — transforms raw input into tokens
│   └── token.rs                # Token type definitions
├── parser/
│   ├── ast.rs                  # AST node definitions (Command, Pipe, Redirect, Background)
│   └── parser.rs               # Recursive descent parser
├── registry/
│   └── command_registry.rs     # Registers and looks up built-in commands
└── shell/
    ├── autocomplete.rs         # Tab-completion integration with rustyline
    ├── built_in_command.rs     # BuiltInCommand trait definition
    ├── completion_registry.rs  # Stores custom completion scripts
    └── shell_context.rs        # Shell state: cwd, history, jobs, variables
```

### How It Works

1. **Read** — `rustyline` presents the `$ ` prompt and reads a line of input with line-editing and tab completion.
2. **Lex** — The `Lexer` tokenizes the input into `Token` values (words, operators, redirects, pipes, etc.).
3. **Parse** — The `Parser` builds an `ASTNode` tree representing the command structure.
4. **Expand** — `$VAR` and `${VAR}` references are expanded using the shell's variable map.
5. **Execute** — The AST is walked recursively:
   - **Built-in commands** are dispatched through the `CommandRegistry`.
   - **External commands** are resolved via `$PATH` and executed with `std::process::Command`.
   - **Pipes** create OS-level pipes connecting child processes.
   - **Redirections** open files and rewire stdout/stderr.
   - **Background jobs** are spawned and tracked for later status reporting.
6. **Record** — The command is appended to the in-memory history (and optionally persisted to `$HISTFILE` on exit).

---

## 📦 Dependencies

| Crate | Purpose |
|---|---|
| [`rustyline`](https://crates.io/crates/rustyline) | Line editing, history, and tab completion |
| [`anyhow`](https://crates.io/crates/anyhow) | Ergonomic error handling |
| [`thiserror`](https://crates.io/crates/thiserror) | Derive macros for custom error types |
| [`libc`](https://crates.io/crates/libc) | Low-level POSIX system calls (pipes, permissions) |
| [`bytes`](https://crates.io/crates/bytes) | Efficient byte buffer management |

---

## 📄 License

This project is open source. Feel free to use, modify, and distribute.

---

## 🙋 Author

**Shri Narayan Pandey** — [GitHub](https://github.com/ShriNarayanPandey01)

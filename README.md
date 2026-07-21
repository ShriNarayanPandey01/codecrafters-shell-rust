# 🐚 BYOShell — Build Your Own Shell

A fully-featured, POSIX-inspired shell written from scratch in **Rust**. It includes a custom lexer, parser, AST-based execution engine, built-in commands, pipelines, I/O redirection, background jobs, tab completion, variable expansion, and persistent command history.

---

## ✨ Features

| Category | Details |
|---|---|
| **Custom Lexer & Parser** | Tokenizes raw input and builds an AST supporting commands, pipes, redirects, and background execution |
| **Built-in Commands** | `echo`, `cd`, `pwd`, `exit`, `type`, `history`, `jobs`, `declare`, `complete`, `cat`, `mkdir`, `rm`, `touch` |
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

### Run With a Custom Executable Path

```bash
cargo run -- --path "/custom/executables"
```

This injects the specified directory into the shell's runtime `$PATH`, making locally built tools available to executed commands.

### Run As An API Server

```bash
cargo run -- serve 7878
```

By default the API binds to `0.0.0.0` and will also honor the `PORT` environment variable when no explicit port argument is provided. Set `BYOSHELL_HOST` if you want to override the bind address.

The HTTP server requires `BYOSHELL_API_KEY` to be set before it will start:

```bash
BYOSHELL_API_KEY="replace-this-with-a-secret" cargo run -- serve 7878
```

Optional rate limiting environment variables:

```bash
BYOSHELL_RATE_LIMIT_MAX_REQUESTS=60
BYOSHELL_RATE_LIMIT_WINDOW_SECS=60
```

`GET /health` stays public for health checks. Other endpoints require either `Authorization: Bearer <key>` or `X-API-Key: <key>`.

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

## 🔧 Test Suite

A simple smoke test script is provided at `tests/test_suite.sh` to exercise built-ins, redirection, and basic file operations.

Run the test suite (PowerShell):

```powershell
Get-Content tests/test_suite.sh | cargo run
```

Notes:
- The shell is under active development; on Windows some POSIX flags and constructs (e.g., `ls -l`, `echo -e`, subshell backgrounding, and some pipeline behaviors) may behave differently or be unsupported. Use WSL, Git Bash, or Linux/macOS for fuller POSIX compatibility.
- The test script is written to use BYOShell-compatible constructs where possible; adjust it if you extend builtin behaviors.

---

## HTTP API

The same shell engine can now be used through HTTP, which makes it easy to connect to a portfolio frontend.

### Endpoint

`POST /execute`

Authentication required:

```http
Authorization: Bearer your-secret-api-key
```

### Request Body

```json
{
  "session_id": "portfolio",
  "command": "echo hello"
}
```

- `session_id` keeps shell state alive across requests, including `cd`, `declare`, history, and background jobs.
- `command` is the shell command to execute.

You can also send plain text instead of JSON. In that case, the server uses the default session.

### Response Body

```json
{
  "session_id": "portfolio",
  "command": "echo hello",
  "stdout": "hello\n",
  "stderr": "",
  "exit_code": 0,
  "current_dir": "/path/to/project",
  "should_exit": false
}
```

### Health Check

`GET /health`

This endpoint is intentionally left unauthenticated so hosting providers such as Render can probe the service.

### Rate Limiting

Authenticated requests are rate limited in memory by client IP address. The defaults are:

- `60` requests
- per `60` seconds

You can override that with `BYOSHELL_RATE_LIMIT_MAX_REQUESTS` and `BYOSHELL_RATE_LIMIT_WINDOW_SECS`.

### Portfolio Frontend Example

```js
async function runShellCommand(command) {
  const response = await fetch("http://127.0.0.1:7878/execute", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer your-secret-api-key",
    },
    body: JSON.stringify({
      session_id: "portfolio",
      command,
    }),
  });

  if (!response.ok) {
    throw new Error(`Request failed with ${response.status}`);
  }

  return response.json();
}
```

This lets your portfolio send commands and render `stdout`, `stderr`, and `current_dir` in the UI while keeping the terminal experience intact.

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

#### `cat` — Print file contents to stdout
```bash
$ cat README.md
```

#### `mkdir` — Create directories
```bash
$ mkdir new-folder
$ mkdir -p nested/folder
```

#### `rm` — Remove files and directories
```bash
$ rm file.txt
$ rm -r dir-to-remove
```

#### `touch` — Create an empty file or update file timestamp
```bash
$ touch empty.txt
$ touch existing.txt
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
│   ├── cat.rs                  #   cat — print file contents
│   ├── cd.rs                   #   cd — change directory
│   ├── complete.rs             #   complete — manage tab completions
│   ├── declare.rs              #   declare — shell variables
│   ├── echo.rs                 #   echo — print to stdout
│   ├── exit.rs                 #   exit — terminate the shell
│   ├── history.rs              #   history — command history management
│   ├── jobs.rs                 #   jobs — background job listing
│   ├── ls.rs                   #   ls — list directory contents
│   ├── mkdir.rs                #   mkdir — create directories
│   ├── pwd.rs                  #   pwd — print working directory
│   ├── rm.rs                   #   rm — remove files and directories
│   └── touch.rs                #   touch — create files / update timestamps
├── external.rs                # External command lookup and execution support
├── engine.rs                  # Core execution engine for command ASTs
├── server.rs                  # HTTP server entrypoint (API mode)
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

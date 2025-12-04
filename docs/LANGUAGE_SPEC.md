# OtterLang Language Specification

This document describes the syntax and semantics implemented by the current OtterLang compiler, runtime, LSP server, and standard library. Code samples use the fully supported `fn` syntax and reflect the features shipped in this repository.

## Table of Contents

1. [Lexical Structure](#lexical-structure)
2. [Type System](#type-system)
3. [Expressions](#expressions)
4. [Statements](#statements)
5. [Functions and Methods](#functions-and-methods)
6. [Structs](#structs)
7. [Enums](#enums)
8. [Pattern Matching](#pattern-matching)
9. [Modules and Visibility](#modules-and-visibility)
10. [Concurrency Primitives](#concurrency-primitives)
11. [Error Handling](#error-handling)
12. [Standard Library Overview](#standard-library-overview)
13. [Grammar Summary](#grammar-summary)
14. [Semantics and Implementation Notes](#semantics-and-implementation-notes)

## Lexical Structure

### Comments

OtterLang uses `#` for single-line comments. Multi-line comments are created by using multiple `#` lines.

```otter
# This is a comment
# Multi-line comments use multiple hash lines
```

### Whitespace and Indentation

OtterLang is indentation-sensitive. Statements are grouped using indentation levels:

- A colon (`:`) introduces a new indentation block
- Block headers include: `fn`, `if`, `elif`, `else`, `for`, `while`, `match`, `struct`, `enum`
- All indentation must use spaces (tabs are not allowed)
- The standard indentation is 4 spaces

### Identifiers

Identifiers start with a letter or underscore and may contain ASCII letters, digits, or underscores. Unicode identifiers are also accepted. The standalone underscore (`_`) is treated as the wildcard identifier in patterns.

### Keywords

The following words are reserved keywords and cannot be used as identifiers:

**Control Flow:**
`if`, `elif`, `else`, `for`, `while`, `break`, `continue`, `pass`, `return`, `match`, `case`

**Functions and Types:**
`fn`, `struct`, `enum`, `type`, `let`

**Modules and Visibility:**
`use`, `pub`, `as`

**Concurrency:**
`async`, `await`, `spawn`

**Operators:**
`and`, `or`, `not`, `in`, `is`

**Literals:**
`true`, `false`, `None`

**Other:**
`print`, `pass`

**Contextual Keywords:**
- `type` - recognized only at the start of type alias declarations

### Literals

- **Numbers** support underscores for readability and may be written as integers (`42`, `1_000`) or floating-point values (`3.14`, `2.0e-3`).
- **Strings** use single or double quotes. Prefix a string with `f` to enable interpolation with `{expr}` placeholders.
- **Booleans** are `true` and `false`.
- **None/Unit** literals are written as `None`/`none` or as the empty tuple `()`.

## Type System

OtterLang uses a static type system with inference. Type annotations are optional but recommended for public APIs.

### Built-in Types

| Type | Description |
|------|-------------|
| `int` / `i64` | 64-bit signed integer |
| `i32` | 32-bit signed integer |
| `float` / `f64` | 64-bit floating point |
| `bool` | Boolean value |
| `str` / `string` | UTF-8 string |
| `unit` / `None` | Unit type (absence of value) |
| `list<T>` | Dynamic array of type T |
| `dict<K, V>` | Dictionary mapping keys of type K to values of type V |

Any other identifier is treated as a custom type name (e.g., `User`, `Channel<string>`). For dynamic typing, use the conventional `any` type alias.

### Type Annotations

Annotate bindings, parameters, and return types with a colon:

```otter
let name: string = "Otter"
let values: list<int> = [1, 2, 3]
fn len_text(text: string) -> int:
    return len(text)
```

### Generics

Functions, structs, enums, and type aliases support generic parameters:

```otter
fn first<T>(items: list<T>) -> T:
    return items[0]

struct Pair<T, U>:
    first: T
    second: U
```

### Type Aliases

Define aliases with the contextual `type` keyword:

```otter
pub type UserId = int
pub type Response<T> = Result<T, Error>
```

## Expressions

### Arithmetic and Comparison

OtterLang supports `+`, `-`, `*`, `/`, and `%`. The `+` operator also performs string concatenation, automatically converting integers, floats, and booleans to strings. Comparison operators include `==`, `!=`, `<`, `>`, `<=`, `>=`, `is`, and `is not`.

```otter
let normalized = (value - min) / (max - min)
if count is not None and count > 0:
    print("ready")
```

### Logical Operators

Use `and`, `or`, and `not` for boolean logic.

```otter
if is_ready and not has_failed:
    proceed()
```

### Function and Method Calls

Call syntax uses parentheses. Methods are regular functions stored inside structs, so you call them with the dot operator: `point.distance()`.

### Member Access and Namespaces

Use `object.field` or `Module.symbol`. Enum variants use the same syntax: `Option.Some(value)`.

### Struct Instantiation

Structs use keyword-style arguments:

```otter
let origin = Point(x=0.0, y=0.0)
```

### Collection Literals

```otter
let numbers = [1, 2, 3]
let mapping = {"a": 1, "b": 2}
```

### Comprehensions

Lists and dictionaries support comprehension syntax with an optional `if` filter:

```otter
let squares = [x * x for x in 0..10]
let indexed = {x: idx for idx in 0..len(items) if items[idx] != None}
```

### Range Expressions

`start..end` produces a range expression. Ranges are evaluated eagerly inside `for` loops and are exclusive of `end`.

```otter
for i in 0..count:
    println(str(i))
```

### Anonymous Functions

Anonymous functions can be created using `fn` syntax:

```otter
let doubler = fn (value: int) -> int: value * 2
let handler = fn (event):
    if event.kind == "update":
        process(event)
```

### Await and Spawn

`await` consumes the result of an asynchronous computation. `spawn` starts an asynchronous computation and returns a task handle.

```otter
let task = spawn fetch_data(url)
let payload = await task
```

### F-Strings and Interpolation

Prefix strings with `f` to embed arbitrary expressions:

```otter
let summary = f"Processed {len(items)} items in {duration_ms}ms"
```

## Statements

### Variable Declarations and Assignment

Use `let` to introduce bindings. `pub let` exports a binding from the current module.

```otter
let total = 0.0
pub let version: string = runtime.version()
```

Simple reassignments omit `let`:

```otter
total = total + chunk
items += [extra]
```

### Expression Statements

Any expression can appear as a statement. This is how function calls and comprehensions that produce side effects are executed.

### Control Flow

#### `if` / `elif` / `else`

```otter
if size == 0:
    return
elif size < 10:
    print("small batch")
else:
    print("large batch")
```

#### `while`

```otter
while remaining > 0:
    remaining -= 1
```

#### `for`

`for` iterates over any iterable expression. Ranges are the easiest way to create numeric loops.

```otter
for user in users:
    println(user.name)
```

#### `match`

`match` dispatches on patterns. Guards (`case ... if ...`) are not supported in the current grammar.

```otter
let description = match result:
    case Result.Ok(value):
        f"ok: {value}"
    case Result.Err(error):
        f"error: {error}"
```

#### Error Handling with `Result<T, E>`

OtterLang uses `Result<T, E>` enum for error handling instead of exceptions. Functions return `Result.Ok(value)` for success or `Result.Err(error)` for errors.

```otter
fn divide(x: float, y: float) -> Result<float, string>:
    if y == 0.0:
        return Result.Err("Division by zero")
    return Result.Ok(x / y)

match divide(10.0, 2.0):
    case Result.Ok(value):
        println(f"Result: {value}")
    case Result.Err(error):
        println(f"Error: {error}")
```

For unrecoverable errors, use `panic(message)` from the standard library.

### Loop Control

Use `break`, `continue`, and `pass` inside loops or placeholders. `return` exits the current function.

## Functions and Methods

Functions use the following syntax:

```otter
pub fn greet(name: string, greeting: string = "Hello") -> string:
    return f"{greeting}, {name}!"

fn main():
    println("Hello, World!")
```

- Functions are declared with `fn` followed by the function name, parameters in parentheses, optional return type, and a colon
- Parameters can have default values. Once a parameter declares a default, all subsequent parameters must also declare defaults
- Functions may declare type parameters: `fn parse<T>(text: string) -> T`
- Nested functions are allowed
- Method definitions live inside `struct` blocks and take `self` explicitly as the first parameter

Top-level code may only contain function definitions, `let` statements, struct/enum/type declarations, `use`/`pub use` statements, and expression statements. Other control-flow constructs must appear inside functions.

## Structs

Structs group named fields and optional methods.

```otter
pub struct Point:
    x: float
    y: float

    fn distance(self) -> float:
        return math.sqrt(self.x * self.x + self.y * self.y)
```

Instantiate structs with keyword arguments: `Point(x=3.0, y=4.0)`.

Struct definitions can declare generics: `struct Box<T>:`.

## Enums

Enums define tagged unions. Variants either carry payloads or act as unit variants.

```otter
pub enum Result<T, E>:
    Ok: (T)
    Err: (E)
```

Construct variants via `Result.Ok(value)`/`Result.Err(error)` and pattern match on them in `match` expressions.

## Pattern Matching

Patterns allow destructuring and conditional matching in `match` expressions and `let` bindings:

| Pattern | Example | Description |
|---------|---------|-------------|
| Wildcard | `_` | Matches any value, ignores it |
| Variable | `name` | Binds the matched value to a variable |
| Literal | `42`, `"hello"`, `true` | Matches exact values |
| Enum | `Result.Ok(value)` | Matches enum variants with payloads |
| Struct | `Point{x, y}` | Destructures struct fields |
| List | `[head, ..rest]` | Matches list elements |

Patterns are used in:
- `match` expression case clauses
- Destructuring `let` bindings
- Function parameters (planned)

## Modules and Visibility

Each `.ot` file defines a module. Items are private by default. Mark functions, structs, enums, `let` bindings, and type aliases with `pub` to export them. The compiler supports two import forms:

```otter
use std/io as io
use math, std/time as time

pub use core.Option
pub use math.sqrt as square_root
```

Only the built-in primitives (enums, `Option`/`Result`, `panic`, `print`, `len`, and the core String/List/Map helpers plus arithmetic) live in the implicit prelude. Every other stdlib module—`http`, `json`, `io`, `sys`, `net`, `runtime`, `task`, etc.—must be imported with `use module_name` before its dotted members (`module.fn`) become visible.

Module paths consist of segments separated by `/` or `:` (`use std/io`) and may start with `.` or `..` for relative imports. Transparent Rust FFI uses the same mechanism (`use rust:serde/json`).

`pub use` statements re-export items or entire modules. `pub use math` re-exports everything that `math` already exposes as `pub`.

## Concurrency Primitives

OtterLang currently ships two levels of concurrency support:

1. **Language-level operators**: `spawn fn_call(...)` runs a function asynchronously and returns a task handle. `await handle` waits for completion and yields the underlying result.
2. **Standard library**: `stdlib/otter/task.ot` exposes helpers for spawning tasks, joining/detaching them, sleeping, working with typed channels, and building `select` statements.

Example:

```otter
let worker = spawn process_batch(batch)
let snapshot = spawn fetch_snapshot()
let batch_result = await worker
let snapshot_result = await snapshot
```

## Error Handling

OtterLang uses `Result<T, E>` enum for error handling. Functions return `Result.Ok(value)` for success or `Result.Err(error)` for errors. Pattern matching with `match` is used to handle results.

- `Result<T, E>` and `Option<T>` live in `stdlib/otter/core.ot` and provide algebraic error handling.
- `panic(message)` is a built-in for unrecoverable failures.
- Use `match` expressions to handle `Result` and `Option` values.

## Standard Library Overview

The `stdlib/otter` directory contains the modules shipped with the compiler. Import them with `use` statements.

- **builtins** – fundamental helpers such as `len`, `cap`, list/map mutation, `panic`, `recover`, `type_of`, `append`, `range`, and structured error utilities (`try_func`, `select`, `defer`).
- **core** – definitions of `Option<T>` and `Result<T, E>`.
- **fmt** – lightweight wrappers around standard output (`print`, `println`, `eprintln`).
- **fs** – filesystem helpers: `exists`, `mkdir`, `remove`, `list_dir`, file IO shortcuts, etc.
- **http** – convenience wrappers for HTTP verbs built on the runtime networking stack.
- **io** – file IO plus buffered IO helpers.
- **json** – encoding/decoding JSON strings, pretty printing, and validation.
- **math** – numeric algorithms (`sqrt`, `pow`, `exp`, `clamp`, `randf`, etc.).
- **net** – TCP-style networking primitives plus HTTP response helpers.
- **rand** – RNG seeding plus integer/float random generators.
- **runtime** – introspection helpers (`gos`, `cpu_count`, `memory`, `stats`, `version`).
- **task** – task spawning, sleeping, typed channels, and select utilities.
- **time** – timestamps, sleeping, timers, formatting, and parsing.

Each module is pure OtterLang code and may be used as a reference for idiomatic syntax.

## Grammar

The summary below mirrors the parser implementation in [`crates/parser/src/grammar.rs`](../crates/parser/src/grammar.rs). Whitespace and indentation management are omitted for brevity. Keywords are shown in `bold`, literals in *italics*, and optional elements in `[square brackets]`.

### Program Structure

```
program         := (statement | use_stmt | pub_use_stmt | type_alias | struct_def | enum_def | function)*
statement       := let_stmt | assignment_stmt | augmented_assignment | return_stmt
                   | break_stmt | continue_stmt | pass_stmt | if_stmt | while_stmt
                   | for_stmt | match_stmt | expr_stmt
```

### Modules and Imports

```
use_stmt        := "use" module_path ["as" identifier]
pub_use_stmt    := "pub" "use" module_path ["as" identifier]
module_path     := identifier (":" identifier | "/" identifier)*
```

### Types and Type Definitions

```
type            := identifier ["<" type_args ">"] | primitive_type | func_type
type_args       := type ("," type)*
primitive_type  := "int" | "i32" | "float" | "f64" | "bool" | "str" | "string" | "unit"
func_type       := "(" [type ("," type)*] ")" "->" type
type_alias      := ["pub"] "type" identifier ["<" type_params ">"] "=" type
type_params     := identifier ("," identifier)*
```

### Functions

```
function        := ["pub"] "fn" identifier ["<" type_params ">"] "(" [params] ")" ["->" type] ":" block
params          := param ("," param)*
param           := identifier ":" type ["=" expr]
block           := NEWLINE INDENT statement* DEDENT
```

### Structs and Enums

```
struct_def      := ["pub"] "struct" identifier ["<" type_params ">"] ":" NEWLINE INDENT (struct_field | method)* DEDENT
struct_field    := identifier ":" type
method          := "fn" identifier "(" [params] ")" ["->" type] ":" block

enum_def        := ["pub"] "enum" identifier ["<" type_params ">"] ":" NEWLINE INDENT enum_variant+ DEDENT
enum_variant    := identifier ["(" type ")"]
```

### Expressions

```
expr            := primary_expr | unary_expr | binary_expr | await_expr
                   | spawn_expr | range_expr | comprehension | if_expr | match_expr

primary_expr    := literal | identifier | "(" expr ")" | struct_init | list_literal
                   | dict_literal | call_expr | member_expr | index_expr

literal         := INTEGER | FLOAT | STRING | "true" | "false" | "None" | "()"
INTEGER         := [0-9]+ | "0x" [0-9a-fA-F]+ | "0b" [01]+
FLOAT           := [0-9]+ "." [0-9]* | [0-9]+ ("e"|"E") ["+"|"-"] [0-9]+
STRING          := "\"" ([^"\\] | "\\" .)* "\"" | "'" ([^'\\] | "\\" .)* "'"

struct_init     := identifier "(" [field_init ("," field_init)*] ")"
field_init      := identifier "=" expr
list_literal    := "[" [expr ("," expr)*] "]"
dict_literal    := "{" [dict_entry ("," dict_entry)*] "}"
dict_entry      := expr ":" expr

call_expr       := expr "(" [expr ("," expr)*] ")"
member_expr     := expr "." identifier
index_expr      := expr "[" expr "]"

await_expr      := "await" expr
spawn_expr      := "spawn" expr
range_expr      := expr ".." expr
comprehension   := "[" expr "for" identifier "in" expr ["if" expr] "]"
                 | "{" expr ":" expr "for" identifier "in" expr ["if" expr] "}"

if_expr         := "if" expr ":" expr ("elif" expr ":" expr)* ["else" ":" expr]
```

### Statements

```
let_stmt        := ["pub"] "let" identifier [":" type] "=" expr
assignment_stmt := identifier "=" expr
augmented_assignment := identifier ("+=" | "-=" | "*=" | "/=" | "%=") expr

return_stmt     := "return" [expr]
break_stmt      := "break"
continue_stmt   := "continue"
pass_stmt       := "pass"

if_stmt         := "if" expr ":" block ("elif" expr ":" block)* ["else" ":" block]
while_stmt      := "while" expr ":" block
for_stmt        := "for" identifier ["in" expr] ":" block

match_stmt      := "match" expr ":" NEWLINE INDENT match_case+ DEDENT
match_case      := "case" pattern ":" block
```

### Patterns

```
pattern         := wildcard_pattern | literal_pattern | identifier_pattern
                   | enum_pattern | struct_pattern | list_pattern

wildcard_pattern    := "_"
literal_pattern     := literal
identifier_pattern  := identifier
enum_pattern        := identifier "." identifier ["(" pattern ("," pattern)* ")"]
struct_pattern      := identifier "{" [field_pattern ("," field_pattern)*] "}"
field_pattern       := identifier [":" pattern]
list_pattern        := "[" [pattern ("," pattern)* ["," ".." identifier]] "]"
```

### Operators and Precedence

Operators are listed from highest to lowest precedence:

```
Primary:     () [] . () (function call)
Unary:       not - +
Multiplicative: * / %
Additive:    + -
Comparison:  == != < <= > >= is is not
Logical AND: and
Logical OR:  or
Range:       ..
```

### Lexical Structure

```
identifier      := [a-zA-Z_][a-zA-Z0-9_]*
keyword         := fn | let | return | if | elif | else | for | while
                   | match | case | struct | enum
                   | type | pub | async | await | spawn | true | false | None
                   | and | or | not | in | is | as | use | break | continue | pass | print
comment         := "#" [^\n]*
whitespace      := [ \t\n\r]+
```

## Semantics and Implementation Notes

- **Type Checking** – Static type checking with inference is performed before code generation. Generic parameters default to unconstrained type variables.
- **Evaluation Order** – Expressions evaluate left-to-right. Function arguments are evaluated before the call.
- **Memory Management** – The runtime manages memory automatically using reference counting and runtime support utilities in `runtime/`.
- **Code Generation** – The `otter` binary can target LLVM or Cranelift backends. Both backends eventually emit machine code to run programs natively.
- **Tooling** – The repository ships a formatter, language server, REPL, and VS Code syntax highlighter that all understand the syntax described in this document.

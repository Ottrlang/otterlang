# OtterLang Tutorial Series

Welcome to OtterLang! This tutorial series will guide you through learning the language from basic concepts to advanced features.

## Table of Contents

1. [Installation](#installation)
2. [Your First Program](#your-first-program)
3. [Variables and Types](#variables-and-types)
4. [Functions](#functions)
5. [Control Flow](#control-flow)
6. [Collections](#collections)
7. [Structs and Types](#structs-and-types)
8. [Concurrency](#concurrency)
9. [Error Handling](#error-handling)
10. [Advanced Topics](#advanced-topics)

## Installation

See the [Installation Guide](INSTALLATION.md) for detailed setup instructions.

## Your First Program

Let's create your first OtterLang program. Create a file called `hello.ot`:

```otter
fn main():
    println("Hello, World!")
```

Now run it using the Otter CLI:

```bash
otter run hello.ot
```

You should see the output: `Hello, World!`

**Note:** `println()` automatically adds a newline after the text, while `print()` does not.

## Variables and Types

OtterLang supports static typing with type inference. You can explicitly annotate types or let the compiler infer them.

### Basic Types

```otter
fn main():
    # Integers (64-bit signed)
    x = 42

    # Floating-point numbers (64-bit)
    pi = 3.14

    # Strings (UTF-8)
    name = "Otter"

    # Booleans
    is_active = true

    # Arrays (dynamic, homogenous)
    numbers = [1, 2, 3, 4, 5]

    # Dictionaries (key-value maps)
    person = {"name": "Otter", "age": 42}
```

### Type Annotations

While type annotations are optional, they're recommended for public APIs:

```otter
fn main():
    x: int = 42
    name: string = "Otter"
    numbers: list<int> = [1, 2, 3]
    person: dict<string, any> = {"name": "Otter", "age": 42}
```

## Functions

Functions in OtterLang use indentation-based syntax and support type annotations.

### Basic Functions

```otter
fn greet(name: string) -> string:
    return f"Hello, {name}!"

fn main():
    message = greet("World")
    println(message)
```

### Function Parameters

Functions can have multiple parameters with default values:

```otter
fn add(x: int, y: int) -> int:
    return x + y

fn greet(name: string, prefix: string = "Hello") -> string:
    return f"{prefix}, {name}!"

fn main():
    result = add(10, 20)
    println(f"Result: {result}")

    message = greet("World")           # Uses default prefix
    custom = greet("Otter", "Hi")      # Custom prefix
    println(message)
    println(custom)
```

## Control Flow

OtterLang provides several control flow constructs for conditional execution and loops.

### Conditional Statements

```otter
fn check_age(age: int):
    if age >= 18:
        println("Adult")
    elif age >= 13:
        println("Teenager")
    else:
        println("Child")
```

### Pattern Matching

```otter
fn day_name(day: int) -> string:
    return match day:
        case 1:
            "Monday"
        case 2:
            "Tuesday"
        case 3:
            "Wednesday"
        case _:
            "Unknown"

fn describe_value(value):
    return match value:
        case 0:
            "zero"
        case value:
            if value > 0:
                "positive"
            else:
                "negative"
```

### Loops

```otter
fn main():
    # While loop with variable
    let i = 0
    while i < 10:
        println(f"Count: {i}")
        i = i + 1

    # For loop over collection
    for num in [1, 2, 3, 4, 5]:
        println(f"Number: {num}")

    # For loop with range
    for i in 0..5:
        println(f"Index: {i}")
```

## Collections

OtterLang supports dynamic arrays and dictionaries for storing collections of data.

### Arrays

Arrays are dynamic, ordered collections that can grow and shrink:

```otter
fn main():
    # Create an array
    numbers = [1, 2, 3, 4, 5]

    # Access elements (0-based indexing)
    first = numbers[0]    # 1
    last = numbers[4]     # 5

    # Get length
    count = len(numbers)  # 5

    # Iterate over elements
    for num in numbers:
        println(f"Number: {num}")

    # Add elements
    numbers += [6, 7, 8]

    # List comprehensions
    squares = [x * x for x in 1..6]  # [1, 4, 9, 16, 25]
```

### Dictionaries

Dictionaries store key-value pairs:

```otter
fn main():
    # Create a dictionary
    person = {"name": "Otter", "age": 42, "active": true}

    # Access values
    name = person["name"]      # "Otter"
    age = person["age"]        # 42

    # Add/modify entries
    person["email"] = "otter@example.com"

    # Check if key exists
    if "name" in person:
        println("Name found!")

    # Iterate over keys and values
    for key in person:
        println(f"{key}: {person[key]}")

    # Dictionary comprehensions
    squares = {x: x * x for x in 1..6}  # {1: 1, 2: 4, 3: 9, 4: 16, 5: 25}
```

## Structs and Types

Structs allow you to define custom data types with named fields.

### Defining Structs

```otter
struct Point:
    x: float
    y: float

    fn distance_from_origin(self) -> float:
        return math.sqrt(self.x * self.x + self.y * self.y)

struct Person:
    name: string
    age: int
    email: string

fn main():
    # Create struct instances
    origin = Point(x=0.0, y=0.0)
    point = Point(x=3.0, y=4.0)

    # Access fields
    println(f"Point: ({point.x}, {point.y})")

    # Call methods
    distance = point.distance_from_origin()
    println(f"Distance: {distance}")

    # Create person
    person = Person(name="Alice", age=30, email="alice@example.com")
    println(f"Hello, {person.name}!")
```

### Type Aliases

Type aliases provide alternative names for existing types:

```otter
type UserId = int
type Email = string
type Coordinates = Point

fn create_user(id: UserId, email: Email) -> Person:
    return Person(name="Unknown", age=0, email=email)

fn calculate_distance(a: Coordinates, b: Coordinates) -> float:
    dx = a.x - b.x
    dy = a.y - b.y
    return math.sqrt(dx * dx + dy * dy)
```

## Concurrency

OtterLang provides built-in support for concurrent programming with tasks and channels.

### Spawning Tasks

```otter
use time

fn fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

fn compute_heavy_task(id: int) -> string:
    # Simulate heavy computation
    time.sleep(100)
    result = fibonacci(20 + id)
    return f"Task {id} result: {result}"

fn main():
    # Spawn concurrent tasks
    task1 = spawn compute_heavy_task(1)
    task2 = spawn compute_heavy_task(2)
    task3 = spawn compute_heavy_task(3)

    # Wait for all tasks to complete
    result1 = await task1
    result2 = await task2
    result3 = await task3

    println("All tasks completed:")
    println(result1)
    println(result2)
    println(result3)
```

## Error Handling

OtterLang uses `Result<T, E>` enum for error handling. Functions return `Result.Ok(value)` for success or `Result.Err(error)` for errors.

### Result Types

Functions return `Result<T, E>` to indicate success or failure:

```otter
use std/core

fn safe_divide(a: float, b: float) -> Result<float, string>:
    if b == 0.0:
        return Result.Err("Division by zero")
    return Result.Ok(a / b)

fn find_user(id: int) -> Result<Person, string>:
    # Simulate database lookup
    if id == 1:
        return Result.Ok(Person(name="Alice", age=30, email="alice@example.com"))
    return Result.Err("User not found")

fn main():
    # Handle division with pattern matching
    match safe_divide(10.0, 2.0):
        case Result.Ok(result):
            println(f"Division result: {result}")
        case Result.Err(error):
            println(f"Division by zero: {error}")

    # Handle user lookup
    match find_user(1):
        case Result.Ok(user):
            println(f"Found user: {user.name}")
        case Result.Err(error):
            println(f"Error: {error}")
```

### Option Types

For optional values (absence vs error), use `Option<T>`:

```otter
use std/core

fn find_user_optional(id: int) -> Option<Person>:
    if id == 1:
        return Option.Some(Person(name="Alice", age=30, email="alice@example.com"))
    return Option.None

fn main():
    match find_user_optional(1):
        case Option.Some(user):
            println(f"Found user: {user.name}")
        case Option.None:
            println("User not found")
```

## Advanced Topics

### Generic Functions

**Note:** OtterLang currently does not support generic function parameters (e.g., `fn first<T>(...)`). Generic behavior is achieved through generic structs, enums, and type aliases. Functions can work with generic types by accepting them as parameters:

```otter
# Generic behavior through type inference
fn first(items: list<int>) -> int:
    return items[0]

fn first_string(items: list<string>) -> string:
    return items[0]

# Or use generic structs to achieve polymorphism
struct Box<T>:
    value: T

fn get_value(box: Box<int>) -> int:
    return box.value

fn main():
    numbers = [1, 2, 3, 4, 5]
    first_num = first(numbers)  # 1

    strings = ["a", "b", "c"]
    first_str = first_string(strings)  # "a"
    
    boxed = Box(value=42)
    value = get_value(boxed)
    println(f"Boxed value: {value}")
```

### Advanced Structs and Enums

```otter
use std/core

enum Result<T, E>:
    Ok: (T)
    Err: (E)

struct Container<T>:
    items: list<T>

    fn add(self, item: T) -> unit:
        self.items += [item]

    fn get(self, index: int) -> Option<T>:
        if index >= 0 and index < len(self.items):
            return Option.Some(self.items[index])
        return Option.None

fn main():
    container = Container(items=[])
    container.add("hello")
    container.add("world")

    match container.get(0):
        case Option.Some(first):
            println(f"First item: {first}")
        case Option.None:
            println("No item at index 0")
```

### Modules and Imports

Organize your code into modules for better maintainability:

```otter
# math_utils.ot
pub fn add(a: float, b: float) -> float:
    return a + b

pub fn multiply(a: float, b: float) -> float:
    return a * b

pub fn factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

# main.ot
use math_utils

fn main():
    sum = math_utils.add(5.0, 3.0)
    product = math_utils.multiply(4.0, 2.0)
    fact = math_utils.factorial(5)

    println(f"Sum: {sum}, Product: {product}, Factorial: {fact}")
```

### Module Aliases

```otter
# Import with alias
use math_utils as mu

fn main():
    result = mu.add(1.0, 2.0)  # Using alias
    println(f"Result: {result}")
```

**Note:** OtterLang currently supports importing entire modules with optional aliases. Selective imports of specific items from a module are not yet supported. Use module aliases to shorten module names when needed.

For more information, see the [Language Specification](LANGUAGE_SPEC.md) and [API Reference](API_REFERENCE.md).

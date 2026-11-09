use bara::analyzer::Analyzer;
use bara::codegen::Codegen;
use bara::lexer::Lexer;
use bara::parser::Parser;

fn transpile_and_verify(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().expect("Lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("Parsing failed");
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("Analysis failed");
    let mut codegen = Codegen::new(&analyzer);
    let rust_code = codegen.generate(&program);
    let parsed_syn = syn::parse_file(&rust_code);
    assert!(
        parsed_syn.is_ok(),
        "Generated Rust code failed syn validation:\n{rust_code}\nError: {:?}",
        parsed_syn.err()
    );
    rust_code
}

#[test]
fn test_fibonacci_recursive() {
    let code = r#"
fib(n: int) -> int =>
    if n <= 1 =>
        return n
    return fib(n - 1) + fib(n - 2)

res = fib(10)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn fib(n: i64) -> i64"));
    assert!(rust.contains("fib((n - 1)) + fib((n - 2))"));
}

#[test]
fn test_fibonacci_iterative() {
    let code = r#"
fib_iter(n: int) -> int =>
    if n <= 1 =>
        return n
    a = 0
    b = 1
    i = 2
    while i <= n =>
        c = a + b
        a = b
        b = c
        i = i + 1
    return b

ans = fib_iter(15)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn fib_iter(n: i64) -> i64"));
    assert!(rust.contains("let mut a = 0;"));
    assert!(rust.contains("let mut b = 1;"));
    assert!(rust.contains("while (i <= n)"));
}

#[test]
fn test_factorial_recursive() {
    let code = r#"
factorial(n: int) -> int =>
    if n <= 1 =>
        return 1
    return n * factorial(n - 1)

fact5 = factorial(5)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn factorial(n: i64) -> i64"));
    assert!(rust.contains("factorial((n - 1))"));
}

#[test]
fn test_factorial_loop() {
    let code = r#"
factorial_loop(n: int) -> int =>
    result = 1
    i = 2
    while i <= n =>
        result = result * i
        i = i + 1
    return result

f = factorial_loop(6)
"#;
    let rust = transpile_and_verify(code);
    println!("GENERATED FACTORIAL_LOOP:\n{rust}");
    assert!(rust.contains("result"));
    assert!(rust.contains("result = (result * i);"));
}

#[test]
fn test_greatest_common_divisor() {
    let code = r#"
gcd(a: int, b: int) -> int =>
    x = a
    y = b
    while y != 0 =>
        t = y
        y = x % y
        x = t
    return x

g = gcd(48, 18)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn gcd(a: i64, b: i64) -> i64"));
    assert!(rust.contains("y = (x % y);"));
}

#[test]
fn test_least_common_multiple() {
    let code = r#"
gcd(a: int, b: int) -> int =>
    x = a
    y = b
    while y != 0 =>
        t = y
        y = x % y
        x = t
    return x

lcm(a: int, b: int) -> int =>
    if a == 0 or b == 0 =>
        return 0
    return (a * b) / gcd(a, b)

l = lcm(12, 18)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn lcm(a: i64, b: i64) -> i64"));
}

#[test]
fn test_is_prime_trial_division() {
    let code = r#"
is_prime(n: int) -> bool =>
    if n <= 1 =>
        return false
    if n <= 3 =>
        return true
    if n % 2 == 0 or n % 3 == 0 =>
        return false
    i = 5
    while i * i <= n =>
        if n % i == 0 or n % (i + 2) == 0 =>
            return false
        i = i + 6
    return true

p = is_prime(29)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn is_prime(n: i64) -> bool"));
    assert!(rust.contains("let mut i = 5;"));
}

#[test]
fn test_sieve_of_eratosthenes() {
    let code = r#"
count_primes_up_to(limit: int) -> int =>
    flags = [true, true, true, true, true, true, true, true, true, true]
    count = 0
    i = 2
    while i < 10 =>
        if flags[i] =>
            count = count + 1
        i = i + 1
    return count

num_primes = count_primes_up_to(10)
"#;
    let rust = transpile_and_verify(code);
    println!("GENERATED SIEVE:\n{rust}");
    assert!(rust.contains("pub fn count_primes_up_to(limit: i64) -> i64"));
    assert!(rust.contains("flags[(i as usize)]"));
}

#[test]
fn test_fast_exponentiation() {
    let code = r#"
power(base: int, exp: int) -> int =>
    res = 1
    b = base
    e = exp
    while e > 0 =>
        if e % 2 == 1 =>
            res = res * b
        b = b * b
        e = e / 2
    return res

val = power(2, 10)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn power(base: i64, exp: i64) -> i64"));
    assert!(rust.contains("let mut res = 1;"));
    assert!(rust.contains("e = (e / 2);"));
}

#[test]
fn test_matrix_2x2_multiplication() {
    let code = r#"
type Mat2 =
    a: int
    b: int
    c: int
    d: int

    multiply(other: Mat2) -> Mat2 =>
        ra = a * other.a + b * other.c
        rb = a * other.b + b * other.d
        rc = c * other.a + d * other.c
        rd = c * other.b + d * other.d
        return Mat2(a = ra, b = rb, c = rc, d = rd)

    determinant() -> int =>
        return a * d - b * c

m1 = Mat2(a = 1, b = 2, c = 3, d = 4)
m2 = Mat2(a = 2, b = 0, c = 1, d = 2)
m3 = m1.multiply(m2)
det = m3.determinant()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Mat2"));
    assert!(rust.contains("pub fn multiply(&self, other: &Mat2) -> Mat2"));
    assert!(rust.contains("pub fn determinant(&self) -> i64"));
}

#[test]
fn test_collatz_conjecture_steps() {
    let code = r#"
collatz(n: int) -> int =>
    steps = 0
    curr = n
    while curr > 1 =>
        if curr % 2 == 0 =>
            curr = curr / 2
        else =>
            curr = 3 * curr + 1
        steps = steps + 1
    return steps

s = collatz(27)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn collatz(n: i64) -> i64"));
    assert!(rust.contains("curr = ((3 * curr) + 1);"));
}

#[test]
fn test_binary_search_sqrt() {
    let code = r#"
integer_sqrt(x: int) -> int =>
    if x < 2 =>
        return x
    low = 1
    high = x
    ans = 1
    while low <= high =>
        mid = (low + high) / 2
        if mid * mid == x =>
            return mid
        if mid * mid < x =>
            ans = mid
            low = mid + 1
        else =>
            high = mid - 1
    return ans

sq = integer_sqrt(144)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn integer_sqrt(x: i64) -> i64"));
    assert!(rust.contains("let mut low = 1;"));
    assert!(rust.contains("let mut high = x;"));
}

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
fn test_boolean_de_morgan_laws() {
    let code = r#"
de_morgan_first(a: bool, b: bool) -> bool =>
    // !(a and b) == (!a or !b)
    left = !(a and b)
    right = !a or !b
    return left == right

de_morgan_second(a: bool, b: bool) -> bool =>
    // !(a or b) == (!a and !b)
    left = !(a or b)
    right = !a and !b
    return left == right

r1 = de_morgan_first(true, false)
r2 = de_morgan_second(false, true)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn de_morgan_first(a: bool, b: bool) -> bool"));
    assert!(rust.contains("pub fn de_morgan_second(a: bool, b: bool) -> bool"));
    assert!(rust.contains("!(a && b)"));
}

#[test]
fn test_truth_table_generator() {
    let code = r#"
xor_gate(a: bool, b: bool) -> bool =>
    return (a or b) and !(a and b)

implies_gate(a: bool, b: bool) -> bool =>
    return !a or b

check_xor_table() -> bool =>
    t1 = xor_gate(false, false) == false
    t2 = xor_gate(false, true) == true
    t3 = xor_gate(true, false) == true
    t4 = xor_gate(true, true) == false
    return t1 and t2 and t3 and t4

all_ok = check_xor_table()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn xor_gate(a: bool, b: bool) -> bool"));
    assert!(rust.contains("pub fn implies_gate(a: bool, b: bool) -> bool"));
    assert!(rust.contains("pub fn check_xor_table() -> bool"));
}

#[test]
fn test_bitwise_operations() {
    let code = r#"
bit_manipulation(a: int, b: int) -> int =>
    and_res = a & b
    or_res = a | b
    xor_res = a ^ b
    return and_res + or_res + xor_res

val = bit_manipulation(12, 5)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn bit_manipulation(a: i64, b: i64) -> i64"));
    assert!(rust.contains("(a & b)"));
    assert!(rust.contains("(a | b)"));
    assert!(rust.contains("(a ^ b)"));
}

#[test]
fn test_complex_conditional_chains() {
    let code = r#"
classify_grade(score: int) -> str =>
    if score >= 90 =>
        return "A"
    elif score >= 80 =>
        return "B"
    elif score >= 70 =>
        return "C"
    elif score >= 60 =>
        return "D"
    else =>
        return "F"

g1 = classify_grade(95)
g2 = classify_grade(82)
g3 = classify_grade(45)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn classify_grade(score: i64) -> String"));
    assert!(rust.contains("if (score >= 90)"));
    assert!(rust.contains("else if (score >= 80)"));
    assert!(rust.contains("else if (score >= 70)"));
    assert!(rust.contains("else if (score >= 60)"));
}

#[test]
fn test_ternary_expression_logic() {
    let code = r#"
max_of_two(a: int, b: int) -> int =>
    return a > b ? a : b

clamp(val: int, min_val: int, max_val: int) -> int =>
    return val < min_val ? min_val : (val > max_val ? max_val : val)

m = max_of_two(10, 20)
c = clamp(55, 0, 50)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn max_of_two(a: i64, b: i64) -> i64"));
    assert!(rust.contains("pub fn clamp(val: i64, min_val: i64, max_val: i64) -> i64"));
    assert!(rust.contains("if (a > b) { a } else { b }"));
}

#[test]
fn test_boolean_flag_masking() {
    let code = r#"
FLAG_READ = 1
FLAG_WRITE = 2
FLAG_EXECUTE = 4

has_permission(mask: int, flag: int) -> bool =>
    return (mask & flag) == flag

add_permission(mask: int, flag: int) -> int =>
    return mask | flag

remove_permission(mask: int, flag: int) -> int =>
    return mask & (!flag)

m0 = 0
m1 = add_permission(m0, FLAG_READ)
m2 = add_permission(m1, FLAG_WRITE)
can_w = has_permission(m2, FLAG_WRITE)
can_x = has_permission(m2, FLAG_EXECUTE)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn has_permission(mask: i64, flag: i64) -> bool"));
    assert!(rust.contains("pub fn add_permission(mask: i64, flag: i64) -> i64"));
}

#[test]
fn test_leap_year_logic() {
    let code = r#"
is_leap_year(year: int) -> bool =>
    if year % 400 == 0 =>
        return true
    if year % 100 == 0 =>
        return false
    return year % 4 == 0

ly1 = is_leap_year(2000)
ly2 = is_leap_year(1900)
ly3 = is_leap_year(2024)
ly4 = is_leap_year(2023)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn is_leap_year(year: i64) -> bool"));
    assert!(rust.contains("((year % 400) == 0)"));
}

#[test]
fn test_boolean_short_circuit_safe_guards() {
    let code = r#"
safe_divide_check(denom: int, num: int) -> bool =>
    // Short circuit prevents zero division
    if denom != 0 and (num / denom) > 10 =>
        return true
    return false

c1 = safe_divide_check(0, 100)
c2 = safe_divide_check(5, 100)
"#;
    let rust = transpile_and_verify(code);
    println!("GENERATED SHORT CIRCUIT:\n{rust}");
    assert!(rust.contains("pub fn safe_divide_check(denom: i64, num: i64) -> bool"));
    assert!(rust.contains("denom != 0"));
    assert!(rust.contains("num / denom"));
}

#[test]
fn test_nested_boolean_satisfiability() {
    let code = r#"
sat_3cnf(x1: bool, x2: bool, x3: bool) -> bool =>
    clause1 = x1 or !x2 or x3
    clause2 = !x1 or x2 or !x3
    clause3 = !x1 or !x2 or x3
    return clause1 and clause2 and clause3

s = sat_3cnf(true, false, true)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn sat_3cnf(x1: bool, x2: bool, x3: bool) -> bool"));
    assert!(rust.contains("clause1 && clause2"));
}

#[test]
fn test_multiple_return_branches_logic() {
    let code = r#"
evaluate_status(code: int, is_admin: bool) -> str =>
    if code == 200 =>
        return "OK"
    if code == 401 or code == 403 =>
        if is_admin =>
            return "Privileged Bypass"
        return "Access Denied"
    if code == 404 =>
        return "Not Found"
    return "Unknown Code"

res1 = evaluate_status(200, false)
res2 = evaluate_status(403, true)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn evaluate_status(code: i64, is_admin: bool) -> String"));
    assert!(rust.contains("return \"OK\".to_string();"));
    assert!(rust.contains("return \"Privileged Bypass\".to_string();"));
}

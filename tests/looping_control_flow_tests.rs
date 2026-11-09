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
fn test_while_loop_accumulator() {
    let code = r#"
sum_up_to(n: int) -> int =>
    total = 0
    i = 1
    while i <= n =>
        total = total + i
        i = i + 1
    return total

res = sum_up_to(100)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn sum_up_to(n: i64) -> i64"));
    assert!(rust.contains("while (i <= n)"));
    assert!(rust.contains("total = (total + i);"));
}

#[test]
fn test_for_in_list_loop() {
    let code = r#"
sum_list(numbers: [int]) -> int =>
    sum = 0
    for num in numbers =>
        sum = sum + num
    return sum

s = sum_list([10, 20, 30, 40])
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn sum_list(numbers: &Vec<i64>) -> i64"));
    assert!(rust.contains("for num in numbers"));
}

#[test]
fn test_loop_break_statement() {
    let code = r#"
find_first_negative(items: [int]) -> int =>
    idx = 0
    found_idx = -1
    while idx < items.len() =>
        if items[idx] < 0 =>
            found_idx = idx
            break
        idx = idx + 1
    return found_idx

pos = find_first_negative([10, 25, -3, 40])
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn find_first_negative(items: &Vec<i64>) -> i64"));
    assert!(rust.contains("break;"));
}

#[test]
fn test_loop_continue_statement() {
    let code = r#"
sum_only_odds(limit: int) -> int =>
    total = 0
    i = 0
    while i < limit =>
        i = i + 1
        if i % 2 == 0 =>
            continue
        total = total + i
    return total

odd_sum = sum_only_odds(10)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn sum_only_odds(limit: i64) -> i64"));
    assert!(rust.contains("continue;"));
}

#[test]
fn test_nested_while_loops_matrix() {
    let code = r#"
matrix_cell_count(rows: int, cols: int) -> int =>
    count = 0
    r = 0
    while r < rows =>
        c = 0
        while c < cols =>
            count = count + 1
            c = c + 1
        r = r + 1
    return count

cells = matrix_cell_count(5, 5)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn matrix_cell_count(rows: i64, cols: i64) -> i64"));
    assert!(rust.contains("while (r < rows)"));
    assert!(rust.contains("while (c < cols)"));
}

#[test]
fn test_nested_for_loops_pairs() {
    let code = r#"
count_pairs(a: [int], b: [int]) -> int =>
    total_pairs = 0
    for x in a =>
        for y in b =>
            total_pairs = total_pairs + 1
    return total_pairs

p = count_pairs([1, 2, 3], [4, 5])
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn count_pairs(a: &Vec<i64>, b: &Vec<i64>) -> i64"));
    assert!(rust.contains("for x in a"));
    assert!(rust.contains("for y in b"));
}

#[test]
fn test_early_return_in_loop() {
    let code = r#"
linear_contains(items: [int], target: int) -> bool =>
    i = 0
    while i < items.len() =>
        if items[i] == target =>
            return true
        i = i + 1
    return false

has_five = linear_contains([1, 3, 5, 7], 5)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn linear_contains(items: &Vec<i64>, target: i64) -> bool"));
    assert!(rust.contains("return true;"));
}

#[test]
fn test_countdown_loop() {
    let code = r#"
countdown(start: int) -> int =>
    curr = start
    steps = 0
    while curr > 0 =>
        curr = curr - 1
        steps = steps + 1
    return steps

cnt = countdown(10)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn countdown(start: i64) -> i64"));
    assert!(rust.contains("while (curr > 0)"));
}

#[test]
fn test_while_true_break_condition() {
    let code = r#"
find_threshold_power(factor: int, limit: int) -> int =>
    val = 1
    while true =>
        val = val * factor
        if val >= limit =>
            break
    return val

v = find_threshold_power(2, 1000)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn find_threshold_power(factor: i64, limit: i64) -> i64"));
    assert!(rust.contains("while true"));
    assert!(rust.contains("break;"));
}

#[test]
fn test_complex_loop_filter_mapping() {
    let code = r#"
filter_and_double_positives(nums: [int]) -> int =>
    sum = 0
    i = 0
    while i < nums.len() =>
        val = nums[i]
        i = i + 1
        if val <= 0 =>
            continue
        sum = sum + val * 2
    return sum

res = filter_and_double_positives([-5, 3, -1, 4, 10])
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn filter_and_double_positives(nums: &Vec<i64>) -> i64"));
    assert!(rust.contains("continue;"));
    assert!(rust.contains("sum = (sum + (val * 2));"));
}

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
fn test_vector3d_math() {
    let code = r#"
type Vec3 =
    x: float = 0.0
    y: float = 0.0
    z: float = 0.0

    dot(other: Vec3) -> float =>
        return x * other.x + y * other.y + z * other.z

    cross(other: Vec3) -> Vec3 =>
        cx = y * other.z - z * other.y
        cy = z * other.x - x * other.z
        cz = x * other.y - y * other.x
        return Vec3(x = cx, y = cy, z = cz)

    magnitude_sq() -> float =>
        return x * x + y * y + z * z

v1 = Vec3(x = 1.0, y = 2.0, z = 3.0)
v2 = Vec3(x = 4.0, y = 5.0, z = 6.0)
d = v1.dot(v2)
v3 = v1.cross(v2)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Vec3"));
    assert!(rust.contains("pub fn dot(&self, other: &Vec3) -> f64"));
    assert!(rust.contains("pub fn cross(&self, other: &Vec3) -> Vec3"));
}

#[test]
fn test_projectile_motion() {
    let code = r#"
type Projectile =
    v0: float
    angle_rad: float
    g: float = 9.81

    flight_time() -> float =>
        return 2.0 * v0 / g

    max_height() -> float =>
        return (v0 * v0) / (2.0 * g)

    range_dist() -> float =>
        return (v0 * v0) / g

p = Projectile(v0 = 20.0, angle_rad = 0.785)
t = p.flight_time()
h = p.max_height()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Projectile"));
    assert!(rust.contains("pub fn flight_time(&self) -> f64"));
    assert!(rust.contains("pub fn max_height(&self) -> f64"));
}

#[test]
fn test_statistics_mean_variance() {
    let code = r#"
type Stats =
    items: [float]

    mean() -> float =>
        if items.len() == 0 =>
            return 0.0
        sum = 0.0
        i = 0
        while i < items.len() =>
            sum = sum + items[i]
            i = i + 1
        return sum / (items.len() as float)

s = Stats(items = [10.0, 20.0, 30.0, 40.0])
m = s.mean()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Stats"));
    assert!(rust.contains("pub fn mean(&self) -> f64"));
}

#[test]
fn test_linear_regression() {
    let code = r#"
type LinearModel =
    slope: float = 0.0
    intercept: float = 0.0

    predict(x: float) -> float =>
        return slope * x + intercept

fit_model(x_mean: float, y_mean: float, cov: float, var_x: float) -> LinearModel =>
    m = cov / var_x
    b = y_mean - m * x_mean
    return LinearModel(slope = m, intercept = b)

model = fit_model(2.0, 5.0, 3.0, 1.5)
pred = model.predict(10.0)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct LinearModel"));
    assert!(rust.contains("pub fn predict(&self, x: f64) -> f64"));
    assert!(rust.contains("pub fn fit_model(x_mean: f64, y_mean: f64, cov: f64, var_x: f64) -> LinearModel"));
}

#[test]
fn test_trapezoidal_integration() {
    let code = r#"
f(x: float) -> float =>
    return x * x

integrate_trapezoid(a: float, b: float, n: int) -> float =>
    h = (b - a) / (n as float)
    sum = 0.5 * (f(a) + f(b))
    i = 1
    while i < n =>
        xi = a + (i as float) * h
        sum = sum + f(xi)
        i = i + 1
    return sum * h

area = integrate_trapezoid(0.0, 3.0, 100)
"#;
    let rust = transpile_and_verify(code);
    println!("GENERATED TRAPEZOID:\n{rust}");
    assert!(rust.contains("pub fn integrate_trapezoid(a: f64, b: f64, n: i64) -> f64"));
    assert!(rust.contains("let mut sum"));
    assert!(rust.contains("sum = (sum + f(&xi));"));
}

#[test]
fn test_newton_raphson_root() {
    let code = r#"
func(x: float) -> float =>
    return x * x - 2.0

func_prime(x: float) -> float =>
    return 2.0 * x

newton_root(initial_guess: float, iterations: int) -> float =>
    x = initial_guess
    i = 0
    while i < iterations =>
        y = func(x)
        dy = func_prime(x)
        x = x - y / dy
        i = i + 1
    return x

root = newton_root(1.5, 10)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn newton_root(initial_guess: f64, iterations: i64) -> f64"));
    assert!(rust.contains("let mut x = initial_guess;"));
    assert!(rust.contains("x = (x - (y / dy));"));
}

#[test]
fn test_temperature_converter() {
    let code = r#"
celsius_to_fahrenheit(c: float) -> float =>
    return c * 1.8 + 32.0

fahrenheit_to_celsius(f: float) -> float =>
    return (f - 32.0) / 1.8

celsius_to_kelvin(c: float) -> float =>
    return c + 273.15

f1 = celsius_to_fahrenheit(100.0)
c1 = fahrenheit_to_celsius(212.0)
k1 = celsius_to_kelvin(25.0)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn celsius_to_fahrenheit(c: f64) -> f64"));
    assert!(rust.contains("pub fn fahrenheit_to_celsius(f: f64) -> f64"));
    assert!(rust.contains("pub fn celsius_to_kelvin(c: f64) -> f64"));
}

#[test]
fn test_polynomial_horner() {
    let code = r#"
evaluate_poly(coeffs: [float], x: float) -> float =>
    result = 0.0
    i = 0
    while i < coeffs.len() =>
        result = result * x + coeffs[i]
        i = i + 1
    return result

p = evaluate_poly([2.0, -6.0, 2.0, -1.0], 3.0)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn evaluate_poly"));
    assert!(rust.contains("coeffs: &Vec<f64>"));
    assert!(rust.contains("let mut result = 0.0;"));
}

#[test]
fn test_complex_numbers() {
    let code = r#"
type Complex =
    re: float
    im: float

    add(other: Complex) -> Complex =>
        return Complex(re = re + other.re, im = im + other.im)

    mul(other: Complex) -> Complex =>
        r = re * other.re - im * other.im
        i = re * other.im + im * other.re
        return Complex(re = r, im = i)

    norm_sq() -> float =>
        return re * re + im * im

c1 = Complex(re = 3.0, im = 4.0)
c2 = Complex(re = 1.0, im = 2.0)
c3 = c1.mul(c2)
norm = c3.norm_sq()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Complex"));
    assert!(rust.contains("pub fn add(&self, other: &Complex) -> Complex"));
    assert!(rust.contains("pub fn mul(&self, other: &Complex) -> Complex"));
    assert!(rust.contains("pub fn norm_sq(&self) -> f64"));
}

#[test]
fn test_scientific_matrix_det() {
    let code = r#"
det3x3(m: [float]) -> float =>
    a = m[0] * (m[4] * m[8] - m[5] * m[7])
    b = m[1] * (m[3] * m[8] - m[5] * m[6])
    c = m[2] * (m[3] * m[7] - m[4] * m[6])
    return a - b + c

val = det3x3([1.0, 2.0, 3.0, 0.0, 1.0, 4.0, 5.0, 6.0, 0.0])
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn det3x3"));
    assert!(rust.contains("m: &Vec<f64>"));
}

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
fn test_struct_defaults_and_construction() {
    let code = r#"
type WindowConfig =
    title: str = "Untitled"
    width: int = 800
    height: int = 600
    resizable: bool = true

cfg1 = WindowConfig(title = "Bara App", width = 1280, height = 720, resizable = true)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct WindowConfig"));
    assert!(rust.contains("pub fn WindowConfig"));
    assert!(rust.contains("impl Default for WindowConfig"));
}

#[test]
fn test_immutable_method_field_access() {
    let code = r#"
type Rectangle =
    width: int
    height: int

    area() -> int =>
        return width * height

    perimeter() -> int =>
        return 2 * (width + height)

rect = Rectangle(width = 10, height = 20)
a = rect.area()
p = rect.perimeter()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Rectangle"));
    assert!(rust.contains("pub fn area(&self) -> i64"));
    assert!(rust.contains("pub fn perimeter(&self) -> i64"));
    assert!(rust.contains("(self.width * self.height)"));
}

#[test]
fn test_mutable_method_mutation() {
    let code = r#"
type Counter =
    count: int = 0
    step: int = 1

    increment() =>
        count = count + step

    reset() =>
        count = 0

c = Counter(count = 0, step = 5)
c.increment()
c.reset()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Counter"));
    assert!(rust.contains("pub fn increment(&mut self)"));
    assert!(rust.contains("pub fn reset(&mut self)"));
    assert!(rust.contains("self.count = (self.count + self.step);"));
    assert!(rust.contains("self.count = 0;"));
}

#[test]
fn test_nested_struct_composition() {
    let code = r#"
type Point =
    x: int
    y: int

type Circle =
    center: Point
    radius: int

    diameter() -> int =>
        return radius * 2

p = Point(x = 10, y = 15)
c = Circle(center = p, radius = 5)
d = c.diameter()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Point"));
    assert!(rust.contains("pub struct Circle"));
    assert!(rust.contains("pub fn diameter(&self) -> i64"));
}

#[test]
fn test_struct_with_nested_list_of_objects() {
    let code = r#"
type Task =
    id: int
    done: bool

type TaskList =
    tasks: [Task]

    count_completed() -> int =>
        completed = 0
        i = 0
        while i < tasks.len() =>
            if tasks[i].done =>
                completed = completed + 1
            i = i + 1
        return completed

t1 = Task(id = 1, done = true)
t2 = Task(id = 2, done = false)
tl = TaskList(tasks = [t1, t2])
c = tl.count_completed()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Task"));
    assert!(rust.contains("pub struct TaskList"));
    assert!(rust.contains("pub fn count_completed(&self) -> i64"));
}

#[test]
fn test_getter_and_setter_pattern() {
    let code = r#"
type TemperatureSensor =
    reading_c: float = 0.0

    get_celsius() -> float =>
        return reading_c

    set_celsius(new_val: float) =>
        reading_c = new_val

    get_fahrenheit() -> float =>
        return reading_c * 1.8 + 32.0

sensor = TemperatureSensor(reading_c = 20.0)
sensor.set_celsius(25.0)
f = sensor.get_fahrenheit()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct TemperatureSensor"));
    assert!(rust.contains("pub fn get_celsius(&self) -> f64"));
    assert!(rust.contains("pub fn set_celsius(&mut self, new_val: f64)"));
    assert!(rust.contains("pub fn get_fahrenheit(&self) -> f64"));
}

#[test]
fn test_factory_constructor_functions() {
    let code = r#"
type ColorRgb =
    r: int
    g: int
    b: int

create_white() -> ColorRgb =>
    return ColorRgb(r = 255, g = 255, b = 255)

create_black() -> ColorRgb =>
    return ColorRgb(r = 0, g = 0, b = 0)

w = create_white()
b = create_black()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct ColorRgb"));
    assert!(rust.contains("pub fn create_white() -> ColorRgb"));
    assert!(rust.contains("pub fn create_black() -> ColorRgb"));
}

#[test]
fn test_computed_tax_calculation() {
    let code = r#"
type OrderItem =
    name: str
    price: int
    quantity: int

    subtotal() -> int =>
        return price * quantity

    total_with_tax(tax_rate_percent: int) -> int =>
        st = subtotal()
        tax = (st * tax_rate_percent) / 100
        return st + tax

item = OrderItem(name = "Book", price = 20, quantity = 3)
st = item.subtotal()
tot = item.total_with_tax(10)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct OrderItem"));
    assert!(rust.contains("pub fn subtotal(&self) -> i64"));
    assert!(rust.contains("pub fn total_with_tax(&self, tax_rate_percent: i64) -> i64"));
    assert!(rust.contains("self.subtotal()"));
}

#[test]
fn test_method_calling_another_method() {
    let code = r#"
type MathHelper =
    square(x: int) -> int =>
        return x * x

    cube(x: int) -> int =>
        return square(x) * x

mh = MathHelper()
sq = mh.square(4)
cb = mh.cube(3)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct MathHelper"));
    assert!(rust.contains("pub fn square(&self, x: i64) -> i64"));
    assert!(rust.contains("pub fn cube(&self, x: i64) -> i64"));
    assert!(rust.contains("self.square(x)"));
}

#[test]
fn test_struct_equality_comparison() {
    let code = r#"
type Point2D =
    x: int
    y: int

is_origin(p: Point2D) -> bool =>
    return p.x == 0 and p.y == 0

are_same(p1: Point2D, p2: Point2D) -> bool =>
    return p1.x == p2.x and p1.y == p2.y

pt1 = Point2D(x = 0, y = 0)
pt2 = Point2D(x = 10, y = 20)
o = is_origin(pt1)
s = are_same(pt1, pt2)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Point2D"));
    assert!(rust.contains("pub fn is_origin(p: &Point2D) -> bool"));
    assert!(rust.contains("pub fn are_same(p1: &Point2D, p2: &Point2D) -> bool"));
}

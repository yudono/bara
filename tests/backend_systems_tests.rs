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
fn test_http_request_response_types() {
    let code = r#"
type HttpRequest =
    method: str
    path: str
    body: str

type HttpResponse =
    status: int
    body: str

handle_ping(req: HttpRequest) -> HttpResponse =>
    if req.path == "/ping" =>
        return HttpResponse(status = 200, body = "pong")
    return HttpResponse(status = 404, body = "not found")

req = HttpRequest(method = "GET", path = "/ping", body = "")
res = handle_ping(req)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct HttpRequest"));
    assert!(rust.contains("pub struct HttpResponse"));
    assert!(rust.contains("pub fn handle_ping(req: &HttpRequest) -> HttpResponse"));
}

#[test]
fn test_router_url_dispatcher() {
    let code = r#"
type RouteResult =
    handler_id: int
    matched: bool

route_dispatch(path: str) -> RouteResult =>
    if path == "/api/users" =>
        return RouteResult(handler_id = 1, matched = true)
    elif path == "/api/posts" =>
        return RouteResult(handler_id = 2, matched = true)
    elif path == "/api/health" =>
        return RouteResult(handler_id = 3, matched = true)
    return RouteResult(handler_id = 0, matched = false)

r1 = route_dispatch("/api/users")
r2 = route_dispatch("/api/unknown")
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct RouteResult"));
    assert!(rust.contains("pub fn route_dispatch(path: &String) -> RouteResult"));
}

#[test]
fn test_bearer_token_authorization() {
    let code = r#"
type AuthContext =
    is_authenticated: bool
    user_id: int

verify_bearer_token(header: str, secret: str) -> AuthContext =>
    if header.len() == 0 =>
        return AuthContext(is_authenticated = false, user_id = 0)
    if header == secret =>
        return AuthContext(is_authenticated = true, user_id = 42)
    return AuthContext(is_authenticated = false, user_id = 0)

auth = verify_bearer_token("secret_token_123", "secret_token_123")
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct AuthContext"));
    assert!(rust.contains("pub fn verify_bearer_token(header: &String, secret: &String) -> AuthContext"));
}

#[test]
fn test_status_code_messages() {
    let code = r#"
get_status_text(code: int) -> str =>
    if code == 200 =>
        return "OK"
    elif code == 201 =>
        return "Created"
    elif code == 400 =>
        return "Bad Request"
    elif code == 401 =>
        return "Unauthorized"
    elif code == 404 =>
        return "Not Found"
    elif code == 500 =>
        return "Internal Server Error"
    return "Unknown"

s200 = get_status_text(200)
s404 = get_status_text(404)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn get_status_text(code: i64) -> String"));
    assert!(rust.contains("return \"Created\".to_string();"));
}

#[test]
fn test_token_bucket_rate_limiter() {
    let code = r#"
type RateLimiter =
    capacity: int = 10
    tokens: int = 10

    allow_request(cost: int) -> bool =>
        if tokens >= cost =>
            tokens = tokens - cost
            return true
        return false

    refill(amount: int) =>
        tokens = tokens + amount
        if tokens > capacity =>
            tokens = capacity

limiter = RateLimiter(capacity = 100, tokens = 100)
ok1 = limiter.allow_request(5)
limiter.refill(2)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct RateLimiter"));
    assert!(rust.contains("pub fn allow_request(&mut self, cost: i64) -> bool"));
    assert!(rust.contains("pub fn refill(&mut self, amount: i64)"));
}

#[test]
fn test_query_string_parser_simulation() {
    let code = r#"
find_query_param(query: str, key: str) -> str =>
    if query.len() == 0 or key.len() == 0 =>
        return ""
    if query == key =>
        return "found"
    return "not_found"

res = find_query_param("id=123", "id=123")
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn find_query_param(query: &String, key: &String) -> String"));
}

#[test]
fn test_middleware_pipeline() {
    let code = r#"
type RequestContext =
    path: str
    duration_ms: int
    passed_auth: bool

log_middleware(ctx: RequestContext) -> RequestContext =>
    return RequestContext(path = ctx.path, duration_ms = ctx.duration_ms + 5, passed_auth = ctx.passed_auth)

auth_middleware(ctx: RequestContext) -> RequestContext =>
    return RequestContext(path = ctx.path, duration_ms = ctx.duration_ms + 2, passed_auth = true)

pipeline(ctx: RequestContext) -> RequestContext =>
    return auth_middleware(log_middleware(ctx))

initial = RequestContext(path = "/api/data", duration_ms = 0, passed_auth = false)
final_ctx = pipeline(initial)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct RequestContext"));
    assert!(rust.contains("pub fn log_middleware(ctx: &RequestContext) -> RequestContext"));
    assert!(rust.contains("pub fn auth_middleware(ctx: &RequestContext) -> RequestContext"));
}

#[test]
fn test_in_memory_kv_cache() {
    let code = r#"
type CacheStore =
    data: {str: str}

    get_val(key: str) -> str =>
        if data.contains_key(key) =>
            return "hit"
        return "miss"

cache = CacheStore(data = {"user:1": "Alice", "user:2": "Bob"})
v = cache.get_val("user:1")
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct CacheStore"));
    assert!(rust.contains("pub fn get_val(&self, key: &String) -> String"));
}

#[test]
fn test_event_dispatcher() {
    let code = r#"
type Event =
    name: str
    payload_size: int

type EventLog =
    count: int = 0
    total_bytes: int = 0

    record(e: Event) =>
        count = count + 1
        total_bytes = total_bytes + e.payload_size

log = EventLog(count = 0, total_bytes = 0)
e1 = Event(name = "user_login", payload_size = 128)
e2 = Event(name = "user_click", payload_size = 64)
log.record(e1)
log.record(e2)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct Event"));
    assert!(rust.contains("pub struct EventLog"));
    assert!(rust.contains("pub fn record(&mut self, e: &Event)"));
}

#[test]
fn test_payload_validation() {
    let code = r#"
type CreateUserPayload =
    username: str
    email: str
    age: int

validate_payload(p: CreateUserPayload) -> bool =>
    if p.username.len() < 3 =>
        return false
    if p.email.len() < 5 =>
        return false
    if p.age <= 0 or p.age > 150 =>
        return false
    return true

p1 = CreateUserPayload(username = "bob", email = "bob@example.com", age = 25)
p2 = CreateUserPayload(username = "a", email = "b@c", age = -1)
v1 = validate_payload(p1)
v2 = validate_payload(p2)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct CreateUserPayload"));
    assert!(rust.contains("pub fn validate_payload(p: &CreateUserPayload) -> bool"));
}

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
fn test_stack_lifo_operations() {
    let code = r#"
type IntStack =
    items: [int]

    push_val(v: int) =>
        items.push(v)

    pop_val() -> int =>
        if items.len() == 0 =>
            return -1
        return items.pop().unwrap_or(-1)

    is_empty() -> bool =>
        return items.len() == 0

s = IntStack(items = [])
s.push_val(10)
s.push_val(20)
v1 = s.pop_val()
empty = s.is_empty()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct IntStack"));
    assert!(rust.contains("pub fn push_val(&mut self, v: i64)"));
    assert!(rust.contains("pub fn pop_val(&mut self) -> i64"));
    assert!(rust.contains("pub fn is_empty(&self) -> bool"));
}

#[test]
fn test_queue_fifo_operations() {
    let code = r#"
type IntQueue =
    elements: [int]

    enqueue(x: int) =>
        elements.push(x)

    dequeue() -> int =>
        if elements.len() == 0 =>
            return -1
        return elements.remove(0)

    size() -> int =>
        return elements.len() as int

q = IntQueue(elements = [])
q.enqueue(1)
q.enqueue(2)
first = q.dequeue()
sz = q.size()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct IntQueue"));
    assert!(rust.contains("pub fn enqueue(&mut self, x: i64)"));
    assert!(rust.contains("pub fn dequeue(&mut self) -> i64"));
    assert!(rust.contains("pub fn size(&self) -> i64"));
}

#[test]
fn test_circular_ring_buffer() {
    let code = r#"
type RingBuffer =
    capacity: int = 4
    head: int = 0
    tail: int = 0
    size: int = 0

    is_full() -> bool =>
        return size == capacity

    write(val: int) -> bool =>
        if is_full() =>
            return false
        tail = (tail + 1) % capacity
        size = size + 1
        return true

    read() -> int =>
        if size == 0 =>
            return -1
        head = (head + 1) % capacity
        size = size - 1
        return 1

rb = RingBuffer(capacity = 8, head = 0, tail = 0, size = 0)
w = rb.write(42)
r = rb.read()
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct RingBuffer"));
    assert!(rust.contains("pub fn write(&mut self, val: i64) -> bool"));
    assert!(rust.contains("pub fn read(&mut self) -> i64"));
}

#[test]
fn test_singly_linked_list_node() {
    let code = r#"
type ListNode =
    val: int
    has_next: bool
    next_val: int

sum_two_nodes(n1: ListNode, n2: ListNode) -> int =>
    return n1.val + n2.val

node1 = ListNode(val = 10, has_next = true, next_val = 20)
node2 = ListNode(val = 20, has_next = false, next_val = 0)
total = sum_two_nodes(node1, node2)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct ListNode"));
    assert!(rust.contains("pub fn sum_two_nodes(n1: &ListNode, n2: &ListNode) -> i64"));
}

#[test]
fn test_bst_node_search() {
    let code = r#"
type BstNode =
    key: int
    has_left: bool
    left_key: int
    has_right: bool
    right_key: int

search_bst(node: BstNode, target: int) -> bool =>
    if node.key == target =>
        return true
    if target < node.key and node.has_left =>
        return node.left_key == target
    if target > node.key and node.has_right =>
        return node.right_key == target
    return false

root = BstNode(key = 50, has_left = true, left_key = 25, has_right = true, right_key = 75)
found = search_bst(root, 25)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct BstNode"));
    assert!(rust.contains("pub fn search_bst(node: &BstNode, target: i64) -> bool"));
}

#[test]
fn test_hash_bucket_index() {
    let code = r#"
hash_string(s: str, num_buckets: int) -> int =>
    hash = 0
    i = 0
    while i < s.len() =>
        hash = (hash * 31 + 7) % num_buckets
        i = i + 1
    return hash

b1 = hash_string("user_profile", 16)
b2 = hash_string("auth_token", 16)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn hash_string(s: &String, num_buckets: i64) -> i64"));
    assert!(rust.contains("hash = (((hash * 31) + 7) % num_buckets);"));
}

#[test]
fn test_binary_min_heap_indices() {
    let code = r#"
heap_parent(i: int) -> int =>
    return (i - 1) / 2

heap_left_child(i: int) -> int =>
    return 2 * i + 1

heap_right_child(i: int) -> int =>
    return 2 * i + 2

p = heap_parent(4)
l = heap_left_child(1)
r = heap_right_child(1)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn heap_parent(i: i64) -> i64"));
    assert!(rust.contains("pub fn heap_left_child(i: i64) -> i64"));
    assert!(rust.contains("pub fn heap_right_child(i: i64) -> i64"));
}

#[test]
fn test_memory_pool_allocator() {
    let code = r#"
type MemoryBlock =
    id: int
    is_free: bool
    size_bytes: int

type MemoryPool =
    blocks: [MemoryBlock]

    allocate(req_size: int) -> int =>
        i = 0
        while i < blocks.len() =>
            if blocks[i].is_free and blocks[i].size_bytes >= req_size =>
                blocks[i].is_free = false
                return blocks[i].id
            i = i + 1
        return -1

pool = MemoryPool(blocks = [])
block_id = pool.allocate(256)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct MemoryBlock"));
    assert!(rust.contains("pub struct MemoryPool"));
    assert!(rust.contains("pub fn allocate(&mut self, req_size: i64) -> i64"));
}

#[test]
fn test_array_deduplication_simulation() {
    let code = r#"
count_unique_sorted(items: [int]) -> int =>
    if items.len() <= 1 =>
        return items.len() as int
    unique_count = 1
    i = 1
    while i < items.len() =>
        if items[i] != items[i - 1] =>
            unique_count = unique_count + 1
        i = i + 1
    return unique_count

u = count_unique_sorted([1, 1, 2, 3, 3, 3, 4, 5, 5])
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub fn count_unique_sorted(items: &Vec<i64>) -> i64"));
    assert!(rust.contains("let mut unique_count = 1;"));
}

#[test]
fn test_dynamic_array_growth() {
    let code = r#"
type DynArray =
    len: int = 0
    cap: int = 2

    append(x: int) =>
        len = len + 1
        if len > cap =>
            cap = cap * 2

arr = DynArray(len = 0, cap = 2)
arr.append(10)
arr.append(20)
arr.append(30)
"#;
    let rust = transpile_and_verify(code);
    assert!(rust.contains("pub struct DynArray"));
    assert!(rust.contains("pub fn append(&mut self, x: i64)"));
    assert!(rust.contains("self.cap = (self.cap * 2);"));
}

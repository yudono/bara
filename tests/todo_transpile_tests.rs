use bara::lexer::Lexer;
use bara::parser::Parser;
use bara::analyzer::Analyzer;
use bara::codegen::Codegen;

#[test]
fn test_todo_app_transpilation() {
    let todo_br = r#"
type TodoItem =
    id: int
    teks: str
    selesai = false

type TodoApp =
    input_tugas = ""
    filter_status = "Semua"
    daftar_tugas = [
        TodoItem(id = 1, teks = "Bikin compiler Bara", selesai = true),
        TodoItem(id = 2, teks = "Integrasi parser JSX .brx", selesai = false),
        TodoItem(id = 3, teks = "Compile ke egui native", selesai = false)
    ]

    tambah_tugas() =>
        if input_tugas != "" =>
            id_baru = daftar_tugas.len() + 1
            daftar_tugas.push(TodoItem(id = id_baru, teks = input_tugas, selesai = false))
            input_tugas = ""

    hapus_tugas(id: int) =>
        daftar_tugas.retain(t => t.id != id)

    hitung_aktif() =>
        daftar_tugas.filter(t => !t.selesai).len()
"#;

    let todo_brx = r#"
use eframe
use egui
use TodoApp

view TodoApp =>
    <Panel class="layar_utama">
        <Col class="container_utama">
            
            {/* Header Aplikasi */}
            <Row class="header_box">
                <Text class="judul">⚡ Bara Todo App</Text>
                <Text class="badge">Sisa: {hitung_aktif()} tugas</Text>
            </Row>

            {/* Form Input Tugas Baru */}
            <Row class="form_box">
                <Input 
                    bind={input_tugas} 
                    placeholder="Apa rencana kerjamu hari ini?..." 
                />
                <Button class="btn_tambah" onClick={tambah_tugas()}>
                    Tambah
                </Button>
            </Row>

            {/* Filter Tab */}
            <Row class="filter_box">
                <Button class={filter_status == "Semua" ? "btn_filter aktif" : "btn_filter"} onClick={filter_status = "Semua"}>
                    Semua
                </Button>
                <Button class={filter_status == "Aktif" ? "btn_filter aktif" : "btn_filter"} onClick={filter_status = "Aktif"}>
                    Aktif
                </Button>
                <Button class={filter_status == "Selesai" ? "btn_filter aktif" : "btn_filter"} onClick={filter_status = "Selesai"}>
                    Selesai
                </Button>
            </Row>

            <Separator />

            {/* List Tugas (Scrollable Area) */}
            <Scroll class="scroll_area">
                <Col class="list_col">
                    {for item in daftar_tugas =>
                        {if filter_status == "Semua" or (filter_status == "Aktif" and !item.selesai) or (filter_status == "Selesai" and item.selesai) =>
                            <Row class={item.selesai ? "item_tugas selesai" : "item_tugas"}>
                                <Checkbox bind={item.selesai} />
                                <Text class="teks_tugas">{item.teks}</Text>
                                <Button class="btn_hapus" onClick={hapus_tugas(item.id)}>
                                    ✕
                                </Button>
                            </Row>
                        }
                    }
                </Col>
            </Scroll>

        </Col>
    </Panel>

<style>
    .layar_utama {
        background: #090d16;
        padding: 24px;
    }
    .container_utama {
        gap: 16px;
    }
    .header_box {
        justify-content: space-between;
        align-items: center;
    }
    .judul {
        font-size: 22px;
        color: #f8fafc;
        font-weight: bold;
    }
    .badge {
        font-size: 13px;
        color: #38bdf8;
        background: #0369a133;
        padding: 4px 8px;
        border-radius: 6px;
    }
    .form_box {
        gap: 10px;
    }
    .btn_tambah {
        background: #6366f1;
        color: #ffffff;
        border-radius: 8px;
        padding: 8px 16px;
    }
    .filter_box {
        gap: 8px;
    }
    .btn_filter {
        background: #1e293b;
        color: #94a3b8;
        border-radius: 6px;
        padding: 4px 12px;
    }
    .btn_filter.aktif {
        background: #334155;
        color: #f8fafc;
        font-weight: bold;
    }
    .scroll_area {
        height: 350px;
    }
    .list_col {
        gap: 8px;
    }
    .item_tugas {
        background: #111827;
        border: 1px solid #1f2937;
        padding: 12px;
        border-radius: 10px;
        align-items: center;
        justify-content: space-between;
    }
    .item_tugas.selesai {
        opacity: 0.5;
    }
    .teks_tugas {
        color: #e2e8f0;
        font-size: 14px;
    }
    .btn_hapus {
        background: transparent;
        color: #ef4444;
        font-weight: bold;
    }
</style>
"#;

    let main_br = r#"
use brui
use TodoApp

brui.provider("egui")
app = brui.load("todo.brx")
app.run("Bara Todo Desktop", width = 450, height = 600)
"#;

    // Combine items from all three files into a single compilation unit
    let combined_source = format!("{todo_br}\n{todo_brx}\n{main_br}");
    let tokens = Lexer::new(&combined_source).tokenize().expect("Lexing failed");
    let mut program = Parser::new(tokens).parse_program().expect("Parsing failed");
    
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(&mut program).expect("Semantic analysis failed");

    let mut codegen = Codegen::new(&analyzer);
    let rust_code = codegen.generate(&program);

    println!("=== GENERATED RUST CODE ===\n{rust_code}\n===========================");

    // Verify it parses as 100% valid Rust syn syntax tree
    let parsed_syn = syn::parse_file(&rust_code);
    assert!(parsed_syn.is_ok(), "Generated Rust code is not syntactically valid: {:?}", parsed_syn.err());

    // Verify expected constructs
    assert!(rust_code.contains("pub struct TodoItem"));
    assert!(rust_code.contains("pub struct TodoApp"));
    assert!(rust_code.contains("impl Default for TodoItem"));
    assert!(rust_code.contains("impl Default for TodoApp"));
    assert!(rust_code.contains("pub fn TodoItem("));
    assert!(rust_code.contains("pub fn tambah_tugas(&mut self)"));
    assert!(rust_code.contains("pub fn hapus_tugas(&mut self, id: i64)"));
    assert!(rust_code.contains("pub fn hitung_aktif(&self) -> i64"));
    assert!(rust_code.contains("impl eframe::App for TodoApp"));
    assert!(rust_code.contains(".frame("));
    assert!(rust_code.contains("egui::Color32::from_rgb(9, 13, 22)"));
    assert!(rust_code.contains(".max_height(350.0)"));
    assert!(rust_code.contains("egui::TextEdit::singleline(&mut self.input_tugas)"));
    assert!(rust_code.contains("eframe::run_native"));
    assert!(rust_code.contains("\"Bara Todo Desktop\""));
    assert!(rust_code.contains("fn main() -> Result<(), eframe::Error>"));
}

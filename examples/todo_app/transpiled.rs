#![allow(unused_imports, unused_mut, unused_variables, dead_code, unused_parens)]
use eframe;
use egui;
#[derive(Debug, Clone, PartialEq)]
pub struct TodoItem {
    pub id: i64,
    pub teks: String,
    pub selesai: bool,
}
impl Default for TodoItem {
    fn default() -> Self {
        Self {
            id: 0,
            teks: String::new(),
            selesai: false,
        }
    }
}
#[allow(non_snake_case)]
pub fn TodoItem(id: i64, teks: impl Into<String>, selesai: bool) -> TodoItem {
    TodoItem {
        id: id,
        teks: teks.into(),
        selesai: selesai,
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct TodoApp {
    pub input_tugas: String,
    pub filter_status: String,
    pub daftar_tugas: Vec<TodoItem>,
}
impl Default for TodoApp {
    fn default() -> Self {
        Self {
            input_tugas: "".to_string(),
            filter_status: "Semua".to_string(),
            daftar_tugas: vec![
                TodoItem(1, "Bikin compiler Bara".to_string(), true), TodoItem(2,
                "Integrasi parser JSX .brx".to_string(), false), TodoItem(3,
                "Compile ke egui native".to_string(), false)
            ],
        }
    }
}
#[allow(non_snake_case)]
pub fn TodoApp(
    input_tugas: impl Into<String>,
    filter_status: impl Into<String>,
    daftar_tugas: Vec<TodoItem>,
) -> TodoApp {
    TodoApp {
        input_tugas: input_tugas.into(),
        filter_status: filter_status.into(),
        daftar_tugas: daftar_tugas,
    }
}
impl TodoApp {
    pub fn tambah_tugas(&mut self) {
        if (self.input_tugas != "".to_string()) {
            let id_baru = ((self.daftar_tugas.len() as i64) + 1);
            self.daftar_tugas.push(TodoItem(id_baru, self.input_tugas.clone(), false));
            self.input_tugas = "".to_string();
        }
    }
    pub fn hapus_tugas(&mut self, id: i64) {
        self.daftar_tugas.retain(|t| (t.id != id))
    }
    pub fn hitung_aktif(&self) -> i64 {
        (self.daftar_tugas.iter().filter(|t| !t.selesai).count() as i64)
    }
}
impl eframe::App for TodoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(9, 13, 22))
                    .inner_margin(egui::Margin::same(24.0)),
            )
            .show(
                ctx,
                |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 16.0;
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("\u{26a1} Bara Todo App")
                                    .color(egui::Color32::from_rgb(248, 250, 252))
                                    .size(22.0)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                        format!("Sisa : {} tugas", self.hitung_aktif()),
                                    )
                                    .color(egui::Color32::from_rgb(56, 189, 248))
                                    .size(13.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.add(
                                egui::TextEdit::singleline(&mut self.input_tugas)
                                    .hint_text("Apa rencana kerjamu hari ini?..."),
                            );
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Tambah")
                                            .color(egui::Color32::from_rgb(255, 255, 255)),
                                    ),
                                )
                                .clicked()
                            {
                                self.tambah_tugas();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Semua")
                                            .color(egui::Color32::from_rgb(248, 250, 252))
                                            .strong(),
                                    ),
                                )
                                .clicked()
                            {
                                self.filter_status = "Semua".to_string();
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Aktif")
                                            .color(egui::Color32::from_rgb(248, 250, 252))
                                            .strong(),
                                    ),
                                )
                                .clicked()
                            {
                                self.filter_status = "Aktif".to_string();
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Selesai")
                                            .color(egui::Color32::from_rgb(248, 250, 252))
                                            .strong(),
                                    ),
                                )
                                .clicked()
                            {
                                self.filter_status = "Selesai".to_string();
                            }
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .max_height(350.0)
                            .show(
                                ui,
                                |ui| {
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 8.0;
                                        let mut __actions: Vec<Box<dyn FnOnce(&mut Self)>> = Vec::new();
                                        for item in &mut self.daftar_tugas {
                                            let __item_clone = item.clone();
                                            if (((self.filter_status == "Semua".to_string())
                                                || ((self.filter_status == "Aktif".to_string())
                                                    && !item.selesai))
                                                || ((self.filter_status == "Selesai".to_string())
                                                    && item.selesai))
                                            {
                                                egui::Frame::none()
                                                    .fill(egui::Color32::from_rgb(17, 24, 39))
                                                    .inner_margin(egui::Margin::same(12.0))
                                                    .rounding(egui::Rounding::same(10.0))
                                                    .stroke(
                                                        egui::Stroke::new(1.0, egui::Color32::from_rgb(31, 41, 55)),
                                                    )
                                                    .show(
                                                        ui,
                                                        |ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.checkbox(&mut item.selesai, "");
                                                                ui.label(
                                                                    egui::RichText::new(format!("{}", item.teks))
                                                                        .color(egui::Color32::from_rgb(226, 232, 240))
                                                                        .size(14.0),
                                                                );
                                                                if ui
                                                                    .add(
                                                                        egui::Button::new(
                                                                            egui::RichText::new("\u{2715}")
                                                                                .color(egui::Color32::from_rgb(239, 68, 68))
                                                                                .strong(),
                                                                        ),
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    let __item = __item_clone.clone();
                                                                    __actions.push(Box::new(move |s: &mut Self| {
                                                                        s.hapus_tugas(__item.id);
                                                                    }));
                                                                }
                                                            });
                                                        },
                                                    );
                                            }
                                        }
                                        for act in __actions {
                                            act(self);
                                        }
                                    });
                                },
                            );
                    });
                },
            );
    }
}
fn main() -> Result<(), eframe::Error> {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = egui::ViewportBuilder::default()
        .with_inner_size([450.0, 600.0]);
    eframe::run_native(
        "Bara Todo Desktop",
        native_options,
        Box::new(|_cc| Box::new(TodoApp::default())),
    )
}

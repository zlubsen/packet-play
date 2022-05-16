use eframe::NativeOptions;
use crate::{Cli};
use crate::model::pcap::Pcap;

pub(crate) fn run_gui(cli : Cli, recording: Pcap) {
    let options = NativeOptions::default();
    eframe::run_native(
        "packet-play",
        options,
        Box::new(|_cc| Box::new(GuiApp{ cli })),
    )
}

struct GuiApp {
    cli: Cli
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(&mut String::from("Aap"));
            });
            ui.add(egui::Slider::new(&mut 88, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                // self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", "aap", "88"));
        });
    }
}
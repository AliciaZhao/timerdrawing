use eframe::{egui, App};
use image::DynamicImage;
use rfd::FileDialog;
use std::fs;
use std::path::{Path, PathBuf};

struct ImageViewerApp {
    image_paths: Vec<PathBuf>,
    current_index: usize,
    current_image: Option<DynamicImage>,
    texture: Option<egui::TextureHandle>,
    last_size: Option<egui::Vec2>,
}

impl ImageViewerApp {
    fn load_image(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.image_paths.get(self.current_index) {
            if let Ok(img) = image::open(path) {
                let rgba = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.texture = Some(ctx.load_texture("image", color_image, Default::default()));
                self.current_image = Some(img);
                self.last_size = None;
            }
        }
    }

    fn next_image(&mut self, ctx: &egui::Context) {
        if !self.image_paths.is_empty() {
            self.current_index = (self.current_index + 1) % self.image_paths.len();
            self.load_image(ctx);
        }
    }
}

impl App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Next Image").clicked() {
                self.next_image(ctx);
            }

            if let Some(img) = &self.current_image {
                let img_width = img.width() as f32;
                let img_height = img.height() as f32;
                let aspect_ratio = img_width / img_height;

                let available = ui.available_size();
                let mut target_width = available.x;
                let mut target_height = target_width / aspect_ratio;

                if target_height > available.y {
                    target_height = available.y;
                    target_width = target_height * aspect_ratio;
                }

                let target_size = egui::Vec2::new(target_width.max(300.0), target_height.max(200.0));

                if self.last_size.map_or(true, |s| (s - target_size).length_sq() > 1.0) {
                    frame.set_window_size(target_size + egui::vec2(16.0, 56.0));
                    self.last_size = Some(target_size);
                }

                if let Some(texture) = &self.texture {
                    ui.allocate_ui_with_layout(
                        ui.available_size(),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.image(texture.id(), target_size);
                        },
                    );
                }
            }
        });
    }
}

fn get_image_paths(folder: &Path) -> Vec<PathBuf> {
    fs::read_dir(folder)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "bmp")
            } else {
                false
            }
        })
        .collect()
}

fn main() {
    let folder = FileDialog::new()
        .set_title("Select an image folder")
        .pick_folder()
        .expect("No folder selected");

    let image_paths = get_image_paths(&folder);

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        resizable: true,
        ..Default::default()
    };

    eframe::run_native(
        "Rust Image Viewer",
        native_options,
        Box::new(move |_cc| {
            Box::new(ImageViewerApp {
                image_paths,
                current_index: 0,
                current_image: None,
                texture: None,
                last_size: None,
            })
        }),
    );
}

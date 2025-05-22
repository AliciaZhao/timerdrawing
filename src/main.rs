use eframe::{egui, egui::ViewportBuilder};
use image::{DynamicImage, GenericImageView};
use rfd::FileDialog;
use std::fs;
use std::path::{Path, PathBuf};

struct ImageViewerApp {
    image_paths: Vec<PathBuf>,
    current_index: usize,
    current_image: Option<DynamicImage>,
    texture: Option<egui::TextureHandle>, 
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

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

                let target_size = egui::Vec2::new(target_width, target_height);

                if let Some(texture) = &self.texture {
                    ui.allocate_ui_with_layout(
                        ui.available_size(),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.image((texture.id(), target_size));
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
        viewport: ViewportBuilder::default()
            .with_inner_size(egui::Vec2::new(800.0, 600.0))
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Image Viewer",
        native_options,
        Box::new(move |ctx| {
            let mut app = ImageViewerApp {
                image_paths,
                current_index: 0,
                current_image: None,
                texture: None,
            };
            app.load_image(&ctx.egui_ctx);
            Box::new(app)
        }),
    );
}


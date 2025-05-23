use eframe::{egui, App};
use image::DynamicImage;
use rfd::FileDialog;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use windows::{
    Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
        WindowFromPoint, GetCursorPos,
    },
    Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    Win32::System::ProcessStatus::K32GetModuleBaseNameW,
    Win32::Foundation::{HWND, POINT, CloseHandle},
};

struct ImageViewerApp {
    image_paths: Vec<PathBuf>,
    current_index: usize,
    current_image: Option<DynamicImage>,
    texture: Option<egui::TextureHandle>,
    last_size: Option<egui::Vec2>,
    last_hover: Instant,
    decorations_visible: bool,
    folder_map: HashMap<PathBuf, bool>,
    show_folder_manager: bool,
    show_context_menu: bool,
    context_menu_pos: egui::Pos2,
    target_exe_name: Option<String>,
    target_is_active: bool,
    target_is_hovered: bool,
}

impl ImageViewerApp {
    fn get_exe_name_from_hwnd(hwnd: HWND) -> Option<String> {
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let handle = match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
                Ok(h) => h,
                Err(_) => return None,
            };

            let mut buffer = [0u16; 260];
            let len = K32GetModuleBaseNameW(handle, None, &mut buffer);
            CloseHandle(handle);

            if len == 0 {
                return None;
            }

            Some(String::from_utf16_lossy(&buffer[..len as usize]).to_lowercase())
        }
    }

    fn load_image(&mut self, ctx: &egui::Context) {
        while let Some(path) = self.image_paths.get(self.current_index) {
            match image::open(path) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let size = [img.width() as usize, img.height() as usize];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                    self.texture = Some(ctx.load_texture("image", color_image, Default::default()));
                    self.current_image = Some(img);
                    self.last_size = None;
                    break;
                }
                Err(_) => {
                    self.image_paths.remove(self.current_index);
                    if self.current_index >= self.image_paths.len() && !self.image_paths.is_empty() {
                        self.current_index = 0;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    fn next_image(&mut self, ctx: &egui::Context) {
        if !self.image_paths.is_empty() {
            self.current_index = (self.current_index + 1) % self.image_paths.len();
            self.load_image(ctx);
        }
    }

    fn refresh_image_list(&mut self) {
        let mut seen = HashSet::new();
        self.image_paths.clear();

        for (folder, enabled) in &self.folder_map {
            if *enabled {
                for path in get_image_paths(folder) {
                    if seen.insert(path.clone()) {
                        self.image_paths.push(path);
                    }
                }
            }
        }

        self.current_index = 0;
    }
}

impl App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let pointer_over = ctx.input(|i| i.pointer.hover_pos().is_some());

        if pointer_over {
            self.last_hover = Instant::now();
            if !self.decorations_visible {
                frame.set_decorations(true);
                self.decorations_visible = true;
            }
        } else if self.decorations_visible && self.last_hover.elapsed() > Duration::from_secs(2) {
            frame.set_decorations(false);
            self.decorations_visible = false;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.next_image(ctx);
        }

        if ctx.input(|i| i.pointer.secondary_clicked()) {
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                self.context_menu_pos = pos;
                self.show_context_menu = true;
            }
        }

        if let Some(target_name) = &self.target_exe_name {
            let active_hwnd = unsafe { GetForegroundWindow() };
            self.target_is_active = Self::get_exe_name_from_hwnd(active_hwnd)
                .map_or(false, |name| name == *target_name);

            let mut pt = POINT::default();
            unsafe { GetCursorPos(&mut pt) };
            let hovered_hwnd = unsafe { WindowFromPoint(pt) };
            self.target_is_hovered = Self::get_exe_name_from_hwnd(hovered_hwnd)
                .map_or(false, |name| name == *target_name);
        }

        if self.show_context_menu {
            egui::Area::new("right_click_menu")
                .fixed_pos(self.context_menu_pos)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        if ui.button("Next Image").clicked() {
                            self.next_image(ctx);
                        }

                        if ui.button("Folder Manager").clicked() {
                            self.show_folder_manager = true;
                            self.show_context_menu = false;
                        }

                        if ui.button("Add Folder").clicked() {
                            self.show_context_menu = false;
                            if let Some(new_folder) = FileDialog::new().set_title("Add Folder").pick_folder() {
                                self.folder_map.insert(new_folder.clone(), true);
                                self.image_paths.extend(get_image_paths(&new_folder));
                            }
                        }

                        if ui.button("Track EXE...").clicked() {
                            self.show_context_menu = false;
                            if let Some(path) = FileDialog::new().add_filter("EXE", &["exe"]).pick_file() {
                                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                    self.target_exe_name = Some(name.to_lowercase());
                                }
                            }
                        }

                        if ui.button("Close Menu").clicked() {
                            self.show_context_menu = false;
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(name) = &self.target_exe_name {
                ui.label(format!("Tracking {} | Active: {} | Hovered: {}", name, self.target_is_active, self.target_is_hovered));
            }

            if self.current_image.is_none() && self.image_paths.is_empty() {
                ui.label("No image to display. Right-click to add folders.");
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

        let mut apply_changes = false;

        if self.show_folder_manager {
            egui::Window::new("Folder Manager")
                .open(&mut self.show_folder_manager)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    for (folder, enabled) in &mut self.folder_map {
                        ui.checkbox(enabled, &format!("{}", folder.display()));
                    }

                    if ui.button("Apply Changes").clicked() {
                        apply_changes = true;
                    }
                });
        }

        if apply_changes {
            self.refresh_image_list();
            self.load_image(ctx);
        }
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
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    let folder = FileDialog::new()
        .set_title("Select an image folder")
        .pick_folder()
        .expect("No folder selected");

    let mut image_paths = get_image_paths(&folder);
    image_paths.shuffle(&mut thread_rng());

    let mut folder_map = HashMap::new();
    folder_map.insert(folder.clone(), true);

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        resizable: true,
        ..Default::default()
    };

    eframe::run_native(
        "Germi Board",
        native_options,
        Box::new(move |_cc| {
            Box::new(ImageViewerApp {
                image_paths,
                current_index: 0,
                current_image: None,
                texture: None,
                last_size: None,
                last_hover: Instant::now(),
                decorations_visible: true,
                folder_map,
                show_folder_manager: false,
                show_context_menu: false,
                context_menu_pos: egui::pos2(100.0, 100.0),
                target_exe_name: None,
                target_is_active: false,
                target_is_hovered: false,
            })
        }),
    );
}

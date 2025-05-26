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
        GetForegroundWindow, GetWindowThreadProcessId,
        WindowFromPoint, GetCursorPos,
    },
    Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    Win32::System::ProcessStatus::K32GetModuleBaseNameW,
    Win32::Foundation::{HWND, POINT, CloseHandle},
};

use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;

#[derive(Serialize, Deserialize, Default)]
struct ConfigData {
    folder_map: HashMap<PathBuf, bool>,
    target_exe_name: Option<String>,
    current_index: usize,
    is_pinned: bool,
    alarm_seconds: Option<u64>,
    alarm_sound_path: Option<PathBuf>,
}

struct ImageViewerApp {
    image_timer: Instant,
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
    elapsed_time: Duration,
    last_timer_check: Instant,
    is_pinned: bool,
    pin_state_changed: bool,
    alarm_duration: Option<Duration>,
    alarm_triggered: bool,
    alarm_sound_path: Option<PathBuf>,
    show_alarm_config: bool,
    alarm_seconds: Option<u64>,
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
            let _ = CloseHandle(handle);

            if len == 0 {
                return None;
            }

            Some(String::from_utf16_lossy(&buffer[..len as usize]).to_lowercase())
        }
    }

    fn save_config(&self) {
        let config = ConfigData {
            folder_map: self.folder_map.clone(),
            target_exe_name: self.target_exe_name.clone(),
            current_index: self.current_index,
            is_pinned: self.is_pinned,
            alarm_seconds: self.alarm_seconds,
            alarm_sound_path: self.alarm_sound_path.clone(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write("viewer_config.json", json);
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
                    self.image_timer = Instant::now();
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
            self.elapsed_time = Duration::ZERO;
            self.image_timer = Instant::now();
            self.last_timer_check = Instant::now();
            self.alarm_triggered = false;
            self.save_config();
        }
    }

    fn refresh_image_list(&mut self) {
        let mut collected_paths = Vec::new();
        let mut seen = HashSet::new();

        for (folder, enabled) in &self.folder_map {
            if *enabled {
                for path in get_image_paths(folder) {
                    if seen.insert(path.clone()) {
                        collected_paths.push(path);
                    }
                }
            }
        }
        collected_paths.shuffle(&mut rand::thread_rng()); 
        self.image_paths = collected_paths;
    }
}

impl App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        if self.pin_state_changed {
            use egui::WindowLevel;
            let level = if self.is_pinned {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
            self.pin_state_changed = false;
        }


        ctx.request_repaint_after(Duration::from_secs(1));
         egui::Area::new("")
        .fixed_pos(egui::pos2(10.0, 10.0))
        .show(ctx, |ui| {
            let now = Instant::now();

            if self.target_is_active {
                let delta = now.duration_since(self.last_timer_check);
                self.elapsed_time += delta;
            }

            self.last_timer_check = now;

            let minutes = self.elapsed_time.as_secs() / 60;
            let seconds = self.elapsed_time.as_secs() % 60;
            let timer_text = format!("{:02}:{:02}", minutes, seconds);

            ui.label(
                egui::RichText::new(timer_text)
                    .color(egui::Color32::RED)
                    .background_color(egui::Color32::from_rgb(30, 0, 0))
                    .font(egui::FontId::monospace(28.0)),
            );
        });



        let pointer_over = ctx.input(|i| i.pointer.hover_pos().is_some());

        if pointer_over {
            self.last_hover = Instant::now();
            if !self.decorations_visible {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                self.decorations_visible = true;
            }
        } else if self.decorations_visible && self.last_hover.elapsed() > Duration::from_secs(2) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
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
            let _ = unsafe { GetCursorPos(&mut pt) };
            let hovered_hwnd = unsafe { WindowFromPoint(pt) };
            self.target_is_hovered = Self::get_exe_name_from_hwnd(hovered_hwnd)
                .map_or(false, |name| name == *target_name);
        }

        //timer logic
        if let Some(alarm) = self.alarm_duration {
            if !self.alarm_triggered && self.elapsed_time >= alarm {
                self.alarm_triggered = true;
                println!("Alarm triggered at {:?}", self.elapsed_time); // Debug log
                if let Some(path) = &self.alarm_sound_path {
                    println!("Attempting to play: {:?}", path); // Debug log
                    play_alarm_sound(path.clone()); 
                }
            }
        }


        if self.show_context_menu {
            egui::Area::new("right_click_menu")
                .fixed_pos(self.context_menu_pos)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        if ui.button("Next Image").clicked() {
                            self.next_image(ctx);
                        }

                        if ui.button(if self.is_pinned { "Unpin from Top" } else { "Pin to Top" }).clicked() {
                            self.is_pinned = !self.is_pinned;
                            self.pin_state_changed = true;
                            self.save_config();
                            self.show_context_menu = false;
                        }

                        if ui.button("Folder Manager").clicked() {
                            self.show_folder_manager = true;
                            self.show_context_menu = false;
                        }

                        use rand::seq::SliceRandom;
                        

                        if ui.button("Add Folder").clicked() {
                            self.show_context_menu = false;
                            if let Some(new_folder) = FileDialog::new().set_title("Add Folder").pick_folder() {
                                self.folder_map.insert(new_folder.clone(), true);
                                let mut new_images = get_image_paths(&new_folder);
                                new_images.shuffle(&mut rand::thread_rng());
                                self.image_paths.extend(new_images);
                                self.save_config();
                            }
                        }

                        if ui.button("Set Alarm...").clicked() {
                            self.show_alarm_config = true;
                            self.show_context_menu = false;
                            self.save_config();
                        }

                        if ui.button("Track EXE...").clicked() {
                            self.show_context_menu = false;
                            if let Some(path) = FileDialog::new().add_filter("EXE", &["exe"]).pick_file() {
                                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                    self.target_exe_name = Some(name.to_lowercase());
                                    self.save_config();
                                }
                            }
                        }

                        if ui.button("Close Menu").clicked() {
                            self.show_context_menu = false;
                        }
                    });
                });
        }

        if self.show_alarm_config {
            egui::Window::new("Set Alarm").show(ctx, |ui| {
                if self.alarm_seconds.is_none() {
                    self.alarm_seconds = Some(180);
                }

                ui.add(
                    egui::Slider::new(self.alarm_seconds.as_mut().unwrap(), 10..=3600)
                        .text("Trigger Alarm After (sec)")
                );

                if ui.button("Choose Sound").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("Audio", &["mp3", "wav", "ogg", "mp4"]).pick_file() {
                        self.alarm_sound_path = Some(path);
                    }
                }

                if ui.button("Set Alarm").clicked() {
                    let seconds = self.alarm_seconds.unwrap_or(180);
                    self.alarm_duration = Some(Duration::from_secs(seconds));
                    self.alarm_triggered = false;
                    self.show_alarm_config = false;
                    self.save_config();
                }
            });
        }


        egui::CentralPanel::default().show(ctx, |ui| {

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
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target_size + egui::vec2(16.0, 56.0)));
                    self.last_size = Some(target_size);
                }

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

fn play_alarm_sound(path: PathBuf) {
    use std::io::BufReader;
    use rodio::{Decoder, OutputStream, Sink};

    println!("Trying to play {:?}", path);

    if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
        if let Ok(file) = std::fs::File::open(&path) {
            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                let sink = Sink::try_new(&stream_handle).unwrap();
                sink.append(source);
                sink.sleep_until_end(); // for testing
            } else {
                println!("Failed to decode audio");
            }
        } else {
            println!("Failed to open file: {:?}", path);
        }
    } else {
        println!("No audio output stream found");
    }
}

fn main() {
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    let mut folder_map = HashMap::new();
    let mut target_exe_name = None;
    let mut current_index = 0;
    let mut is_pinned = false;
    let mut alarm_seconds = None;
    let mut alarm_duration = None;
    let mut alarm_sound_path = None;

    if let Ok(data) = std::fs::read_to_string("viewer_config.json") {
        if let Ok(config) = serde_json::from_str::<ConfigData>(&data) {
            folder_map = config.folder_map;
            target_exe_name = config.target_exe_name;
            current_index = config.current_index;
            is_pinned = config.is_pinned;
            alarm_seconds = config.alarm_seconds;
            alarm_sound_path = config.alarm_sound_path.clone();
            alarm_duration = alarm_seconds.map(Duration::from_secs);
        }
    }

    let folder = if folder_map.is_empty() {
        let folder = FileDialog::new()
            .set_title("Select an image folder")
            .pick_folder()
            .expect("No folder selected");

        folder_map.insert(folder.clone(), true);
        folder
    } else {
        folder_map.keys().next().unwrap().clone()
    };

    let mut image_paths = get_image_paths(&folder);
    image_paths.shuffle(&mut thread_rng());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(800.0, 600.0))
            .with_resizable(true),
        ..Default::default()
    };


    let _ = eframe::run_native(
        "Germi Board",
        native_options,
        
        Box::new(move |_cc| {
            Box::new(ImageViewerApp {
                image_timer: Instant::now(),
                image_paths,
                current_index,
                current_image: None,
                texture: None,
                last_size: None,
                last_hover: Instant::now(),
                decorations_visible: true,
                folder_map,
                show_folder_manager: false,
                show_context_menu: false,
                context_menu_pos: egui::pos2(100.0, 100.0),
                target_exe_name,
                target_is_active: false,
                target_is_hovered: false,
                elapsed_time: Duration::ZERO,
                last_timer_check: Instant::now(),
                is_pinned,
                pin_state_changed: true,
                alarm_seconds,
                alarm_duration,
                alarm_triggered: false,
                alarm_sound_path,
                show_alarm_config: false,
            })
        }),
    );
}

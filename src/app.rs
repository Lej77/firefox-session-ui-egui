use core::f32;
use std::path::PathBuf;

use egui::Widget;

use crate::{
    background::EguiBackgroundWork,
    egui_utils::{FakeMutable, ObservableMutable},
    host::{self, WebSendable},
};

#[derive(Clone)]
pub enum Command {
    SetInputPath(String, WebSendable<rfd::FileHandle>),
    UpdateLoadedData(host::FileInfo),
    ParsedTabGroups(host::AllTabGroups),
    SetPreview(String),
    ChangeTabGroupSelection {
        open: bool,
        index: u32,
        select: bool,
    },
    SetSavePath(String),
    SetStatus(String),
    SaveLinksToFile,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct FirefoxSessionDataApp {
    #[serde(skip)]
    wizard_state: Option<Vec<host::FirefoxProfileInfo>>,

    input_path: String,
    #[serde(skip)]
    input_data: Option<WebSendable<rfd::FileHandle>>,

    #[serde(skip)]
    loaded_path: String,
    #[serde(skip)]
    loaded_data: Option<host::FileInfo>,
    #[serde(skip)]
    tab_groups: host::AllTabGroups,
    #[serde(skip)]
    selected_tab_groups: host::GenerateOptions,

    #[serde(skip)]
    preview: String,

    save_path: String,
    #[serde(skip)] // <- TODO: we want to persist this
    output_options: host::OutputOptions,

    #[serde(skip)]
    status: String,

    #[serde(skip)]
    background: EguiBackgroundWork<Command>,
}

impl Default for FirefoxSessionDataApp {
    fn default() -> Self {
        Self {
            wizard_state: None,

            input_path: "".to_owned(),
            input_data: None,

            loaded_path: String::new(),
            loaded_data: None,
            #[cfg(debug_assertions)]
            tab_groups: host::AllTabGroups {
                open: vec![
                    host::TabGroup {
                        index: 0,
                        name: "Window 1".into(),
                    },
                    host::TabGroup {
                        index: 1,
                        name: "Window 2".into(),
                    },
                ],
                closed: vec![host::TabGroup {
                    index: 3,
                    name: "Closed window 1".into(),
                }],
            },
            #[cfg(not(debug_assertions))]
            tab_groups: Default::default(),
            selected_tab_groups: Default::default(),

            preview: String::new(),

            // TODO: more robust finding of downloads folder.
            save_path: {
                #[cfg(not(target_family = "wasm"))]
                {
                    std::env::var("USERPROFILE")
                        .map(|home| home + r"\Downloads\firefox-links")
                        .unwrap_or_default()
                }
                #[cfg(target_family = "wasm")]
                {
                    String::new()
                }
            },
            output_options: Default::default(),

            status: String::new(),

            background: EguiBackgroundWork::default(),
        }
    }
}

impl FirefoxSessionDataApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        cc.egui_ctx.set_pixels_per_point(1.3);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }

    pub fn handle_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::SetInputPath(v, handle) => {
                self.input_path = v;
                self.input_data = Some(handle);
            }
            Command::UpdateLoadedData(new_data) => {
                self.loaded_data = Some(new_data);
            }
            Command::ParsedTabGroups(all_groups) => {
                self.tab_groups = all_groups;
                self.regenerate_preview(ctx)
            }
            Command::SetPreview(v) => {
                self.status = "Successfully loaded session data".to_string();
                self.preview = v;
            }
            Command::ChangeTabGroupSelection {
                index,
                open,
                select,
            } => {
                self.change_selected_tab_group(ctx, index, open, select);
            }
            Command::SetSavePath(v) => {
                self.save_path = v;
            }
            Command::SetStatus(v) => {
                self.status = v;
            }
            Command::SaveLinksToFile => {
                let Some(data) = self.loaded_data.clone() else {
                    return;
                };
                let save_path = PathBuf::from(self.save_path.as_str());
                let selected = self.selected_tab_groups.clone();
                let output_options = self.output_options.clone();

                self.status = "Saving links to file".to_string();
                self.background.spawn(ctx, async move {
                    Some(
                        if let Err(e) = data.save_links(save_path, selected, output_options).await {
                            Command::SetStatus(format!("Failed to save links to file: {e}"))
                        } else {
                            Command::SetStatus("Successfully saved links to a file".to_owned())
                        },
                    )
                });
            }
        }
    }

    fn change_selected_tab_group(
        &mut self,
        ctx: &egui::Context,
        index: u32,
        open: bool,
        select: bool,
    ) {
        let (mut indexes, mut other) = (
            &mut self.selected_tab_groups.open_group_indexes,
            &mut self.selected_tab_groups.closed_group_indexes,
        );
        if !open {
            std::mem::swap(&mut indexes, &mut other);
        }
        if select {
            let indexes = indexes.get_or_insert_with(Vec::new);
            other.get_or_insert_with(Vec::new);
            if !indexes.contains(&index) {
                indexes.push(index);
                self.regenerate_preview(ctx)
            }
        } else if let Some(indexes) = indexes {
            let len = indexes.len();
            indexes.retain(|v| *v != index);
            if indexes.len() != len {
                // Something was removed => update preview:
                if self.selected_tab_groups.selected_groups() == 0 {
                    // Nothing selected => select all open windows:
                    self.selected_tab_groups.open_group_indexes = None;
                    self.selected_tab_groups
                        .closed_group_indexes
                        .get_or_insert_with(Vec::new);
                }
                self.regenerate_preview(ctx)
            }
        }
    }

    fn regenerate_preview(&mut self, ctx: &egui::Context) {
        let Some(data) = self.loaded_data.clone() else {
            return;
        };
        let options = self.selected_tab_groups.clone();
        self.status = "Generating preview".to_string();
        self.background.spawn(ctx, async move {
            Some(match data.to_text_links(options).await {
                Ok(preview) => Command::SetPreview(preview),
                Err(e) => Command::SetStatus(format!("Failed to generate preview: {e}")),
            })
        });
    }

    pub fn load_input_data(&mut self, ctx: &egui::Context) {
        self.loaded_path.clone_from(&self.input_path);

        let mut data = host::FileInfo::new(PathBuf::from(self.input_path.clone()));
        data.file_handle = self.input_data.clone();
        self.loaded_data = Some(data.clone());
        self.selected_tab_groups.open_group_indexes = None;
        self.selected_tab_groups.closed_group_indexes = Some(Vec::new());
        self.status = "Reading input file".to_string();

        self.background.spawn(ctx, {
            let sender = self.background.sender().clone();
            let ctx = ctx.clone();
            async move {
                if let Err(e) = data.load_data().await {
                    return Some(Command::SetStatus(format!("Failed to read file: {e}")));
                };
                sender.send(&ctx, Command::UpdateLoadedData(data.clone()));
                loop {
                    match &data.data {
                        Some(host::FileData::Compressed { .. }) => {
                            sender.send(&ctx, Command::SetStatus("Decompressing data".to_string()));
                            if let Err(e) = data.decompress_data().await {
                                return Some(Command::SetStatus(format!(
                                    "Failed to decompress data: {e}"
                                )));
                            }
                        }
                        Some(host::FileData::Uncompressed { .. }) => {
                            sender
                                .send(&ctx, Command::SetStatus("Parsing session data".to_string()));
                            if let Err(e) = data.parse_session_data().await {
                                return Some(Command::SetStatus(format!(
                                    "Failed to parse session data: {e}"
                                )));
                            }
                        }
                        Some(host::FileData::Parsed { .. }) => {
                            return Some(match data.get_groups_from_session(true).await {
                                Ok(all_groups) => Command::ParsedTabGroups(all_groups),
                                Err(e) => Command::SetStatus(format!(
                                    "Failed to list windows in session: {e}"
                                )),
                            })
                        }
                        None => unreachable!("we just loaded the data"),
                    }
                    sender.send(&ctx, Command::UpdateLoadedData(data.clone()));
                }
            }
        });
    }
}

impl eframe::App for FirefoxSessionDataApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        while let Some(command) = self.background.poll_work() {
            self.handle_command(ctx, command);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no "File->Quit" menu item on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        if let Some(wizard) = &self.wizard_state {
            let mut finished = false;

            let modal_response = egui::Modal::new(egui::Id::new("wizard"))
                .frame(egui::Frame::popup(&ctx.style()).inner_margin(30_i8))
                .show(ctx, |ui| {
                    ui.add_space(10.);
                    ui.strong("Firefox Profiles:");
                    ui.add_space(10.);
                    egui_extras::TableBuilder::new(ui)
                        .sense(egui::Sense::click())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::remainder().clip(true))
                        .body(|mut body| {
                            for profile in wizard {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        egui::Label::new(&*profile.name()).selectable(false).ui(ui);
                                    });
                                    if row.response().clicked() {
                                        self.input_path = profile
                                            .find_sessionstore_file()
                                            .to_string_lossy()
                                            .into_owned();
                                        self.input_data = None;
                                        finished = true;
                                    }
                                });
                            }
                        });
                    ui.add_space(10.);
                });
            if modal_response.should_close() {
                self.wizard_state = None;
            } else if finished {
                self.wizard_state = None;
                self.load_input_data(ctx);
            }
        }

        egui::SidePanel::left("selected_windows")
            .min_width(120.0)
            .show(ctx, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    egui::warn_if_debug_build(ui);
                    powered_by_egui_and_eframe(ui);
                    ui.label("");

                    egui_extras::TableBuilder::new(ui)
                        .sense(egui::Sense::click())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::remainder().clip(true))
                        .body(|mut body| {
                            let mut open = true;
                            loop {
                                if open {
                                    // Open windows header:
                                    body.row(20.0, |mut row| {
                                        row.set_hovered(false);
                                        row.col(|col| {
                                            col.strong("Open Windows");
                                        });
                                    });
                                } else {
                                    // Empty space:
                                    body.row(20.0, |mut row| {
                                        row.set_hovered(false);
                                        row.col(|col| {
                                            col.label("");
                                        });
                                    });
                                    // Closed windows header:
                                    body.row(20.0, |mut row| {
                                        row.set_hovered(false);
                                        row.col(|col| {
                                            col.strong("Closed Windows");
                                        });
                                    });
                                }

                                for (index, group) in if open {
                                    self.tab_groups.open.iter()
                                } else {
                                    self.tab_groups.closed.iter()
                                }
                                .enumerate()
                                {
                                    let index = u32::try_from(index).unwrap();
                                    body.row(20.0, |mut row| {
                                        let is_selected = if open {
                                            self.selected_tab_groups.open_group_indexes.as_ref()
                                        } else {
                                            self.selected_tab_groups.closed_group_indexes.as_ref()
                                        }
                                        .is_some_and(|indexes| indexes.contains(&index));
                                        row.set_selected(is_selected);
                                        row.col(|ui| {
                                            ui.label(group.name.as_str());
                                        });
                                        if row.response().clicked() {
                                            self.background.sender().send(
                                                ctx,
                                                Command::ChangeTabGroupSelection {
                                                    open,
                                                    index,
                                                    select: !is_selected,
                                                },
                                            );
                                        }
                                    });
                                }

                                if open {
                                    open = false;
                                } else {
                                    break;
                                }
                            }
                        });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.horizontal(|ui| {
                ui.label("Path to sessionstore file:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Browse").clicked() {
                        log::debug!("Pressed browse input path button");

                        #[cfg(not(target_family = "wasm"))]
                        let handle_fut = crate::host::prompt_load_file(Some(_frame));
                        #[cfg(target_family = "wasm")]
                        let handle_fut = crate::host::prompt_load_file(None);

                        self.background.spawn(ctx, async move {
                            let handle = handle_fut.await?;
                            #[cfg(target_family = "wasm")]
                            let name = handle.file_name();
                            #[cfg(not(target_family = "wasm"))]
                            let name = handle.path().to_string_lossy().into_owned();
                            Some(Command::SetInputPath(name, WebSendable(handle)))
                        });
                    }
                    if cfg!(not(target_family = "wasm")) && ui.button("Wizard").clicked() {
                        log::debug!("Pressed Wizard button");
                        self.wizard_state = Some(host::FirefoxProfileInfo::all_profiles());
                        ctx.request_repaint();
                    }
                    egui::TextEdit::singleline(&mut ObservableMutable::new(
                        &mut self.input_path,
                        |_| {
                            log::trace!("Modified input path");
                            self.input_data = None;
                        },
                    ))
                    .desired_width(f32::INFINITY)
                    .ui(ui);
                })
            });
            ui.horizontal(|ui| {
                ui.label("Current data was loaded from:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Load new data").clicked() {
                        self.load_input_data(ctx);
                    }
                    egui::TextEdit::singleline(&mut FakeMutable(self.loaded_path.as_str()))
                        .desired_width(f32::INFINITY)
                        .ui(ui);
                })
            });

            ui.label("Tabs as links:");
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Status: ");
                    egui::TextEdit::singleline(&mut FakeMutable(self.status.as_str()))
                        .desired_width(f32::INFINITY)
                        .ui(ui);
                });

                ui.horizontal(|ui| {
                    if ui.button("Copy links to clipboard").clicked() {
                        let text_to_copy = self.preview.clone();
                        self.background.spawn(ctx, async move {
                            if let Err(e) =
                                crate::clipboard::write_text_to_clipboard(text_to_copy.as_str())
                                    .await
                            {
                                Some(Command::SetStatus(format!(
                                    "Failed to write to clipboard: {e}"
                                )))
                            } else {
                                Some(Command::SetStatus("Copied links to clipboard".to_owned()))
                            }
                        });
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Save links to file").clicked() {
                            self.background.sender().send(ctx, Command::SaveLinksToFile);
                        }

                        egui::ComboBox::from_label("Output format: ")
                            .selected_text(self.output_options.format.as_str())
                            .show_ui(ui, |ui| {
                                for &value in host::FormatInfo::all() {
                                    ui.selectable_value(
                                        &mut self.output_options.format,
                                        value,
                                        value.as_str(),
                                    )
                                    .on_hover_text(value.to_string());
                                }
                            })
                            .response
                            .on_hover_text(self.output_options.format.to_string());
                    });
                });

                if cfg!(not(target_family = "wasm")) {
                    ui.label("");

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.output_options.create_folder,
                            "Create folder if it doesn't exist",
                        );
                        ui.checkbox(
                            &mut self.output_options.overwrite,
                            "Overwrite file if it already exists",
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("File path to write links to:");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Browse").clicked() {
                                #[cfg(not(target_family = "wasm"))]
                                let handle_fut = crate::host::prompt_save_file(Some(_frame));
                                #[cfg(target_family = "wasm")]
                                let handle_fut = crate::host::prompt_save_file(None);

                                self.background.spawn(ctx, async move {
                                    let handle = handle_fut.await?;
                                    #[cfg(target_family = "wasm")]
                                    let name = handle.file_name();
                                    #[cfg(not(target_family = "wasm"))]
                                    let name = handle.path().to_string_lossy().into_owned();
                                    Some(Command::SetSavePath(name))
                                });
                            }
                            egui::TextEdit::singleline(&mut self.save_path)
                                .desired_width(f32::INFINITY)
                                .ui(ui);
                        })
                    });
                }

                ui.label("");

                // Item with flexible height has to be rendered last when we
                // already know how much space we have used for other items:
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        egui::TextEdit::multiline(&mut FakeMutable(self.preview.as_str()))
                            .desired_rows(100)
                            .desired_width(f32::INFINITY)
                            .ui(ui);
                    });
                });
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
    });
}

use crate::state::{AppState, FilterCategory};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Vec2};
use game_focus_scanner::Platform;

pub fn render_ui(state: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(
            Frame::none()
                .fill(Color32::from_rgb(15, 17, 23))
                .inner_margin(Margin::same(16.0)),
        )
        .show(ctx, |ui| {
            render_header(state, ui);
            ui.add_space(12.0);

            render_toolbar(state, ui);
            ui.add_space(12.0);

            render_game_list(state, ui);
        });
}

fn render_header(state: &mut AppState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("🎮 Game Focus Manager")
                .size(20.0)
                .strong()
                .color(Color32::WHITE),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(RichText::new("🚪 Exit").size(12.0).color(Color32::from_rgb(248, 113, 113)))
                .clicked()
            {
                state.quit_requested = true;
            }

            if ui
                .button(RichText::new("📥 Minimize to Tray").size(12.0).color(Color32::WHITE))
                .clicked()
            {
                state.minimize_requested = true;
            }

            if let Some(status) = state.status() {
                ui.label(
                    RichText::new(status)
                        .size(13.0)
                        .color(Color32::from_rgb(52, 211, 153)),
                );
            }
        });
    });

    ui.label(
        RichText::new("Keep windowed games active in background with sound and controller")
            .size(12.0)
            .color(Color32::from_rgb(156, 163, 175)),
    );
}

fn render_toolbar(state: &mut AppState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Search bar
        ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("🔍 Search games...")
                .desired_width(220.0),
        );

        ui.add_space(8.0);

        // Filter tabs
        let active_count = state.games.iter().filter(|g| g.enabled).count();

        filter_button(ui, state, FilterCategory::All, "All");
        filter_button(
            ui,
            state,
            FilterCategory::EnabledOnly,
            &format!("Active ({})", active_count),
        );
        filter_button(ui, state, FilterCategory::Steam, "Steam");
        filter_button(ui, state, FilterCategory::Epic, "Epic");
        filter_button(ui, state, FilterCategory::Custom, "Custom");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(RichText::new("➕ Add Custom Game").color(Color32::WHITE))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Game Executable", &["exe"])
                    .pick_file()
                {
                    state.add_custom_game(path);
                }
            }

            if ui.button("🔄 Rescan").clicked() {
                state.rescan();
            }
        });
    });
}

fn filter_button(
    ui: &mut egui::Ui,
    state: &mut AppState,
    cat: FilterCategory,
    label: &str,
) {
    let is_selected = state.filter == cat;
    let text = if is_selected {
        RichText::new(label)
            .strong()
            .color(Color32::from_rgb(167, 139, 250))
    } else {
        RichText::new(label).color(Color32::from_rgb(209, 213, 219))
    };

    if ui.selectable_label(is_selected, text).clicked() {
        state.filter = cat;
    }
}

fn render_game_list(state: &mut AppState, ui: &mut egui::Ui) {
    let game_ids: Vec<String> = state
        .filtered_games()
        .into_iter()
        .map(|g| g.id.clone())
        .collect();

    if game_ids.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(
                RichText::new("No games found matching your filter.")
                    .size(15.0)
                    .color(Color32::from_rgb(156, 163, 175)),
            );
            ui.add_space(8.0);
            if ui.button("➕ Add a game manually").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Game Executable", &["exe"])
                    .pick_file()
                {
                    state.add_custom_game(path);
                }
            }
        });
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for id in game_ids {
            render_game_card(state, ui, &id);
            ui.add_space(8.0);
        }
    });
}

fn render_game_card(state: &mut AppState, ui: &mut egui::Ui, id: &str) {
    let mut enable_clicked = false;
    let mut disable_clicked = false;
    let mut settings_changed = false;

    Frame::none()
        .fill(Color32::from_rgb(24, 27, 36))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 65)))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            let game = match state.games.iter_mut().find(|g| g.id == id) {
                Some(g) => g,
                None => return,
            };

            // Top Row: Title, Platform badge, Status pill, Action button
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&game.name)
                        .size(15.0)
                        .strong()
                        .color(Color32::WHITE),
                );

                // Platform badge
                let (badge_text, badge_bg) = match game.platform {
                    Platform::Steam => ("Steam", Color32::from_rgb(30, 64, 175)),
                    Platform::Epic => ("Epic", Color32::from_rgb(109, 40, 217)),
                    Platform::Custom => ("Custom", Color32::from_rgb(180, 83, 9)),
                };

                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    Rounding::same(4.0),
                    badge_bg,
                );

                ui.label(
                    RichText::new(badge_text)
                        .size(11.0)
                        .color(Color32::WHITE),
                );

                // Active status badge
                ui.add_space(4.0);
                if game.enabled {
                    ui.label(
                        RichText::new("● Active")
                            .size(12.0)
                            .color(Color32::from_rgb(52, 211, 153)),
                    );
                } else {
                    ui.label(
                        RichText::new("○ Inactive")
                            .size(12.0)
                            .color(Color32::from_rgb(156, 163, 175)),
                    );
                }

                // Action button aligned to right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if game.enabled {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Disable").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(220, 38, 38))
                                .min_size(Vec2::new(75.0, 24.0)),
                            )
                            .clicked()
                        {
                            disable_clicked = true;
                        }
                    } else {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Enable").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(16, 185, 129))
                                .min_size(Vec2::new(75.0, 24.0)),
                            )
                            .clicked()
                        {
                            enable_clicked = true;
                        }
                    }
                });
            });

            // Path subheader
            ui.label(
                RichText::new(game.exe_path.to_string_lossy())
                    .size(11.0)
                    .color(Color32::from_rgb(107, 114, 128)),
            );

            ui.add_space(6.0);

            // Toggles Row with descriptive tooltips
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut game.spoof_focus, "Spoof Focus")
                    .on_hover_text("Tricks the game engine into believing it is always the focused foreground window, preventing pause menus, background throttling, and deactivation.")
                    .changed()
                {
                    settings_changed = true;
                }
                if ui
                    .checkbox(&mut game.keep_audio, "Keep Audio")
                    .on_hover_text("Prevents the game from muting sound when you switch to another monitor or application.")
                    .changed()
                {
                    settings_changed = true;
                }
                if ui
                    .checkbox(&mut game.background_controller, "Background Controller")
                    .on_hover_text("Forces gamepads (Xbox, PlayStation, DirectInput, SDL) to remain active and send inputs to the game while in the background.")
                    .changed()
                {
                    settings_changed = true;
                }
                if ui
                    .checkbox(&mut game.unlock_cursor, "Unlock Cursor")
                    .on_hover_text("Bypasses ClipCursor so your mouse can freely leave the game window and move to your other monitors without getting trapped.")
                    .changed()
                {
                    settings_changed = true;
                }
            });
        });

    if enable_clicked {
        state.enable_game(id);
    } else if disable_clicked {
        state.disable_game(id);
    } else if settings_changed {
        state.update_game_settings(id);
    }
}

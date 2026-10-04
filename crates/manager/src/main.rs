#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod embedded;
mod state;
mod tray;
mod ui;

use eframe::egui::{self, Vec2, ViewportBuilder, ViewportCommand};
use parking_lot::Mutex;
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tray::SystemTray;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayAction {
    Show,
    Quit,
    Rescan,
}

static PENDING_ACTIONS: Mutex<Vec<TrayAction>> = Mutex::new(Vec::new());

fn restore_and_focus_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(ViewportCommand::Focus);

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, GetWindowThreadProcessId, SetForegroundWindow,
            ShowWindow, SW_RESTORE, SW_SHOW,
        };

        let title: Vec<u16> = "Game Focus Manager\0".encode_utf16().collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if !hwnd.is_null() {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == GetCurrentProcessId() {
                ShowWindow(hwnd, SW_RESTORE);
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
            }
        }
    }
}

struct GameFocusApp {
    state: AppState,
    tray: Option<SystemTray>,
    should_quit: Arc<AtomicBool>,
}

impl GameFocusApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let tray = match SystemTray::new() {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("Failed to initialize system tray: {}", e);
                None
            }
        };

        if let Some(t) = &tray {
            let show_id = t.show_item_id.clone();
            let quit_id = t.quit_item_id.clone();
            let rescan_id = t.rescan_item_id.clone();
            let ctx_menu = cc.egui_ctx.clone();

            // Menu handler: push action and wake up eframe
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if event.id == show_id {
                    PENDING_ACTIONS.lock().push(TrayAction::Show);
                } else if event.id == quit_id {
                    PENDING_ACTIONS.lock().push(TrayAction::Quit);
                } else if event.id == rescan_id {
                    PENDING_ACTIONS.lock().push(TrayAction::Rescan);
                }
                ctx_menu.request_repaint();
            }));

            let ctx_tray = cc.egui_ctx.clone();
            // Tray icon click handler: left click or double click restores window
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                match event {
                    TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        ..
                    }
                    | TrayIconEvent::DoubleClick {
                        button: tray_icon::MouseButton::Left,
                        ..
                    } => {
                        PENDING_ACTIONS.lock().push(TrayAction::Show);
                        ctx_tray.request_repaint();
                    }
                    _ => {}
                }
            }));
        }

        Self {
            state: AppState::new(),
            tray,
            should_quit: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl eframe::App for GameFocusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Drain pending tray actions
        let actions: Vec<TrayAction> = {
            let mut lock = PENDING_ACTIONS.lock();
            std::mem::take(&mut *lock)
        };

        let mut just_restored = false;
        for action in actions {
            match action {
                TrayAction::Quit => {
                    self.should_quit.store(true, Ordering::Relaxed);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                TrayAction::Show => {
                    restore_and_focus_window(ctx);
                    just_restored = true;
                }
                TrayAction::Rescan => {
                    self.state.rescan();
                    restore_and_focus_window(ctx);
                    just_restored = true;
                }
            }
        }

        // 2. Handle window close -> minimize to system tray unless Quit was selected
        if !just_restored && ctx.input(|i| i.viewport().close_requested()) {
            if !self.should_quit.load(Ordering::Relaxed) && self.tray.is_some() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                self.state.set_status("Minimized to system tray");
            }
        }

        // 3. Render main UI
        ui::render_ui(&mut self.state, ctx);

        // Keep polling smoothly
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Game Focus Manager")
            .with_inner_size(Vec2::new(880.0, 640.0))
            .with_min_inner_size(Vec2::new(650.0, 450.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Game Focus Manager",
        native_options,
        Box::new(|cc| Ok(Box::new(GameFocusApp::new(cc)))),
    )
}

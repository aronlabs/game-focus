#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod embedded;
mod state;
mod tray;
mod ui;

use eframe::egui::{self, Vec2, ViewportBuilder, ViewportCommand};
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tray::SystemTray;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;

struct GameFocusApp {
    state: AppState,
    tray: Option<SystemTray>,
    should_quit: Arc<AtomicBool>,
}

fn restore_and_focus_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(ViewportCommand::Focus);

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, SetForegroundWindow, ShowWindow, SW_RESTORE,
            SW_SHOW,
        };

        unsafe extern "system" fn enum_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == GetCurrentProcessId() {
                ShowWindow(hwnd, SW_RESTORE);
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
                return 0; // Found main window, stop
            }
            1
        }

        let _ = EnumWindows(Some(enum_proc), 0);
    }
}

impl GameFocusApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set dark theme default
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        // Ensure tray menu & click events wake up eframe's event loop even when the window is hidden!
        let ctx_menu = cc.egui_ctx.clone();
        MenuEvent::set_event_handler(Some(move |_| {
            ctx_menu.request_repaint();
        }));

        let ctx_tray = cc.egui_ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |_| {
            ctx_tray.request_repaint();
        }));

        let tray = match SystemTray::new() {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("Failed to initialize system tray: {}", e);
                None
            }
        };

        Self {
            state: AppState::new(),
            tray,
            should_quit: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl eframe::App for GameFocusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process tray menu events
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(tray) = &self.tray {
                if event.id == tray.quit_item_id {
                    self.should_quit.store(true, Ordering::Relaxed);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                } else if event.id == tray.show_item_id {
                    restore_and_focus_window(ctx);
                } else if event.id == tray.rescan_item_id {
                    self.state.rescan();
                    restore_and_focus_window(ctx);
                }
            }
        }

        // 2. Process tray icon click events (single or double click restores window)
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                TrayIconEvent::Click { .. } | TrayIconEvent::DoubleClick { .. } => {
                    restore_and_focus_window(ctx);
                }
                _ => {}
            }
        }

        // 3. Handle window close -> minimize to system tray unless Quit was selected
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.should_quit.load(Ordering::Relaxed) && self.tray.is_some() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                self.state.set_status("Minimized to system tray");
            }
        }

        // 4. Render main UI
        ui::render_ui(&mut self.state, ctx);

        // Keep polling for tray events smoothly
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
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

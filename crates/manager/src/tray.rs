use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct SystemTray {
    pub _tray: TrayIcon,
    pub show_item_id: String,
    pub rescan_item_id: String,
    pub quit_item_id: String,
}

impl SystemTray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let tray_menu = Menu::new();

        let show_item = MenuItem::new("Show Game Focus Manager", true, None);
        let rescan_item = MenuItem::new("Rescan Games", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let show_id = show_item.id().0.clone();
        let rescan_id = rescan_item.id().0.clone();
        let quit_id = quit_item.id().0.clone();

        tray_menu.append(&show_item)?;
        tray_menu.append(&rescan_item)?;
        tray_menu.append(&PredefinedMenuItem::separator())?;
        tray_menu.append(&quit_item)?;

        let icon = create_default_icon()?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Game Focus Manager")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _tray: tray,
            show_item_id: show_id,
            rescan_item_id: rescan_id,
            quit_item_id: quit_id,
        })
    }
}

fn create_default_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    const WIDTH: u32 = 32;
    const HEIGHT: u32 = 32;
    let mut rgba = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            // Draw a controller-like colored rounded icon (purple/cyan)
            let is_controller_body = (y >= 10 && y <= 22 && x >= 4 && x <= 27)
                || (y >= 14 && y <= 26 && ((x >= 4 && x <= 10) || (x >= 21 && x <= 27)));

            if is_controller_body {
                rgba.extend_from_slice(&[138, 92, 246, 255]); // Purple accent
            } else if (x == 8 && (y >= 13 && y <= 19)) || (y == 16 && (x >= 5 && x <= 11)) {
                rgba.extend_from_slice(&[255, 255, 255, 255]); // D-pad
            } else if (x == 23 || x == 25) && (y == 14 || y == 18) {
                rgba.extend_from_slice(&[56, 189, 248, 255]); // Action buttons (cyan)
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]); // Transparent
            }
        }
    }

    Ok(Icon::from_rgba(rgba, WIDTH, HEIGHT)?)
}

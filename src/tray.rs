use image::{Rgba, RgbaImage};
use ab_glyph::FontRef;
use imageproc::drawing::{draw_text_mut, text_size};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, };

// Disconnected icon value
const HEADSET_ICON: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/headset.png"));

// Tooltip const
const DISCONNECTED_TEXT: &'static str = "Disconnected";

// Battery level icon const
const ICON_SIZE: u32 = 32;
const ICON_COLOR: Rgba<u8> = Rgba([255u8, 255, 255, 255]);
const ICON_FONT: &[u8] = include_bytes!("/usr/share/fonts/noto/NotoSans-Medium.ttf");

pub struct HeadsetTrayIcon {
    disconnected_icon: Icon,
    pub tray: TrayIcon,
}

impl HeadsetTrayIcon {
    pub fn new() -> Self {
        let icon_image = image
                ::load_from_memory(HEADSET_ICON)
                .expect("No tray icon image found.")
                .to_rgba8();

        let (width, height) = icon_image.dimensions();

        let icon = Icon::from_rgba(icon_image.into_raw(), width, height).expect("Could not instantiate icon image for tray.");

        let tray = TrayIconBuilder::new()
                .with_tooltip(DISCONNECTED_TEXT)
                .with_icon(icon.clone())
                .build()
                .expect("Could not start tray icon.");

        Self { disconnected_icon: icon, tray: tray }
    }

    pub fn set_disconnected(&mut self) {
        self.tray.set_icon(Some(self.disconnected_icon.clone())).and(
            self.tray.set_tooltip(Some(DISCONNECTED_TEXT))
        ).expect("Failed to change tray icon to disconnected state");
    }

    pub fn change_battery_level(&mut self, percentage: u8, charging: Option<bool>) {
        let generated_image = create_battery_level_text(percentage);
        let (width, height) = generated_image.dimensions();

        let icon = Icon::from_rgba(generated_image.into_raw(), width, height).expect("Could not instantiate icon image for tray.");
        self.tray.set_icon(Some(icon)).expect("Failed to change tray battery level icon");

        let tooltip = if charging.and_then(|v| v.into()).is_some() {
            format!("{}% (charging)", percentage)
        } else {
            format!("{}%", percentage)
        };

        self.tray.set_tooltip(Some(tooltip)).expect("Failed to change tray battery level tooltip")
    }
}

fn create_battery_level_text(percentage: u8) -> RgbaImage {
    let mut image = RgbaImage::new(ICON_SIZE, ICON_SIZE);

    let scale = 40.0f32;
    let font = FontRef::try_from_slice(ICON_FONT).unwrap();
    let text = format!("{percentage}");

    // get the size of the text and calculate the x, y coordinate where to start to be center aligned
    // both horizontally and vertically
    let (text_width, text_height) = text_size(scale, &font, &text);
    println!("Text size: {}x{}", text_width, text_height);
    let text_start_x = ((ICON_SIZE as i32) - (text_width as i32)) / 2i32;
    let text_start_y = ((ICON_SIZE as i32) - (text_height as i32)) / 2i32;

    draw_text_mut(&mut image, ICON_COLOR, text_start_x, text_start_y, scale, &font, &text);

    image.save("/tmp/percentage.png").expect("Failed to write image");

    println!("Image saved");
    image
}

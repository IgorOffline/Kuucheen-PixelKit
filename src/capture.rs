use crate::{APP_ID, color::Rgb, measurement::PixelSource};
use anyhow::{Context, Result, anyhow, bail};
use percent_encoding::percent_decode_str;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use x11rb::{
    connection::Connection,
    protocol::{
        randr::ConnectionExt as _,
        xproto::{ConnectionExt as _, ImageFormat, ImageOrder, Screen},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    X11,
    Portal,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl CaptureRegion {
    pub fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesktopLayout {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub monitors: Vec<CaptureRegion>,
    pub active_monitor: usize,
}

impl DesktopLayout {
    fn matches_capture(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

#[derive(Debug, Clone)]
pub struct CaptureFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub dpi: f32,
    pub backend: CaptureBackend,
    pub desktop_layout: Option<DesktopLayout>,
}

impl CaptureFrame {
    pub fn from_path(path: &Path, dpi: f32) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read screenshot {}", path.display()))?;
        let image = image::load_from_memory(&bytes)
            .with_context(|| format!("failed to decode screenshot {}", path.display()))?
            .into_rgba8();
        let (width, height) = image.dimensions();
        Ok(Self {
            width,
            height,
            rgba: image.into_raw(),
            dpi,
            backend: CaptureBackend::File,
            desktop_layout: None,
        })
    }

    pub fn full_region(&self) -> CaptureRegion {
        CaptureRegion {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        }
    }

    pub fn pixel_checked(&self, x: u32, y: u32) -> Option<Rgb> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = ((y * self.width + x) * 4) as usize;
        Some(Rgb::new(
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
        ))
    }

    pub(crate) fn egui_image_region(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> eframe::egui::ColorImage {
        assert!(
            width > 0 && height > 0,
            "capture texture region must not be empty"
        );
        assert!(
            x <= self.width.saturating_sub(width) && y <= self.height.saturating_sub(height),
            "capture texture region must be inside the source image"
        );

        let source_width = self.width as usize;
        let x = x as usize;
        let y = y as usize;
        let width = width as usize;
        let height = height as usize;

        if x == 0 && width == source_width {
            let start = y * source_width * 4;
            let end = start + width * height * 4;
            return eframe::egui::ColorImage::from_rgba_unmultiplied(
                [width, height],
                &self.rgba[start..end],
            );
        }

        let mut region = Vec::with_capacity(width * height * 4);
        for row in y..y + height {
            let start = (row * source_width + x) * 4;
            region.extend_from_slice(&self.rgba[start..start + width * 4]);
        }
        eframe::egui::ColorImage::from_rgba_unmultiplied([width, height], &region)
    }
}

impl PixelSource for CaptureFrame {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn pixel(&self, x: u32, y: u32) -> Rgb {
        self.pixel_checked(x, y).unwrap_or_default()
    }
}

pub fn is_wayland_session() -> bool {
    env::var_os("WAYLAND_DISPLAY").is_some()
        || env::var("XDG_SESSION_TYPE").is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
}

pub fn capture_screen(interactive_portal: bool, fallback_dpi: f32) -> Result<CaptureFrame> {
    let force_portal = env::var_os("PIXELKIT_FORCE_PORTAL").is_some();
    if !force_portal && !is_wayland_session() && env::var_os("DISPLAY").is_some() {
        return capture_x11().context("X11 screen capture failed");
    }
    match capture_portal(interactive_portal, fallback_dpi) {
        Ok(frame) => Ok(frame),
        Err(portal_error) if env::var_os("DISPLAY").is_some() && !force_portal => {
            capture_x11().with_context(|| format!("Wayland screenshot portal failed ({portal_error:#}); XWayland fallback also failed"))
        }
        Err(error) => Err(error).context("Wayland screenshot portal failed; ensure xdg-desktop-portal and a desktop portal backend are installed"),
    }
}

fn capture_portal(interactive: bool, fallback_dpi: f32) -> Result<CaptureFrame> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let uri = runtime.block_on(async {
        // Do not use ashpd's process-global cached connection here. Recapture
        // runs on a fresh worker/runtime and a connection owned by an earlier
        // short-lived runtime can no longer make progress.
        let connection = ashpd::zbus::Connection::session().await?;
        if let Ok(app_id) = APP_ID.parse()
            && let Err(error) =
                ashpd::register_host_app_with_connection(connection.clone(), app_id).await
        {
            // Screenshot works for host applications without a persistent
            // identity on older portals, so registration is best-effort here.
            eprintln!("PixelKit: could not register screenshot portal identity: {error}");
        }
        let response = ashpd::desktop::screenshot::Screenshot::request()
            .interactive(interactive)
            .modal(false)
            .connection(Some(connection))
            .send()
            .await?
            .response()?;
        Ok::<String, ashpd::Error>(response.uri().as_str().to_owned())
    })?;
    let path = file_uri_to_path(&uri)?;
    let mut frame = CaptureFrame::from_path(&path, fallback_dpi)?;
    frame.backend = CaptureBackend::Portal;
    frame.desktop_layout =
        x11_desktop_layout().filter(|layout| layout.matches_capture(frame.width, frame.height));
    Ok(frame)
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    let encoded = uri
        .strip_prefix("file://")
        .ok_or_else(|| anyhow!("portal returned unsupported screenshot URI: {uri}"))?;
    // Portal screenshot URIs are local. A non-empty host is deliberately
    // rejected rather than accidentally treating it as part of a path.
    let encoded = if encoded.starts_with('/') {
        encoded
    } else {
        bail!("portal returned a non-local screenshot URI: {uri}")
    };
    let decoded = percent_decode_str(encoded)
        .decode_utf8()
        .context("portal screenshot path is not valid UTF-8")?;
    Ok(PathBuf::from(decoded.as_ref()))
}

fn capture_x11() -> Result<CaptureFrame> {
    let (connection, screen_index) =
        x11rb::connect(None).context("could not connect to the X server")?;
    let setup = connection.setup();
    let screen = setup
        .roots
        .get(screen_index)
        .context("X server has no selected screen")?;
    let width = screen.width_in_pixels;
    let height = screen.height_in_pixels;
    let format = setup
        .pixmap_formats
        .iter()
        .find(|format| format.depth == screen.root_depth)
        .with_context(|| {
            format!(
                "X server has no pixmap format for depth {}",
                screen.root_depth
            )
        })?;
    let visual = screen
        .allowed_depths
        .iter()
        .flat_map(|depth| depth.visuals.iter())
        .find(|visual| visual.visual_id == screen.root_visual)
        .context("X server root visual was not advertised")?;
    let reply = connection
        .get_image(
            ImageFormat::Z_PIXMAP,
            screen.root,
            0,
            0,
            width,
            height,
            u32::MAX,
        )?
        .reply()?;
    let bits_per_pixel = usize::from(format.bits_per_pixel);
    if !matches!(bits_per_pixel, 16 | 24 | 32) {
        bail!("unsupported X11 pixel size: {bits_per_pixel} bits");
    }
    let scanline_pad = usize::from(format.scanline_pad);
    let stride_bits = (usize::from(width) * bits_per_pixel).div_ceil(scanline_pad) * scanline_pad;
    let stride = stride_bits / 8;
    if reply.data.len() < stride * usize::from(height) {
        bail!("X11 returned a truncated screenshot");
    }
    let bytes_per_pixel = bits_per_pixel / 8;
    let little_endian = setup.image_byte_order == ImageOrder::LSB_FIRST;
    let mut rgba = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
    for y in 0..usize::from(height) {
        for x in 0..usize::from(width) {
            let offset = y * stride + x * bytes_per_pixel;
            let bytes = &reply.data[offset..offset + bytes_per_pixel];
            let pixel = if little_endian {
                bytes
                    .iter()
                    .enumerate()
                    .fold(0_u32, |value, (index, byte)| {
                        value | (u32::from(*byte) << (index * 8))
                    })
            } else {
                bytes
                    .iter()
                    .fold(0_u32, |value, byte| (value << 8) | u32::from(*byte))
            };
            rgba.push(mask_channel(pixel, visual.red_mask));
            rgba.push(mask_channel(pixel, visual.green_mask));
            rgba.push(mask_channel(pixel, visual.blue_mask));
            rgba.push(255);
        }
    }
    let dpi = if screen.width_in_millimeters > 0 {
        f32::from(width) / f32::from(screen.width_in_millimeters) * 25.4
    } else {
        96.0
    };
    let desktop_layout = desktop_layout_for_x11(&connection, screen)
        .ok()
        .filter(|layout| layout.matches_capture(u32::from(width), u32::from(height)));
    Ok(CaptureFrame {
        width: u32::from(width),
        height: u32::from(height),
        rgba,
        dpi,
        backend: CaptureBackend::X11,
        desktop_layout,
    })
}

pub(crate) fn x11_desktop_layout() -> Option<DesktopLayout> {
    let (connection, screen_index) = x11rb::connect(None).ok()?;
    let screen = connection.setup().roots.get(screen_index)?;
    desktop_layout_for_x11(&connection, screen).ok()
}

fn desktop_layout_for_x11(connection: &impl Connection, screen: &Screen) -> Result<DesktopLayout> {
    let reply = connection
        .randr_get_monitors(screen.root, true)?
        .reply()
        .context("X server did not return monitor geometry")?;
    let width = u32::from(screen.width_in_pixels);
    let height = u32::from(screen.height_in_pixels);
    let mut monitors = Vec::new();
    let mut primary_monitor = None;
    let mut monitor_dimensions = Vec::new();
    for monitor in reply.monitors {
        let Ok(x) = u32::try_from(monitor.x) else {
            continue;
        };
        let Ok(y) = u32::try_from(monitor.y) else {
            continue;
        };
        let region = CaptureRegion {
            x,
            y,
            width: u32::from(monitor.width),
            height: u32::from(monitor.height),
        };
        if region.width == 0
            || region.height == 0
            || region.x.saturating_add(region.width) > width
            || region.y.saturating_add(region.height) > height
        {
            continue;
        }
        if monitor.primary {
            primary_monitor = Some(monitors.len());
        }
        monitor_dimensions.push((monitor.width_in_millimeters, monitor.height_in_millimeters));
        monitors.push(region);
    }
    if monitors.is_empty() {
        bail!("X server did not advertise any active monitors");
    }

    let pointer = connection.query_pointer(screen.root)?.reply()?;
    let pointer_x = u32::try_from(pointer.root_x).ok();
    let pointer_y = u32::try_from(pointer.root_y).ok();
    let active_monitor = pointer_x
        .zip(pointer_y)
        .and_then(|(x, y)| monitors.iter().position(|monitor| monitor.contains(x, y)))
        .or(primary_monitor)
        .unwrap_or(0);
    let scale_factor = x11_scale_factor(
        connection,
        monitors[active_monitor],
        monitor_dimensions[active_monitor],
    );

    Ok(DesktopLayout {
        width,
        height,
        scale_factor,
        monitors,
        active_monitor,
    })
}

fn x11_scale_factor(
    connection: &impl Connection,
    active_monitor: CaptureRegion,
    physical_size: (u32, u32),
) -> f32 {
    let override_value = env::var("WINIT_X11_SCALE_FACTOR")
        .ok()
        .or_else(|| env::var("WINIT_HIDPI_FACTOR").ok());
    if let Some(value) = override_value {
        if value.eq_ignore_ascii_case("randr") {
            return randr_scale_factor(active_monitor, physical_size);
        }
        if let Ok(scale) = value.parse::<f32>()
            && valid_scale_factor(scale)
        {
            return scale;
        }
    }

    if let Ok(database) = x11rb::resource_manager::new_from_default(connection)
        && let Ok(Some(dpi)) = database.get_value::<f32>("Xft.dpi", "")
    {
        let scale = dpi / 96.0;
        if valid_scale_factor(scale) {
            return scale;
        }
    }

    randr_scale_factor(active_monitor, physical_size)
}

fn randr_scale_factor(region: CaptureRegion, physical_size: (u32, u32)) -> f32 {
    let (width_mm, height_mm) = physical_size;
    if width_mm == 0 || height_mm == 0 {
        return 1.0;
    }
    let pixels = f64::from(region.width) * f64::from(region.height);
    let millimetres = f64::from(width_mm) * f64::from(height_mm);
    let pixels_per_millimetre = (pixels / millimetres).sqrt();
    let scale = ((pixels_per_millimetre * (12.0 * 25.4 / 96.0)).round() / 12.0).max(1.0);
    if scale <= 20.0 { scale as f32 } else { 1.0 }
}

fn valid_scale_factor(scale: f32) -> bool {
    scale.is_finite() && scale > 0.0 && scale <= 20.0
}

fn mask_channel(pixel: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let maximum = mask >> shift;
    let value = (pixel & mask) >> shift;
    ((value * 255 + maximum / 2) / maximum) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_scaling_supports_rgb565() {
        assert_eq!(
            mask_channel(0b1111_1000_0000_0000, 0b1111_1000_0000_0000),
            255
        );
        assert_eq!(mask_channel(0, 0b11111), 0);
    }

    #[test]
    fn local_portal_uri_is_decoded() {
        assert_eq!(
            file_uri_to_path("file:///tmp/PixelKit%20Shot.png").unwrap(),
            PathBuf::from("/tmp/PixelKit Shot.png")
        );
        assert!(file_uri_to_path("https://example.test/image.png").is_err());
    }

    #[test]
    fn gpu_texture_regions_preserve_every_source_pixel() {
        let mut rgba = Vec::new();
        for value in 0..8_u8 {
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
        let frame = CaptureFrame {
            width: 4,
            height: 2,
            rgba,
            dpi: 96.0,
            backend: CaptureBackend::File,
            desktop_layout: None,
        };
        let left = frame.egui_image_region(0, 0, 2, 2);
        let right = frame.egui_image_region(2, 0, 2, 2);
        assert_eq!(left.size, [2, 2]);
        assert_eq!(right.size, [2, 2]);
        assert_eq!(left.pixels[3], eframe::egui::Color32::from_gray(5));
        assert_eq!(right.pixels[0], eframe::egui::Color32::from_gray(2));
        assert_eq!(right.pixels[3], eframe::egui::Color32::from_gray(7));
        assert_eq!(frame.pixel_checked(3, 1), Some(Rgb::new(7, 7, 7)));
    }

    #[test]
    fn capture_regions_use_exclusive_right_and_bottom_edges() {
        let region = CaptureRegion {
            x: 100,
            y: 50,
            width: 200,
            height: 100,
        };
        assert!(region.contains(100, 50));
        assert!(region.contains(299, 149));
        assert!(!region.contains(300, 149));
        assert!(!region.contains(299, 150));
    }

    #[test]
    fn randr_scale_matches_winit_quantization() {
        let region = CaptureRegion {
            x: 0,
            y: 0,
            width: 2240,
            height: 1400,
        };
        assert_eq!(randr_scale_factor(region, (302, 189)), 2.0);
        assert_eq!(randr_scale_factor(region, (0, 0)), 1.0);
    }
}

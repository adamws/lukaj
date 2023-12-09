use clap::ValueEnum;
use log::{debug, trace};
use sdl2::event::Event;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::video::Window;
use sdl2::video::WindowContext;
use sdl2::VideoSubsystem;
use std::cmp;
use std::collections::HashSet;
use std::env;
use std::path::Path;

#[cfg(feature = "use-usvg")]
use usvg::{fontdb, TreeParsing, TreeTextToPath};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum SvgBackend {
    #[cfg(feature = "use-rsvg")]
    RsvgWithCairo,
    #[cfg(feature = "use-usvg")]
    UsvgWithSkia,
}

trait SvgTextureBuilder<'a> {
    fn query_size(&self, scale: f64) -> Result<Rect, String>;

    fn rasterize(
        &self,
        texture_creator: &'a TextureCreator<WindowContext>,
        scale: f64,
    ) -> Result<Texture<'a>, String>;
}

#[cfg(feature = "use-rsvg")]
struct RsvgWithCairo {
    handle: rsvg::SvgHandle,
}

#[cfg(feature = "use-rsvg")]
impl RsvgWithCairo {
    fn new<P: AsRef<Path>>(path: P) -> Result<RsvgWithCairo, String> {
        let mut handle = rsvg::Loader::new()
            .read_path(path)
            .map_err(|e| e.to_string())?;
        // TODO: crispEdges should be optional
        handle
            .set_stylesheet(":root { shape-rendering: crispEdges; } ")
            .map_err(|e| e.to_string())?;
        Ok(RsvgWithCairo { handle })
    }
}

#[cfg(feature = "use-rsvg")]
impl<'a> SvgTextureBuilder<'a> for RsvgWithCairo {
    fn query_size(&self, scale: f64) -> Result<Rect, String> {
        let size = rsvg::CairoRenderer::new(&self.handle)
            .intrinsic_size_in_pixels()
            .ok_or("ERROR: Could not determine SVG size in pixels")?;

        let width = f64::ceil(size.0 * scale) as u32;
        let height = f64::ceil(size.1 * scale) as u32;

        Ok(Rect::new(0, 0, width, height))
    }

    fn rasterize(
        &self,
        texture_creator: &'a TextureCreator<WindowContext>,
        scale: f64,
    ) -> Result<Texture<'a>, String> {
        let size = self.query_size(scale)?;

        let mut texture: Texture<'a> = texture_creator
            .create_texture_streaming(
                sdl2::pixels::PixelFormatEnum::ARGB8888,
                size.width(),
                size.height(),
            )
            .map_err(|e| e.to_string())?;

        let _ = texture.with_lock(
            None,
            |buffer: &mut [u8], pitch: usize| -> Result<(), String> {
                let data_ptr: *mut u8 = buffer.as_mut_ptr();
                let surface = unsafe {
                    cairo::ImageSurface::create_for_data_unsafe(
                        data_ptr,
                        cairo::Format::ARgb32,
                        size.width() as i32,
                        size.height() as i32,
                        pitch as i32,
                    )
                }
                .map_err(|e| e.to_string())?;
                let cr = cairo::Context::new(&surface).map_err(|e| e.to_string())?;
                rsvg::CairoRenderer::new(&self.handle)
                    .render_document(
                        &cr,
                        &cairo::Rectangle::new(0.0, 0.0, size.width() as f64, size.height() as f64),
                    )
                    .map_err(|e| e.to_string())?;
                Ok(())
            },
        )?;

        texture.set_blend_mode(sdl2::render::BlendMode::Blend);
        Ok(texture)
    }
}

#[cfg(feature = "use-usvg")]
struct UsvgWithSkia {
    tree: resvg::Tree,
}

#[cfg(feature = "use-usvg")]
impl UsvgWithSkia {
    fn new<P: AsRef<Path>>(path: P) -> Result<UsvgWithSkia, String> {
        let tree = {
            let mut opt = usvg::Options::default();
            // Get file's absolute directory.
            opt.resources_dir = std::fs::canonicalize(&path)
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()));

            let mut fontdb = fontdb::Database::new();
            fontdb.load_system_fonts();

            let svg_data = std::fs::read(&path).map_err(|e| e.to_string())?;
            let mut tree = usvg::Tree::from_data(&svg_data, &opt).map_err(|e| e.to_string())?;
            tree.convert_text(&fontdb);
            resvg::Tree::from_usvg(&tree)
        };
        Ok(UsvgWithSkia { tree })
    }
}

#[cfg(feature = "use-usvg")]
impl<'a> SvgTextureBuilder<'a> for UsvgWithSkia {
    fn query_size(&self, scale: f64) -> Result<Rect, String> {
        let pixmap_size = self
            .tree
            .size
            .to_int_size()
            .scale_by(scale as f32)
            .ok_or(format!("ERROR: Could not scale SVG by factor of {}", scale))?;
        Ok(Rect::new(0, 0, pixmap_size.width(), pixmap_size.height()))
    }

    fn rasterize(
        &self,
        texture_creator: &'a TextureCreator<WindowContext>,
        scale: f64,
    ) -> Result<Texture<'a>, String> {
        let size = self.query_size(scale)?;

        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
            .ok_or("ERROR: Failed to create new pixmap")?;
        let render_ts = tiny_skia::Transform::from_scale(scale as f32, scale as f32);
        self.tree.render(render_ts, &mut pixmap.as_mut());

        let mut texture: Texture<'a> = texture_creator
            .create_texture_streaming(
                sdl2::pixels::PixelFormatEnum::ABGR8888,
                size.width(),
                size.height(),
            )
            .map_err(|e| e.to_string())?;

        texture.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
            buffer.copy_from_slice(pixmap.data());
        })?;

        texture.set_blend_mode(sdl2::render::BlendMode::Blend);
        return Ok(texture);
    }
}

trait CanvasEntity {
    fn draw(&self, renderer: &mut sdl2::render::WindowCanvas) -> Result<(), String>;

    fn reposition(&mut self, position: Point) -> Result<(), String>;
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    Left,
    Right,
}

struct SplitView<'a> {
    texture: Texture<'a>,
    width: u32,
    height: u32,
    side: Side,
    position: Point,
    split: u32,
}

impl<'a> SplitView<'a> {
    fn new(texture: Texture<'a>, side: Side) -> SplitView<'a> {
        let query = texture.query();
        SplitView {
            texture,
            width: query.width,
            height: query.height,
            side,
            position: Point::new(0, 0),
            split: 0,
        }
    }

    fn split(&mut self, split: u32) {
        self.split = split.clamp(0, self.width)
    }
}

impl<'a> CanvasEntity for SplitView<'a> {
    fn draw(&self, renderer: &mut sdl2::render::WindowCanvas) -> Result<(), String> {
        let texture = &self.texture;

        let src: Rect;
        let dst: Rect;
        match self.side {
            Side::Left => {
                src = Rect::new(0, 0, self.split, self.height);
                dst = Rect::new(self.position.x, self.position.y, self.split, self.height);
            }
            Side::Right => {
                let left_x = i32::try_from(self.split).map_err(|e| e.to_string())?;
                src = Rect::new(left_x, 0, self.width - self.split, self.height);
                dst = Rect::new(
                    self.position.x + left_x,
                    self.position.y,
                    self.width - self.split,
                    self.height,
                );
            }
        };

        // Rect type has to have width greater than 0 so we should better detect
        // case when split is out of texture and do not render corresponding part.
        // Right now we will render 1px width slice in such scenario.
        renderer.copy(texture, src, dst)?;

        Ok(())
    }

    fn reposition(&mut self, position: Point) -> Result<(), String> {
        self.position = position;
        Ok(())
    }
}

struct Diff<'a> {
    left: SplitView<'a>,
    right: SplitView<'a>,
    position: Point,
    split: u32,
}

impl<'a> Diff<'a> {
    fn new(left: Texture<'a>, right: Texture<'a>) -> Diff<'a> {
        let mut left = SplitView::new(left, Side::Left);
        let mut right = SplitView::new(right, Side::Right);
        let position = Point::new(0, 0);
        let split = cmp::min(left.width, right.width) / 2;
        left.split(split);
        right.split(split);
        Diff {
            left,
            right,
            position,
            split,
        }
    }

    fn update_split(&mut self, split: u32) {
        self.split = split;
        self.left.split(self.split);
        self.right.split(self.split);
        debug!("New split position {:?}", self.split);
    }

    fn update(&mut self, e: &sdl2::EventPump) -> Result<(), String> {
        let state = e.mouse_state();
        if state.is_mouse_button_pressed(MouseButton::Left) {
            let max = cmp::max(self.left.width, self.right.width);
            let split = u32::try_from(state.x() - self.position.x())
                .unwrap_or(0)
                .clamp(0, max);
            self.update_split(split);
        }
        Ok(())
    }

    fn split_by_fraction(&mut self, fraction: f64) {
        let split = (fraction.clamp(0.0, 1.0) * f64::from(self.get_size().0)) as u32;
        self.update_split(split);
    }

    fn get_left_fraction(&self) -> f64 {
        f64::from(self.split) / f64::from(self.get_size().0)
    }

    fn get_size(&self) -> (u32, u32) {
        let width = cmp::max(self.left.width, self.right.width);
        let height = cmp::max(self.left.height, self.right.height);
        (width, height)
    }
}

impl<'a> CanvasEntity for Diff<'a> {
    fn draw(&self, renderer: &mut sdl2::render::WindowCanvas) -> Result<(), String> {
        self.left.draw(renderer)?;
        self.right.draw(renderer)?;

        // draw left/right separator
        let split_x = i32::try_from(self.split).map_err(|e| e.to_string())?;
        let height = cmp::max(self.left.height, self.right.height);
        renderer.set_draw_color(Color::RGB(255, 0, 0));
        renderer.fill_rect(Rect::new(
            self.position.x + split_x,
            self.position.y,
            3,
            height,
        ))?;

        Ok(())
    }

    fn reposition(&mut self, position: Point) -> Result<(), String> {
        self.left.reposition(position)?;
        self.right.reposition(position)?;
        self.position = position;
        Ok(())
    }
}

struct CheckerBoard<'a> {
    texture: Texture<'a>,
    width: u32,
    height: u32,
    position: Point,
}

impl<'a> CheckerBoard<'a> {
    const SQUARE_SIZE: u32 = 8;
    const WIDTH: u32 = CheckerBoard::SQUARE_SIZE * 64;
    const HEIGHT: u32 = CheckerBoard::WIDTH;

    fn new(
        texture_creator: &'a TextureCreator<WindowContext>,
        size: (u32, u32),
    ) -> Result<CheckerBoard<'a>, String> {
        // Create pixel data for the checkerboard pattern
        let mut pixels =
            Vec::with_capacity((CheckerBoard::WIDTH * CheckerBoard::HEIGHT) as usize * 4);

        for y in 0..CheckerBoard::HEIGHT {
            for x in 0..CheckerBoard::WIDTH {
                let color =
                    if (x / CheckerBoard::SQUARE_SIZE + y / CheckerBoard::SQUARE_SIZE) % 2 == 0 {
                        Color::RGB(189, 189, 189)
                    } else {
                        Color::RGB(209, 209, 209)
                    };

                let (r, g, b, a) = color.rgba();
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
                pixels.push(a);
            }
        }

        // Create a Surface and load pixel data
        let surface = sdl2::surface::Surface::from_data(
            &mut pixels,
            CheckerBoard::WIDTH,
            CheckerBoard::HEIGHT,
            4 * CheckerBoard::WIDTH,
            sdl2::pixels::PixelFormatEnum::ARGB8888,
        )?;

        // Convert the Surface to a Texture
        let texture = texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| e.to_string())?;
        let position = Point::new(0, 0);

        Ok(CheckerBoard {
            texture,
            width: size.0,
            height: size.1,
            position,
        })
    }

    fn set_size(&mut self, size: (u32, u32)) -> &mut CheckerBoard<'a> {
        self.width = size.0;
        self.height = size.1;
        self
    }
}

impl<'a> CanvasEntity for CheckerBoard<'a> {
    fn draw(&self, renderer: &mut sdl2::render::WindowCanvas) -> Result<(), String> {
        let mut remaining_x: u32 = self.width;
        while remaining_x > 0 {
            let w = cmp::min(remaining_x, CheckerBoard::WIDTH);

            let mut remaining_y: u32 = self.height;
            while remaining_y > 0 {
                let h = cmp::min(remaining_y, CheckerBoard::HEIGHT);

                let dst = Rect::new(
                    self.position.x + (self.width - remaining_x) as i32,
                    self.position.y + (self.height - remaining_y) as i32,
                    w,
                    h,
                );
                let src = Rect::new(0, 0, dst.width(), dst.height());

                renderer.copy(&self.texture, src, dst)?;

                remaining_y -= h;
            }

            remaining_x -= w;
        }

        Ok(())
    }

    fn reposition(&mut self, position: Point) -> Result<(), String> {
        self.position = position;
        Ok(())
    }
}

fn get_texture_builder<'a, P: AsRef<Path>>(
    path: P,
    backend: SvgBackend,
) -> Result<Box<dyn SvgTextureBuilder<'a>>, String> {
    let builder: Box<dyn SvgTextureBuilder> = match backend {
        #[cfg(feature = "use-rsvg")]
        SvgBackend::RsvgWithCairo => Box::new(RsvgWithCairo::new(path)?),
        #[cfg(feature = "use-usvg")]
        SvgBackend::UsvgWithSkia => Box::new(UsvgWithSkia::new(path)?),
    };
    Ok(builder)
}

fn get_max_window_size(video_subsystem: &VideoSubsystem) -> Result<Rect, String> {
    let bounds = {
        let video_displays = video_subsystem.num_video_displays()?;
        let mut max = Rect::new(0, 0, 0, 0);
        for n in 0..video_displays {
            let bounds = video_subsystem.display_usable_bounds(n)?;
            let area = bounds.width() * bounds.height();
            if area > max.width() * max.height() {
                max = bounds;
            }
        }
        max
    };
    debug!("Maximum display usable bounds: {:?}", bounds);
    Ok(bounds)
}

fn center_on_window(rect: &mut Rect, window: &Window) {
    rect.center_on(Point::new(
        (window.size().0 / 2) as i32,
        (window.size().1 / 2) as i32,
    ));
}

pub fn app<P: AsRef<Path>>(
    left: P,
    right: P,
    scale: f64,
    backend: SvgBackend,
    testing: Option<String>,
) -> Result<(), String> {
    let texture_creator: TextureCreator<WindowContext>;
    let left_svg = get_texture_builder(left, backend)?;
    let right_svg = get_texture_builder(right, backend)?;

    let mut scale = scale;
    let left_size = left_svg.query_size(scale)?;
    let right_size = right_svg.query_size(scale)?;

    debug!("Left SVG size {:?}", left_size.size());
    debug!("Right SVG size {:?}", right_size.size());

    let mut workarea_rect: Rect = left_size.union(right_size);

    let sdl_context = sdl2::init()?;

    let video_subsystem = sdl_context.video()?;

    let minimum_size = Rect::new(0, 0, 800, 600);
    let maximum_size = get_max_window_size(&video_subsystem)?;
    let window_width = ((workarea_rect.width() as f64 * 1.1) as u32)
        .clamp(minimum_size.width(), maximum_size.width());
    let window_height = ((workarea_rect.height() as f64 * 1.1) as u32)
        .clamp(minimum_size.height(), maximum_size.height());

    let mut window = video_subsystem
        .window("lukaj", window_width, window_height)
        .position_centered()
        .allow_highdpi()
        .resizable()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    window
        .set_minimum_size(400, 300)
        .map_err(|e| e.to_string())?;

    debug!(
        "Initial window size: {:?}x{:?}",
        window.size().0,
        window.size().1
    );
    debug!("Workarea: {:?}", workarea_rect);

    center_on_window(&mut workarea_rect, &window);

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    texture_creator = canvas.texture_creator();

    debug!("Renderer information: {:?}", canvas.info());
    let max_size = if canvas.info().name == "software" {
        // software renderer does not have size limitation but we enforce it anyway,
        // otherwise rendering might hang the application. This limit should be
        // enough for 99.9% of usecases
        (16384, 16384)
    } else {
        (
            canvas.info().max_texture_width,
            canvas.info().max_texture_height,
        )
    };
    if left_size.size() > max_size || right_size.size() > max_size {
        return Err(format!(
            "ERROR: SVG file exceeds size limit of {:?}px",
            max_size
        ));
    }

    // anything smaller would be impractical to use with diff-slider
    let min_size: (u32, u32) = (100, 100);
    if left_size.size() < min_size || right_size.size() < min_size {
        return Err(String::from(
            "ERROR: SVG file too small, consider using --scale option",
        ));
    }

    let left = left_svg.rasterize(&texture_creator, scale)?;
    let right = right_svg.rasterize(&texture_creator, scale)?;

    let mut diff = Diff::new(left, right);

    let mut workarea = CheckerBoard::new(&texture_creator, diff.get_size())?;

    let mut drag_start: Point = Point::new(0, 0);
    let mut drag: Point = Point::new(0, 0);

    let mut event_pump = sdl_context.event_pump()?;
    let mut prev_buttons = HashSet::new();

    'running: loop {
        let frame_start = std::time::Instant::now();

        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode, .. } => match keycode {
                    Some(sdl2::keyboard::Keycode::R) => {
                        center_on_window(&mut workarea_rect, &canvas.window());
                        drag = Point::new(0, 0);
                    }
                    Some(sdl2::keyboard::Keycode::Escape) => break 'running,
                    _ => {}
                },
                Event::Window { .. } => {
                    center_on_window(&mut workarea_rect, &canvas.window());
                }
                Event::MouseWheel { y, .. } => {
                    let new_scale = if y > 0 { scale * 2.0 } else { scale / 2.0 };

                    let left_size = left_svg.query_size(new_scale)?;
                    let right_size = left_svg.query_size(new_scale)?;
                    debug!("New size: {:?}", left_size);

                    if left_size.size() < min_size
                        || left_size.size() > max_size
                        || right_size.size() < min_size
                        || right_size.size() > max_size
                    {
                        // TODO: when GUI status support added, include this message
                        println!(
                            "ERROR: Zooming out of allowed size limit, minimum size {:?}px, maxiumum size {:?}px",
                            min_size,
                            max_size
                        );
                    } else {
                        scale = new_scale;
                        debug!("Scale change: {:?}", scale);

                        // TODO: some caching could be implemented:
                        let left = left_svg.rasterize(&texture_creator, scale)?;
                        let right = right_svg.rasterize(&texture_creator, scale)?;

                        let left_fraction = diff.get_left_fraction();

                        diff = Diff::new(left, right);
                        diff.split_by_fraction(left_fraction);
                        workarea.set_size(diff.get_size());

                        workarea_rect = left_size.union(right_size);
                        center_on_window(&mut workarea_rect, &canvas.window());
                    }
                }
                _ => {}
            }
        }

        // get a mouse state
        let state = event_pump.mouse_state();
        let current_position = Point::new(state.x(), state.y());

        // Create a set of pressed Keys.
        let buttons: HashSet<MouseButton> = state.pressed_mouse_buttons().collect();
        let new_buttons = &buttons - &prev_buttons;

        if new_buttons.contains(&MouseButton::Right) {
            drag_start = current_position - drag;
            debug!("Dragging started: {:?}", drag_start);
        }
        if buttons.contains(&MouseButton::Right) {
            drag = current_position - drag_start;
            debug!("Dragging {:?}", drag);
        }

        let mut workarea_rect_dst = workarea_rect.clone();
        workarea_rect_dst.offset(drag.x(), drag.y());

        workarea.reposition(workarea_rect_dst.top_left())?;
        workarea.draw(&mut canvas)?;

        diff.reposition(workarea_rect_dst.top_left())?;
        diff.update(&event_pump)?;
        diff.draw(&mut canvas)?;

        canvas.present();

        let frame_duration = frame_start.elapsed().as_micros() as u64;
        trace!("Frame duration: {}us", frame_duration);

        prev_buttons = buttons;

        match testing {
            Some(val) => {
                // added slight delay, otherwise it messes up my i3 & tmux setup (terminal
                // scrollback to be more precise) when window popup and is immediately closed
                ::std::thread::sleep(std::time::Duration::new(1, 0));

                let screenshot_name =
                    env::var("TEST_OUTPUT_FILENAME").unwrap_or(String::from("screenshot.bmp"));

                let window = canvas.window();
                let window_rectangle = Rect::new(0, 0, window.size().0, window.size().1);
                let pixel_format = window.window_pixel_format();
                let mut pixels = canvas.read_pixels(window_rectangle, pixel_format)?;

                let screen = sdl2::surface::Surface::from_data(
                    &mut pixels,
                    window_rectangle.width(),
                    window_rectangle.height(),
                    4 * window_rectangle.width(),
                    pixel_format,
                )?;
                return screen.save_bmp(format!("{}/{}", val, screenshot_name));
            }
            _ => {}
        }

        ::std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 30));
    }

    Ok(())
}

use std::sync::Arc;

use glutin::dpi::PhysicalSize;
use log::trace;
use skia_safe::{colors, dash_path_effect, BlendMode, Canvas, Color, Paint, Rect, HSV};

use super::{CachingShaper, RendererSettings};
use crate::editor::{Colors, Style};
use crate::settings::*;
use crate::utils::Dimensions;

pub struct GridRenderer {
    pub shaper: CachingShaper,
    pub paint: Paint,
    pub default_style: Arc<Style>,
    pub font_dimensions: Dimensions,
    pub scale_factor: f64,
    pub is_ready: bool,
}

impl GridRenderer {
    pub fn new(scale_factor: f64) -> Self {
        let mut shaper = CachingShaper::new(scale_factor as f32);
        let mut paint = Paint::new(colors::WHITE, None);
        paint.set_anti_alias(false);
        let default_style = Arc::new(Style::new(Colors::new(
            Some(colors::WHITE),
            Some(colors::BLACK),
            Some(colors::GREY),
        )));
        let font_dimensions: Dimensions = shaper.font_base_dimensions().into();

        GridRenderer {
            shaper,
            paint,
            default_style,
            font_dimensions,
            scale_factor,
            is_ready: false,
        }
    }

    /// Convert PhysicalSize to grid size
    pub fn convert_physical_to_grid(&self, physical: PhysicalSize<u32>) -> Dimensions {
        Dimensions::from(physical) / self.font_dimensions
    }

    /// Convert grid size to PhysicalSize
    pub fn convert_grid_to_physical(&self, grid: Dimensions) -> PhysicalSize<u32> {
        (grid * self.font_dimensions).into()
    }

    pub fn handle_scale_factor_update(&mut self, scale_factor: f64) {
        self.shaper.update_scale_factor(scale_factor as f32);
        self.update_font_dimensions();
    }

    pub fn update_font(&mut self, guifont_setting: &str) {
        self.shaper.update_font(guifont_setting);
        self.update_font_dimensions();
    }

    fn update_font_dimensions(&mut self) {
        self.font_dimensions = self.shaper.font_base_dimensions().into();
        self.is_ready = true;
        trace!("Updated font dimensions: {:?}", self.font_dimensions,);
    }

    fn compute_text_region(&self, grid_position: (u64, u64), cell_width: u64) -> Rect {
        let (x, y) = grid_position * self.font_dimensions;
        let width = cell_width * self.font_dimensions.width;
        let height = self.font_dimensions.height;
        Rect::new(x as f32, y as f32, (x + width) as f32, (y + height) as f32)
    }

    pub fn get_default_background(&self) -> Color {
        self.default_style.colors.background.unwrap().to_color()
    }

    pub fn draw_background(
        &mut self,
        canvas: &mut Canvas,
        grid_position: (u64, u64),
        cell_width: u64,
        style: &Option<Arc<Style>>,
    ) {
        self.paint.set_blend_mode(BlendMode::Src);

        let region = self.compute_text_region(grid_position, cell_width);
        let style = style.as_ref().unwrap_or(&self.default_style);

        if SETTINGS.get::<RendererSettings>().debug_renderer {
            let random_hsv: HSV = (rand::random::<f32>() * 360.0, 0.3, 0.3).into();
            let random_color = random_hsv.to_color(255);
            self.paint.set_color(random_color);
        } else {
            self.paint
                .set_color(style.background(&self.default_style.colors).to_color());
        }
        canvas.draw_rect(region, &self.paint);
    }

    pub fn draw_foreground(
        &mut self,
        canvas: &mut Canvas,
        cells: &[String],
        grid_position: (u64, u64),
        cell_width: u64,
        style: &Option<Arc<Style>>,
    ) {
        let (x, y) = grid_position * self.font_dimensions;
        let width = cell_width * self.font_dimensions.width;

        let style = style.as_ref().unwrap_or(&self.default_style);

        canvas.save();

        let region = self.compute_text_region(grid_position, cell_width);

        canvas.clip_rect(region, None, Some(false));

        if style.underline || style.undercurl {
            let line_position = self.shaper.underline_position();
            let stroke_width = self.shaper.current_size() / 10.0;

            self.paint
                .set_color(style.special(&self.default_style.colors).to_color());
            self.paint.set_stroke_width(stroke_width);

            if style.undercurl {
                self.paint.set_path_effect(dash_path_effect::new(
                    &[stroke_width * 2.0, stroke_width * 2.0],
                    0.0,
                ));
            } else {
                self.paint.set_path_effect(None);
            }

            canvas.draw_line(
                (
                    x as f32,
                    (y - line_position + self.font_dimensions.height) as f32,
                ),
                (
                    (x + width) as f32,
                    (y - line_position + self.font_dimensions.height) as f32,
                ),
                &self.paint,
            );
        }

        let y_adjustment = self.shaper.y_adjustment();

        if SETTINGS.get::<RendererSettings>().debug_renderer {
            let random_hsv: HSV = (rand::random::<f32>() * 360.0, 1.0, 1.0).into();
            let random_color = random_hsv.to_color(255);
            self.paint.set_color(random_color);
        } else {
            self.paint
                .set_color(style.foreground(&self.default_style.colors).to_color());
        }
        self.paint.set_anti_alias(false);

        for blob in self
            .shaper
            .shape_cached(cells, style.bold, style.italic)
            .iter()
        {
            canvas.draw_text_blob(blob, (x as f32, (y + y_adjustment) as f32), &self.paint);
        }

        if style.strikethrough {
            let line_position = region.center_y();
            self.paint
                .set_color(style.special(&self.default_style.colors).to_color());
            canvas.draw_line(
                (x as f32, line_position),
                ((x + width) as f32, line_position),
                &self.paint,
            );
        }

        canvas.restore();
    }
}

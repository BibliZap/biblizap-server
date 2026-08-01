use std::{ops::Deref, rc::Rc};

use crate::usage::data::{BinSize, UsageBinIterator, UsageData, UsageDataPoint};
use scales::{ChartScale, CountScale, DateScale};
use wasm_bindgen::JsCast;
use yew::prelude::*;

mod scales;

#[derive(Properties, PartialEq, Clone)]
pub struct UsageProps {
    pub data: UsageData,
}

struct Insets {
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
}

impl Insets {
    fn plot_height_in_canvas_units(&self, canvas_height: f64) -> f64 {
        canvas_height - self.top - self.bottom
    }

    fn plot_width_in_canvas_units(&self, canvas_width: f64) -> f64 {
        canvas_width - self.left - self.right
    }
}

struct DrawInfo {
    ctx: web_sys::CanvasRenderingContext2d,
    canvas_width: f64,
    canvas_height: f64,
}

struct ChartGeometry {
    canvas_width: f64,
    canvas_height: f64,
    insets: Insets,
}

impl ChartGeometry {
    fn new(canvas_width: f64, canvas_height: f64, insets: Insets) -> Self {
        Self {
            canvas_width,
            canvas_height,
            insets,
        }
    }

    fn plot_bottom(&self) -> f64 {
        self.canvas_height - self.insets.bottom
    }

    fn chart_x_to_canvas_x(&self, chart_x: f64, chart_width: f64) -> f64 {
        let plot_width = self.insets.plot_width_in_canvas_units(self.canvas_width);
        let pixels_per_chart_unit = plot_width / chart_width;

        self.insets.left + (chart_x * pixels_per_chart_unit)
    }

    fn chart_y_to_canvas_y(&self, chart_y: f64, chart_height: f64) -> f64 {
        let plot_height = self.insets.plot_height_in_canvas_units(self.canvas_height);
        let pixels_per_chart_unit = plot_height / chart_height;

        self.plot_bottom() - (chart_y * pixels_per_chart_unit)
    }

    fn chart_position_to_canvas(
        &self,
        chart_x: f64,
        chart_y: f64,
        chart_width: f64,
        chart_height: f64,
    ) -> (f64, f64) {
        let canvas_x = self.chart_x_to_canvas_x(chart_x, chart_width);
        let canvas_y = self.chart_y_to_canvas_y(chart_y, chart_height);
        (canvas_x, canvas_y)
    }
}

struct ChartRenderer<'a> {
    ctx: &'a web_sys::CanvasRenderingContext2d,
    geometry: ChartGeometry,
    scale: &'a ChartScale,
}

impl<'a> ChartRenderer<'a> {
    fn new(
        ctx: &'a web_sys::CanvasRenderingContext2d,
        geometry: ChartGeometry,
        scale: &'a ChartScale,
    ) -> Self {
        Self {
            ctx,
            geometry,
            scale,
        }
    }

    fn chart_x_to_canvas_x(&self, chart_x: f64) -> f64 {
        self.geometry
            .chart_x_to_canvas_x(chart_x, self.scale.x.chart_width_in_chart_units())
    }

    fn chart_y_to_canvas_y(&self, chart_y: f64) -> f64 {
        self.geometry
            .chart_y_to_canvas_y(chart_y, self.scale.y.chart_height_in_chart_units())
    }

    fn chart_position_to_canvas(&self, chart_x: f64, chart_y: f64) -> (f64, f64) {
        self.geometry.chart_position_to_canvas(
            chart_x,
            chart_y,
            self.scale.x.chart_width_in_chart_units(),
            self.scale.y.chart_height_in_chart_units(),
        )
    }
}

fn prepare_canvas(canvas: web_sys::HtmlCanvasElement) -> DrawInfo {
    let ctx: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")
        .expect("Failed to get 2D context")
        .expect("Context is None")
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .expect("Failed to cast to CanvasRenderingContext2d");

    let window = web_sys::window().expect("no global `window` exists");
    let dpr = window.device_pixel_ratio();

    let css_width = canvas.client_width();
    let css_height = canvas.client_height();

    canvas.set_width((css_width as f64 * dpr) as u32);
    canvas.set_height((css_height as f64 * dpr) as u32);

    ctx.scale(dpr, dpr).expect("Failed to scale context");

    DrawInfo {
        ctx,
        canvas_width: css_width as f64,
        canvas_height: css_height as f64,
    }
}

fn usage_bin_to_chart_position(point: &UsageDataPoint, chart_scale: &ChartScale) -> (f64, f64) {
    let start_x = chart_scale.x.date_to_chart_x(point.start_date);
    let end_x = chart_scale.x.date_to_chart_x(point.end_date);
    let x = (start_x + end_x) / 2.0;
    let y = chart_scale.y.count_to_chart_y(point.count);

    (x, y)
}

fn format_date_tick_label(date: time::Date) -> String {
    format!(
        "{:02}-{:02}-{}",
        date.day(),
        u8::from(date.month()),
        date.year()
    )
}

fn draw_chart(
    draw_info: &DrawInfo,
    data_points: &[UsageDataPoint],
    chart_scale: &ChartScale,
    insets: Insets,
) {
    let DrawInfo {
        ctx,
        canvas_width,
        canvas_height,
    } = draw_info;
    let renderer = ChartRenderer::new(
        ctx,
        ChartGeometry::new(*canvas_width, *canvas_height, insets),
        chart_scale,
    );

    // Clear the canvas
    renderer.ctx.clear_rect(
        0.0,
        0.0,
        renderer.geometry.canvas_width,
        renderer.geometry.canvas_height,
    );

    // Draw axes
    renderer.ctx.begin_path();
    renderer
        .ctx
        .move_to(renderer.geometry.insets.left, renderer.geometry.insets.top);
    renderer.ctx.line_to(
        renderer.geometry.insets.left,
        renderer.geometry.plot_bottom(),
    );
    renderer.ctx.line_to(
        renderer.geometry.canvas_width - renderer.geometry.insets.right,
        renderer.geometry.plot_bottom(),
    );
    renderer.ctx.stroke();

    // Draw y axis ticks and labels
    for (y, count_label) in renderer.scale.y.make_ticks_and_labels() {
        let y = renderer.chart_y_to_canvas_y(y);
        renderer.ctx.begin_path();
        renderer.ctx.move_to(renderer.geometry.insets.left - 5.0, y);
        renderer.ctx.line_to(renderer.geometry.insets.left, y);
        renderer.ctx.stroke();

        renderer
            .ctx
            .fill_text(
                &count_label.to_string(),
                renderer.geometry.insets.left - 30.0,
                y + 5.0,
            )
            .expect("Failed to draw text");
    }

    // Draw x axis ticks and labels
    let plot_bottom = renderer.geometry.plot_bottom();
    renderer.ctx.set_text_align("center");
    renderer.ctx.set_text_baseline("top");
    for date in renderer.scale.x.make_ticks() {
        let x = renderer.chart_x_to_canvas_x(renderer.scale.x.date_to_chart_x(date));

        renderer.ctx.begin_path();
        renderer.ctx.move_to(x, plot_bottom);
        renderer.ctx.line_to(x, plot_bottom + 5.0);
        renderer.ctx.stroke();

        renderer
            .ctx
            .fill_text(&format_date_tick_label(date), x, plot_bottom + 8.0)
            .expect("Failed to draw text");
    }

    // Draw data points (for simplicity, just draw circles for each point)
    for point in data_points {
        let (x, y) = usage_bin_to_chart_position(point, renderer.scale);
        gloo_console::log!(format!(
            "Drawing point for date: {}, count: {}, in_chart_x: {}, in_chart_y: {}",
            point.start_date, point.count, x, y
        ));
        let (x, y) = renderer.chart_position_to_canvas(x, y);

        renderer.ctx.begin_path();
        renderer
            .ctx
            .arc(x, y, 5.0, 0.0, std::f64::consts::PI * 2.0)
            .expect("Failed to create arc");
        renderer.ctx.fill();
    }
}

#[function_component]
pub fn UsageChart(UsageProps { data }: &UsageProps) -> Html {
    let data = Rc::new(data.clone());

    let start_date = use_state(|| {
        data.get_first_date()
            .unwrap_or(time::OffsetDateTime::now_utc().date())
    });

    let end_date = use_state(|| time::OffsetDateTime::now_utc().date() + time::Duration::days(1));

    let bin_size = use_state(|| BinSize::Daily);

    let chart_data = use_memo(
        (data, start_date.clone(), end_date.clone(), bin_size),
        |(data, start_date, end_date, bin_size)| {
            let iterator = UsageBinIterator::new(
                data,
                *start_date.deref(),
                *bin_size.deref(),
                *end_date.deref(),
            );
            iterator.collect::<Vec<UsageDataPoint>>()
        },
    );

    let chart_scale = use_memo(
        (start_date.clone(), end_date.clone(), chart_data.clone()),
        |(start_date, end_date, chart_data)| {
            let min_count = chart_data
                .iter()
                .map(|point| point.count)
                .min()
                .unwrap_or(0);
            let max_count = chart_data
                .iter()
                .map(|point| point.count)
                .max()
                .unwrap_or(1);

            ChartScale::new(
                DateScale::new(*start_date.deref(), *end_date.deref()),
                CountScale::new(min_count, max_count, 10),
            )
        },
    );

    let canvas_ref = use_node_ref();

    {
        let canvas_ref = canvas_ref.clone();
        use_effect_with((chart_data.clone(), chart_scale.clone()), move |_| {
            let canvas = canvas_ref
                .cast::<web_sys::HtmlCanvasElement>()
                .expect("canvas_ref not attached to a canvas element");
            let draw_info = prepare_canvas(canvas);

            let insets = Insets {
                top: 20.0,
                right: 20.0,
                bottom: 30.0,
                left: 40.0,
            };

            draw_chart(&draw_info, &chart_data.deref(), &chart_scale, insets);
        })
    }

    html! {
        <div class="usage-chart-container">
            <h2>{ "Usage Data" }</h2>
            // <ul>
            //     { for iterator.map(|UsageDataPoint { start_date, end_date, count }| {
            //         html! {
            //             <li>{ format!("Time Bucket: {} to {}, Total Requests: {}", start_date, end_date, total_requests) }</li>
            //         }
            //     }) }
            // </ul>
            <canvas ref={canvas_ref}></canvas>
        </div>
    }
}

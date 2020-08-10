use druid::Selector;
use std::time::Duration;
use druid::TimerToken;
use std::sync::Arc;
use druid::LifeCycle;
use druid::LifeCycleCtx;
use druid::PaintCtx;
use druid::Size;
use druid::{Color, Point, Rect};
use druid::Event;
use druid::EventCtx;
use druid::UpdateCtx;
use druid::LayoutCtx;
use druid::BoxConstraints;
use druid::Env;
use druid::{AppLauncher, WindowDesc, Widget, PlatformError, WidgetExt};
use druid::{Data, Lens, LensExt};
use druid::widget::{Flex,Label,Spinner,TextBox,Slider,Button};
use druid::piet::{ImageFormat,InterpolationMode};
use druid::widget::prelude::*;

use rand::prelude::*;
use rand_pcg::Pcg64;

// TODO: it will ev
const FIXED_WIDTH : usize = 500;
const FIXED_HEIGHT : usize = 500;
const SEED_LIMIT : u64 = 1_000_000_000_000;

#[derive(Debug, Clone, Data, Lens)]
struct GenData {

    running: bool,

    // parameters to apply to next sketch restart
    mode: GenMode,
    seed: u64,
    startdensity: f64,

    width: usize,
    height: usize,

    sketch: GenSketch,
}

impl GenData {
    pub fn new() -> Self {
        let startdensity = 0.5;
        let mode = GenMode::Annealing;
        let seed = Self::make_seed();

        GenData {
            mode,
            width: FIXED_WIDTH,
            height: FIXED_HEIGHT,
            seed: seed,
            startdensity,
            sketch: GenSketch::new(FIXED_WIDTH, FIXED_HEIGHT, mode, seed, startdensity),
            running: false,
        }
    }

    pub fn resize(&mut self, sz: &Size) {
        self.width = sz.width as usize;
        self.height = sz.height as usize;
        self.restart();
    }

    pub fn restart(&mut self) {
        self.sketch = GenSketch::new(self.width, self.height, self.mode, self.seed, self.startdensity);
    }

    fn make_seed() -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen::<u64>() % SEED_LIMIT

    }
    pub fn random_seed(&mut self) {
        self.seed = Self::make_seed();
    }
}

#[derive(Debug, Clone, Data, PartialEq, Copy)]
enum GenMode {
    Majority,
    Annealing,
}

#[derive(Debug, Clone, Data, PartialEq, Lens)]
struct GenSketch {
    iter: u64,
    mode: GenMode,
    cells: Arc<Vec<bool>>,
    width: usize,
    height: usize,

}

impl GenSketch {
    pub fn new(width: usize, height: usize, mode: GenMode, seed: u64, startdensity: f64) -> Self {
        let mut rng = Pcg64::seed_from_u64(seed);

        let mut v = Vec::with_capacity(width*height);
        for _ in 0..v.capacity() {
            v.push( rng.gen::<f64>() < startdensity)
        }

        GenSketch {
            iter: 0,
            mode,
            cells: Arc::new(v),
            width,
            height,
        }
    }

    pub fn step(&mut self) {
        self.iter += 1;
        let mut v = vec![false; self.cells.len()];

        let w = self.width;
        let h = self.height;

        let c = &self.cells;
        for y in 1..h-1 {
            let start = y * w;
            for x in 1..w-1 {
                let total = 0
                    + c[x+start-w-1] as u8 
                    + c[x+start-w] as u8
                    + c[x+start-w+1] as u8
                    + c[x+start-1] as u8 
                    + c[x+start] as u8
                    + c[x+start+1] as u8
                    + c[x+start+w-1] as u8 
                    + c[x+start+w] as u8
                    + c[x+start+w+1] as u8;

                v[x+start] = match self.mode {
                    GenMode::Majority => {
                        total > 4
                    }
                    GenMode::Annealing => {
                        total == 4 || total > 5
                    }
                }
            }
        }
        self.cells = Arc::new(v);
    }

    pub fn get_image_buffer(&self) -> Vec<u8> {
        let mut im = Vec::with_capacity(self.cells.len() * 4);
        for &v in self.cells.iter() {
                let pix = if v { 0 } else { 0xff };
                im.push(pix);
                im.push(pix);
                im.push(pix);
                im.push(0xff); // alpha
        }
        im
    }
}

struct GenWidget {
    timer_id: TimerToken,
}

impl GenWidget {
    fn new() -> Self {
        GenWidget {
            timer_id: TimerToken::INVALID,
        }
    }

    const RESIZE: Selector<Size> = Selector::new("resize");
}

impl Widget<GenData> for GenWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GenData, _env: &Env) {
        match event {
            Event::WindowConnected => {
                ctx.request_paint();
                let deadline = Duration::from_millis(10 as u64);
                self.timer_id = ctx.request_timer(deadline);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    if data.running {
                        data.sketch.step();
                        ctx.request_paint();
                    }
                    let deadline = Duration::from_millis(10 as u64);
                    self.timer_id = ctx.request_timer(deadline);
                }
            }
            Event::MouseDown(_e) => {
                data.running = true;
            }
            Event::MouseUp(_e) => {
                data.running = false;
            }
            Event::Command(c) => {
                if let Some(size) = c.get(GenWidget::RESIZE) {
                    data.resize(size);
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &GenData, _data: &GenData, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &GenData,
        _env: &Env,
    ) -> Size {
        // BoxConstraints are passed by the parent widget.
        // This method can return any Size within those constraints:
        // bc.constrain(my_size)
        //
        // To check if a dimension is infinite or not (e.g. scrolling):
        // bc.is_width_bounded() / bc.is_height_bounded()
        bc.max()
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &GenData,
        _env: &Env,
    ) {
        match event {
            LifeCycle::Size(size) => {
                ctx.submit_command(druid::Command::new(GenWidget::RESIZE, *size), None);
            }
            _ => {}
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GenData, _env: &Env) {
        // Let's draw a picture with Piet!

        // Clear the whole widget with the color of your choice
        // (ctx.size() returns the size of the layout rect we're painting in)
        let size = ctx.size();
        let rect = Rect::from_origin_size(Point::ORIGIN, size);
        ctx.fill(rect, &Color::WHITE);

        // Let's burn some CPU to make a (partially transparent) image buffer
        let image = ctx
            .make_image(data.sketch.width, data.sketch.height, 
                &data.sketch.get_image_buffer(), ImageFormat::RgbaSeparate)
            .unwrap();
        // The image is automatically scaled to fit the rect you pass to draw_image
        ctx.draw_image(
            &image,
            Rect::from_origin_size(Point::ORIGIN, size),
            InterpolationMode::Bilinear,
        );
    }

}


fn build_ui() -> impl Widget<GenData> {
    let lens_u64 = druid::lens::Map::new(
        |x : &u64| x.to_string(), 
        |x: &mut u64, val| { 
            // ignore invalid number entries
            if let Ok(v) = val.parse::<u64>() {
                *x = v;
            }
        });


    Flex::row()
        .with_flex_child(
            GenWidget::new(),
            0.9,
            )
        .with_spacer(12.0)
        .with_child(
            Flex::column()
            .with_child(
                Flex::row()
                .with_child(
                    TextBox::new()
                    .lens(GenData::seed.then(lens_u64))
                    )
                .with_child(
                    Button::new("dice")
                    // todo: something with command instead?
                    .on_click(|_ctx, data: &mut GenData, _env| {
                        data.random_seed();
                        data.restart();
                        })
                    )
                )
            .with_child(
                Slider::new()
                .with_range(0.0, 1.0)
                .lens(GenData::startdensity.then(druid::lens::Id))
                )
            .with_child(
                Label::dynamic(|iter: &u64, _: &Env| format!("iter {}", iter))
                .lens(GenData::sketch.then(GenSketch::iter))
                )
            .with_child(
                Button::new("New")
                // TODO: something with Command instead?
                .on_click(|_ctx, data: &mut GenData, _env| data.restart())
                )
            )
}

fn main() -> Result<(), PlatformError> {
    let gend = GenData::new();
    AppLauncher::with_window(WindowDesc::new(build_ui)).launch(gend)?;
    Ok(())
}

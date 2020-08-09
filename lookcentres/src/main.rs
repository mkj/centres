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
use druid::{Data, Lens};
use druid::widget::{Flex,Label};
use druid::piet::{ImageFormat,InterpolationMode};
use druid::widget::prelude::*;

use rand::prelude::*;
use rand_pcg::Pcg64;


#[derive(Debug, Clone, Data, Lens)]
struct GenData {
    now: u64,

    mode: GenMode,

    state: GenState,

    // TODO: update speed?
    // iteration redraw interval?
    // stop iteration?
}

// TODO: it will ev
const FIXED_WIDTH : usize = 500;
const FIXED_HEIGHT : usize = 500;

impl GenData {
    pub fn new() -> Self {
        GenData {
            now: 0,
            mode: GenMode::Annealing,
            state: GenState::new(FIXED_WIDTH, FIXED_HEIGHT, 123, 0.5),
        }
    }

    pub fn step(&mut self) {
        self.now += 1;
        let mut v = vec![false; self.state.cells.len()];

        let w = self.state.width;
        let h = self.state.height;

        let c = &self.state.cells;
        for y in 1..h-1 {
            let start = y * w;
            let prevstart = start - w;
            let nextstart = start + w;
            for x in 1..w-1 {
                let total = 0
                    + c[x+prevstart-1] as u8 
                    + c[x+prevstart] as u8
                    + c[x+prevstart+1] as u8
                    + c[x+start-1] as u8 
                    + c[x+start] as u8
                    + c[x+start+1] as u8
                    + c[x+nextstart-1] as u8 
                    + c[x+nextstart] as u8
                    + c[x+nextstart+1] as u8;
                if x < 5 && y < 5 {
                    println!("x {} y {} tot {}", x, y, total);
                }

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
        self.state.cells = Arc::new(v);
    }

}

#[derive(Debug, Clone, Data, PartialEq)]
enum GenMode {
    Majority,
    Annealing,
}

#[derive(Debug, Clone, Data, PartialEq)]
struct GenState {
    cells: Arc<Vec<bool>>,
    width: usize,
    height: usize,
    seed: u64,
    startdensity: f32,
}

impl GenState {
    pub fn new(width: usize, height: usize, seed: u64, startdensity: f32) -> Self {
        let mut rng = Pcg64::seed_from_u64(seed);

        let mut v = Vec::with_capacity(width*height);
        for i in 0..v.capacity() {
            v.push( rng.gen::<f32>() < startdensity)
        }

        GenState {
            cells: Arc::new(v),
            width,
            height,
            seed,
            startdensity,
        }
    }

    pub fn get_image_buffer(&self) -> Vec<u8> {
        let mut im = Vec::with_capacity(self.width*self.height*4);
        let mut i = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                let pix = if self.cells[i] { 0 } else { 0xff };
                i += 1;
                im.push(pix);
                im.push(pix);
                im.push(pix);
                im.push(0xff); // alpha
            }
        }
        im
    }
}

struct GenWidget {

}

impl Widget<GenData> for GenWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GenData, _env: &Env) {
        match event {
            /*
            Event::WindowConnected => {
                ctx.request_paint();
                let deadline = Duration::from_millis(data.iter_interval() as u64);
                self.timer_id = ctx.request_timer(deadline);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    if !data.paused {
                        data.grid.evolve();
                        ctx.request_paint();
                    }
                    let deadline = Duration::from_millis(data.iter_interval() as u64);
                    self.timer_id = ctx.request_timer(deadline);
                }
            }
            */
            Event::MouseDown(e) => {
                data.step();
                ctx.request_paint();
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
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &GenData,
        _env: &Env,
    ) {
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
            .make_image(data.state.width, data.state.height, 
                &data.state.get_image_buffer(), ImageFormat::RgbaSeparate)
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
    Flex::row()
        .with_flex_child(
            GenWidget {},
            0.9,
            )
        .with_spacer(12.0)
        .with_child(
            Label::new("it's centres"),
            )
}

fn main() -> Result<(), PlatformError> {
    let gend = GenData::new();
    AppLauncher::with_window(WindowDesc::new(build_ui)).launch(gend)?;
    Ok(())
}

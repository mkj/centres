use druid::Selector;
use std::time::{Duration,Instant};
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
use druid::widget::{Flex,Label,Stepper,RadioGroup,TextBox,Slider,Button};
use druid::piet::{ImageFormat,InterpolationMode};
use druid::widget::prelude::*;

use rand::prelude::*;
use rand_pcg::Pcg64;

use rayon::prelude::*;

// TODO: it will ev
const FIXED_WIDTH : usize = 500;
const FIXED_HEIGHT : usize = 500;
const SEED_LIMIT : u64 = 1_000_000_000_000;
const INIT_JUMPTO : u64 = 120;

#[derive(Debug, Clone, Data, Lens)]
struct GenData {

    running: bool,

    // parameters to apply to next sketch restart
    mode: GenMode,
    seed: u64,
    startdensity: f64,

    jumpto: u64,

    width: usize,
    height: usize,

    sketch: GenSketch,
}

impl GenData {
    pub fn new() -> Self {
        let startdensity = 0.5;
        let mode = GenMode::Experiment;
        let seed = Self::make_seed();

        GenData {
            mode,
            width: FIXED_WIDTH,
            height: FIXED_HEIGHT,
            seed: seed,
            startdensity,
            sketch: GenSketch::new(FIXED_WIDTH, FIXED_HEIGHT, mode, seed, startdensity),
            running: false,
            jumpto: INIT_JUMPTO,
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
    Star1,
    TwoBonus,
    Experiment,
}

#[derive(Debug, Clone, Data, PartialEq, Lens)]
struct GenSketch {
    iter: u64,
    mode: GenMode,
    cells: Arc<Vec<i8>>,
    width: usize,
    height: usize,
    randompair: Arc<Vec<u32>>,

    // time to draw so far. XXX be nice if Duration implemented Data
    elapsed: f64,
}

impl GenSketch {
    pub fn new(width: usize, height: usize, mode: GenMode, seed: u64, startdensity: f64) -> Self {
        let mut rng = Pcg64::seed_from_u64(seed);

        let mut v = Vec::with_capacity(width*height);
        for _ in 0..v.capacity() {
            v.push( (rng.gen::<f64>() < startdensity).into() );
        }

        assert!(width*height <= u32::MAX as usize);
        let mut p = Vec::with_capacity(width*height);
        for i in 0..v.capacity() {
            p.push(i as u32)
        }
        p.shuffle(&mut rng);

        GenSketch {
            iter: 0,
            mode,
            cells: Arc::new(v),
            width,
            height,
            randompair: Arc::new(p),
            elapsed: 0.0,
        }
    }

    pub fn skip(&mut self, until: u64) {
        while self.iter < until {
            self.step()
        }
    }

    pub fn step(&mut self) {
        self.iter += 1;

        let starttime = Instant::now();

        let w = self.width;
        let h = self.height;

        let mode = self.mode;
        let mut v = Vec::with_capacity(self.cells.len());
        // top blank row
        v.extend((0..w).map(|_| 0 as i8));
        v.par_extend(
            (1..h-1).into_par_iter().map(|y| {
                let c = self.cells.clone();
                let start = y*w;
                // main bit
                (0..w).into_par_iter().map(move |x| {
                    if x == 0 || x == w-1 {
                        // blank left/right
                        0 
                    } else {
                        let total = //i8::min(9, i8::max(0, 
                            0
                            + c[x+start-w-1]
                            + c[x+start-w]
                            + c[x+start-w+1]
                            + c[x+start-1]
                            + c[x+start]
                            + c[x+start+1]
                            + c[x+start+w-1]
                            + c[x+start+w]
                            + c[x+start+w+1]
                            ;
                        match mode {
                            GenMode::Majority => {
                                (total > 4).into()
                            }
                            GenMode::Annealing => {
                                (total == 4 || total > 5).into()
                            }
                            GenMode::Star1 | GenMode::TwoBonus => {
                                let bonus = match mode {
                                    GenMode::Star1 => {1}
                                    GenMode::TwoBonus => {2}
                                    _ => panic!("wrong mode")
                                };
                                let r = total == 4 || total > 5;
                                r as i8
                                    + if r && c[x+start] < 1 { bonus }
                                    else if !r && c[x+start] >= 1 { -bonus }
                                    else { 0 }
                                //new[x+start] = res && !c[x+start];
                                //res
                            }
                            GenMode::Experiment => {
                                let bonus = 3;
                                let r = total == 4 || total > 5;
                                let adj = r as i8
                                    + if total > 5 {
                                        if r && c[x+start] < 0 { bonus }
                                        else if !r && c[x+start] > 0 { -bonus }
                                        else { 0 }
                                    } else {
                                        0
                                    };
                                adj
                                //new[x+start] = res && !c[x+start];
                                //res
                            }
                        }
                    }
                })
            })
            .flatten()
        );
        // bottom blank row
        v.extend((0..w).map(|_| 0 as i8));

        self.cells = Arc::new(v);
        self.elapsed += starttime.elapsed().as_secs_f64();
    }

    pub fn get_image_buffer(&self) -> Vec<u8> {
        let mut im = Vec::with_capacity(self.cells.len() * 4);
        for &v in self.cells.iter() {
                let pix = if v != 0 { 0xff } else { 0 };
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

    fn start_timer(&mut self, ctx: &mut EventCtx) {
        let deadline = Duration::from_millis(5 as u64);
        self.timer_id = ctx.request_timer(deadline);
    }

    const RESIZE: Selector<Size> = Selector::new("resize");
}

impl Widget<GenData> for GenWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GenData, _env: &Env) {
        match event {
            Event::WindowConnected => {
                self.start_timer(ctx);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    if data.running {
                        data.sketch.step();
                    }
                    self.start_timer(ctx);
                }
            }
            Event::MouseDown(_e) => {
                self.start_timer(ctx);
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

    let lens_f64 = druid::lens::Map::new(
        |x : &f64| format!("{:.5}", x),
        |x: &mut f64, val| { 
            // ignore invalid number entries
            if let Ok(v) = val.parse::<f64>() {
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
                RadioGroup::new(vec![
                    ("Majority", GenMode::Majority),
                    ("Annealing", GenMode::Annealing),
                    ("Star", GenMode::Star1),
                    ("TwoBonus", GenMode::TwoBonus),
                    ("Experiment", GenMode::Experiment),
                ])
                .lens(GenData::mode)
                )
            .with_child(
                Flex::row()
                .with_child(
                    Slider::new()
                    .with_range(0., 0.56)
                    .lens(GenData::startdensity.then(druid::lens::Id))
                    )
                .with_child(
                    TextBox::new()
                    .lens(GenData::startdensity.then(lens_f64))
                    )
                )
            .with_child(
                Label::dynamic(|iter: &u64, _: &Env| format!("iter {}", iter))
                .lens(GenData::sketch.then(GenSketch::iter))
                )
            /*
            .with_child(
                Label::dynamic(|sketch: &GenSketch, _: &Env| format!(
                    "{} ms", sketch.elapsed / (sketch.iter as f64) * 0.001))
                )
                */
            .with_child(
                Flex::row()
                .with_child(
                    Label::dynamic(|v: &u64, _: &Env| format!("{}", v))
                    .lens(GenData::jumpto)
                    )
                .with_child(
                    Stepper::new()
                    .with_range(0.0, 1000.0)
                    .with_step(10.0)
                    .lens(GenData::jumpto.map(|&u| u as f64, |u, f| *u = f as u64))
                    )
                .with_child(
                    Button::new("New")
                    // TODO: something with Command instead?
                    .on_click(|_ctx, data: &mut GenData, _env| {
                        data.random_seed();
                        data.restart();
                        data.sketch.skip(data.jumpto);
                    })
                    )
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
    AppLauncher::with_window(
        WindowDesc::new(build_ui)
        .title("centres")
        ).launch(gend)?;
    Ok(())
}

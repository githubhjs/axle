use crate::desktop::DesktopElement;
use crate::println;
use crate::utils::get_timestamp;
use crate::window::Window;
use agx_definitions::{Point, Rect, Size};
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use num_traits::One;

fn lerp(a: f64, b: f64, percent: f64) -> f64 {
    a + (percent * (b - a))
}

fn interpolate_window_frame(from: Rect, to: Rect, percent: f64) -> Rect {
    /*
    // Don't let the window get too small
    // TODO(PT): Pull this out into a MIN_WINDOW_SIZE?
    let to = Size::new(
        isize::max(to.size.width, 1),
        isize::max(to.size.height, (Window::TITLE_BAR_HEIGHT as isize) + 1)
    );
    */
    Rect::from_parts(
        Point::new(
            lerp(from.min_x() as f64, to.min_x() as f64, percent) as isize,
            lerp(from.min_y() as f64, to.min_y() as f64, percent) as isize,
        ),
        Size::new(
            lerp(from.width() as f64, to.width() as f64, percent) as isize,
            lerp(from.height() as f64, to.height() as f64, percent) as isize,
        ),
    )
}

/*
}
*/

pub struct WindowOpenAnimationParams {
    start_time: usize,
    end_time: usize,
    pub window: Rc<Window>,
    pub duration_ms: usize,
    pub frame_from: Rect,
    pub frame_to: Rect,
}

impl WindowOpenAnimationParams {
    pub fn new(
        desktop_size: Size,
        window: &Rc<Window>,
        duration_ms: usize,
        frame_to: Rect,
    ) -> Self {
        let from_size = Size::new(desktop_size.width / 10, desktop_size.height / 10);
        let frame_from = Rect::from_parts(
            Point::new(
                ((desktop_size.width as f64 / 2.0) - (from_size.width as f64 / 2.0)) as isize,
                desktop_size.height - from_size.height,
            ),
            from_size,
        );
        let start_time = get_timestamp() as usize;
        Self {
            start_time,
            end_time: start_time + duration_ms,
            window: Rc::clone(window),
            duration_ms,
            frame_from,
            frame_to,
        }
    }
}

pub struct AnimationDamage {
    pub area_to_recompute_drawable_regions: Rect,
    pub rects_needing_composite: Vec<Rect>,
}

impl AnimationDamage {
    pub fn new(
        area_to_recompute_drawable_regions: Rect,
        rects_needing_composite: Vec<Rect>,
    ) -> Self {
        Self {
            area_to_recompute_drawable_regions,
            rects_needing_composite,
        }
    }
}

pub enum Animation {
    WindowOpen(WindowOpenAnimationParams),
    //WindowClose(Rc<Window>),
}

impl Animation {
    pub fn start(&self) {
        match self {
            Animation::WindowOpen(params) => {
                *params.window.frame.borrow_mut() = params.frame_from;
            }
        }
    }

    pub fn step(&self, now: u64) -> AnimationDamage {
        match self {
            Animation::WindowOpen(params) => {
                let update_region = {
                    let mut window_frame = params.window.frame.borrow_mut();
                    let old_frame = *window_frame;
                    let elapsed = now - (params.start_time as u64);
                    let percent = f64::min(1.0, elapsed as f64 / params.duration_ms as f64);
                    println!(
                        "Window({}) open animation step {percent:.2}%",
                        params.window.name()
                    );
                    let new_frame =
                        interpolate_window_frame(params.frame_from, params.frame_to, percent);
                    *window_frame = new_frame;
                    old_frame.union(new_frame)
                };
                params.window.redraw_title_bar();
                AnimationDamage::new(update_region, vec![update_region])
            }
        }
    }

    pub fn is_complete(&self, now: u64) -> bool {
        match self {
            Animation::WindowOpen(params) => now as usize >= params.end_time,
        }
    }
}
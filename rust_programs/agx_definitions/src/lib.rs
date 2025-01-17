#![cfg_attr(any(target_os = "axle", feature = "no_std"), no_std)]
#![feature(core_intrinsics)]
#![feature(slice_ptr_get)]
#![feature(default_alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(rustc_private)]

extern crate alloc;
extern crate core;
#[cfg(any(target_os = "axle", feature = "no_std"))]
extern crate libc;

use alloc::fmt::Debug;

use alloc::boxed::Box;
use alloc::rc::Weak;
use alloc::vec;
use core::fmt::Formatter;
use core::{
    cmp::{max, min},
    fmt::Display,
    ops::{Add, Mul, Sub},
};
use itertools::Itertools;
use num_traits::Float;

#[cfg(any(target_os = "axle", feature = "no_std"))]
use alloc::vec::Vec;

#[cfg(any(target_os = "axle", feature = "no_std"))]
use axle_rt::println;
use bresenham::{Bresenham, BresenhamInclusive};
#[cfg(not(any(target_os = "axle", feature = "no_std")))]
use std::println;

pub mod font;
pub mod layer;
pub use font::*;
pub use layer::*;

pub trait NestedLayerSlice: Drawable {
    fn get_parent(&self) -> Option<Weak<dyn NestedLayerSlice>>;
    fn set_parent(&self, parent: Weak<dyn NestedLayerSlice>);

    fn get_content_slice_frame(&self) -> Rect {
        let parent = self.get_parent().unwrap().upgrade().unwrap();
        let content_frame = parent.content_frame();
        let constrained_to_content_frame = content_frame.constrain(self.frame());

        /*
        parent_slice.get_slice(Rect::from_parts(
            content_frame.origin + self.frame().origin,
            constrained_to_content_frame.size,
        ))
        */

        let mut origin = content_frame.origin + self.frame().origin;
        /*
        origin.x = max(origin.x, 0);
        origin.y = max(origin.y, 0);
        */
        let mut size = constrained_to_content_frame.size;
        if origin.x < 0 {
            let overhang = -origin.x;
            size.width -= overhang;
            origin.x = 0;
        }
        if origin.y < 0 {
            let overhang = -origin.y;
            size.height -= overhang;
            origin.y = 0;
        }
        Rect::from_parts(origin, size)
    }

    fn get_slice(&self) -> Box<dyn LikeLayerSlice> {
        let parent = self.get_parent().unwrap().upgrade().unwrap();
        let parent_slice = parent.get_slice();
        let content_slice_frame = self.get_content_slice_frame();
        parent_slice.get_slice(content_slice_frame)
    }

    fn get_slice_for_render(&self) -> Box<dyn LikeLayerSlice>;
}

pub trait Drawable {
    fn frame(&self) -> Rect;

    fn content_frame(&self) -> Rect;

    /// Returns the rects damaged while drawing
    fn draw(&self) -> Vec<Rect>;
}

#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<[u8; 3]> for Color {
    fn from(vals: [u8; 3]) -> Self {
        Color::new(vals[0], vals[1], vals[2])
    }
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }
    pub fn black() -> Self {
        Color::from([0, 0, 0])
    }
    pub fn white() -> Self {
        Color::from([255, 255, 255])
    }
    pub fn gray() -> Self {
        Color::from([127, 127, 127])
    }
    pub fn dark_gray() -> Self {
        Color::from([80, 80, 80])
    }
    pub fn light_gray() -> Self {
        Color::from([120, 120, 120])
    }
    pub fn red() -> Self {
        Color::from([255, 0, 0])
    }
    pub fn green() -> Self {
        Color::from([0, 255, 0])
    }
    pub fn blue() -> Self {
        Color::from([0, 0, 255])
    }
    pub fn yellow() -> Self {
        Color::from([0, 234, 255])
    }

    /// Swag RGB to BGR and vice versa
    pub fn swap_order(&self) -> Self {
        Color::new(self.b, self.g, self.r)
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Color({}, {}, {})", self.r, self.g, self.b)
    }
}

// For FFI
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SizeU32 {
    pub width: u32,
    pub height: u32,
}

impl SizeU32 {
    pub fn new(width: u32, height: u32) -> Self {
        SizeU32 { width, height }
    }

    pub fn from(size: Size) -> Self {
        SizeU32 {
            width: size.width as u32,
            height: size.height as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Ord, PartialOrd, Eq)]
pub struct Size {
    pub width: isize,
    pub height: isize,
}

impl Size {
    pub fn new(width: isize, height: isize) -> Self {
        Size { width, height }
    }

    pub fn zero() -> Self {
        Size {
            width: 0,
            height: 0,
        }
    }

    pub fn area(&self) -> isize {
        self.width * self.height
    }

    pub fn mid_x(&self) -> isize {
        self.width / 2
    }

    pub fn mid_y(&self) -> isize {
        self.height / 2
    }
}

impl Size {
    pub fn from(size: &SizeU32) -> Self {
        Size {
            width: size.width.try_into().unwrap(),
            height: size.height.try_into().unwrap(),
        }
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.width, self.height)
    }
}

impl Add for Size {
    type Output = Size;
    fn add(self, rhs: Self) -> Self::Output {
        Size {
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}

impl Sub for Size {
    type Output = Size;
    fn sub(self, rhs: Self) -> Self::Output {
        Size {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct SizeF64 {
    pub width: f64,
    pub height: f64,
}

impl SizeF64 {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

impl From<SizeF64> for Size {
    fn from(value: SizeF64) -> Self {
        Size::new(value.width.round() as _, value.height.round() as _)
    }
}

// For FFI
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PointU32 {
    pub x: u32,
    pub y: u32,
}

impl PointU32 {
    pub fn new(x: u32, y: u32) -> Self {
        PointU32 { x, y }
    }

    pub fn from(point: Point) -> Self {
        PointU32 {
            x: point.x as u32,
            y: point.y as u32,
        }
    }
}

impl Display for SizeF64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "({:.02}, {:.02})", self.width, self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Ord, PartialOrd, Eq)]
pub struct Point {
    pub x: isize,
    pub y: isize,
}

impl Point {
    pub fn new(x: isize, y: isize) -> Self {
        Point { x, y }
    }
    pub fn zero() -> Self {
        Point { x: 0, y: 0 }
    }
    pub fn distance(&self, p2: Point) -> f64 {
        let p1 = self;
        let x_dist = p2.x - p1.x;
        let y_dist = p2.y - p1.y;
        let hypotenuse_squared = x_dist.pow(2) + y_dist.pow(2);
        (hypotenuse_squared as f64).sqrt()
    }

    pub fn cross(&self, other: &PointF64) -> f64 {
        self.x as f64 * other.y - self.y as f64 * other.x
    }

    pub fn div(&self, divisor: f64) -> PointF64 {
        PointF64::new(self.x as f64 / divisor, self.y as f64 / divisor)
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<Point> for Point {
    type Output = Point;
    fn mul(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl Mul<isize> for Point {
    type Output = Point;
    fn mul(self, rhs: isize) -> Self::Output {
        Point {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl From<PointF64> for Point {
    fn from(value: PointF64) -> Self {
        Point::new(value.x.round() as _, value.y.round() as _)
    }
}

impl From<PointU32> for Point {
    fn from(value: PointU32) -> Self {
        Point::new(value.x.try_into().unwrap(), value.y.try_into().unwrap())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct PointF64 {
    pub x: f64,
    pub y: f64,
}

impl PointF64 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn cross(&self, other: &PointF64) -> f64 {
        self.x * other.y - self.y * other.x
    }

    pub fn div(&self, divisor: f64) -> PointF64 {
        PointF64::new(self.x / divisor, self.y / divisor)
    }
}

impl From<Point> for PointF64 {
    fn from(value: Point) -> Self {
        PointF64::new(value.x as _, value.y as _)
    }
}

impl Sub for PointF64 {
    type Output = PointF64;
    fn sub(self, rhs: Self) -> Self::Output {
        PointF64::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Add for PointF64 {
    type Output = PointF64;
    fn add(self, rhs: Self) -> Self::Output {
        PointF64::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Display for PointF64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "({:6.02}, {:6.02})", self.x, self.y)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RectInsets {
    pub left: isize,
    pub top: isize,
    pub right: isize,
    pub bottom: isize,
}

impl RectInsets {
    pub fn new(left: isize, top: isize, right: isize, bottom: isize) -> Self {
        RectInsets {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn zero() -> Self {
        RectInsets {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    pub fn uniform(inset: isize) -> Self {
        RectInsets {
            left: inset,
            top: inset,
            right: inset,
            bottom: inset,
        }
    }
}

impl Add for RectInsets {
    type Output = RectInsets;

    fn add(self, rhs: RectInsets) -> Self::Output {
        // TODO(PT): The side order here should match the constructor passed to inset_by()...
        RectInsets::new(
            self.left + rhs.left,
            self.top + rhs.top,
            self.right + rhs.right,
            self.bottom + rhs.bottom,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Ord, PartialOrd, Eq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(x: isize, y: isize, width: isize, height: isize) -> Self {
        Rect {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn with_size(size: Size) -> Self {
        Self {
            origin: Point::zero(),
            size,
        }
    }

    pub fn with_origin(origin: Point) -> Self {
        Self {
            origin,
            size: Size::zero(),
        }
    }

    pub fn replace_origin(&self, new_origin: Point) -> Self {
        Self::from_parts(new_origin, self.size)
    }

    pub fn replace_size(&self, new_size: Size) -> Self {
        Self::from_parts(self.origin, new_size)
    }

    pub fn add_origin(&self, addend: Point) -> Self {
        Self::from_parts(self.origin + addend, self.size)
    }

    pub fn from_parts(origin: Point, size: Size) -> Self {
        Rect { origin, size }
    }

    pub fn zero() -> Self {
        Rect::from_parts(Point::zero(), Size::zero())
    }

    pub fn inset_by(&self, bottom: isize, left: isize, right: isize, top: isize) -> Self {
        Rect::from_parts(
            self.origin + Point::new(left, top),
            self.size - Size::new(left + right, top + bottom),
        )
    }

    pub fn inset_by_insets(&self, insets: RectInsets) -> Self {
        self.inset_by(insets.bottom, insets.left, insets.right, insets.top)
    }

    pub fn min_x(&self) -> isize {
        self.origin.x
    }

    pub fn min_y(&self) -> isize {
        self.origin.y
    }

    pub fn max_x(&self) -> isize {
        self.min_x() + self.size.width
    }

    pub fn max_y(&self) -> isize {
        self.min_y() + self.size.height
    }

    pub fn mid_x(&self) -> isize {
        self.min_x() + ((self.size.width as f64 / 2f64) as isize)
    }

    pub fn mid_y(&self) -> isize {
        self.min_y() + ((self.size.height as f64 / 2f64) as isize)
    }

    pub fn midpoint(&self) -> Point {
        Point::new(self.mid_x(), self.mid_y())
    }

    pub fn width(&self) -> isize {
        self.size.width
    }

    pub fn height(&self) -> isize {
        self.size.height
    }

    pub fn center(&self) -> Point {
        Point::new(self.mid_x(), self.mid_y())
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min_x() && p.y >= self.min_y() && p.x < self.max_x() && p.y < self.max_y()
    }

    pub fn encloses(&self, rhs: Self) -> bool {
        rhs.min_x() >= self.min_x()
            && rhs.min_y() >= self.min_y()
            && rhs.max_x() <= self.max_x()
            && rhs.max_y() <= self.max_y()
    }

    pub fn constrain(&self, rhs: Self) -> Self {
        if rhs.min_x() >= self.max_x() || rhs.min_y() >= self.max_y() {
            return Rect::zero();
        }

        let mut width = rhs.width();
        if rhs.max_x() > self.width() {
            width -= rhs.max_x() - self.width();
        }

        let mut height = rhs.height();
        if rhs.max_y() > self.height() {
            height -= rhs.max_y() - self.height();
        }

        let origin = rhs.origin;
        /*
        if rhs.min_x() < self.min_x() {
            origin.x = 0;
        }
        if rhs.min_y() < self.min_y() {
            origin.y = 0;
        }
        */
        /*
        if rhs.min_x() < self.min_x() {
            width -= (self.min_x() - rhs.min_x()) * 2;
            origin.x = self.min_x();
        }
        if rhs.min_y() < self.min_y() {
            height -= (self.min_y() - rhs.min_y()) * 2;
            origin.y = self.min_y();
        }
        */

        Rect::from_parts(origin, Size::new(width, height))
    }

    pub fn apply_insets(&self, insets: RectInsets) -> Self {
        Rect::new(
            self.origin.x + insets.left,
            self.origin.y + insets.top,
            self.size.width - (insets.left + insets.right),
            self.size.height - (insets.top + insets.bottom),
        )
    }

    pub fn intersects_with(&self, other: Rect) -> bool {
        self.max_x() > other.min_x()
            && self.min_x() < other.max_x()
            && self.max_y() > other.min_y()
            && self.min_y() < other.max_y()
    }

    pub fn area_excluding_rect(&self, exclude_rect: Rect) -> Vec<Self> {
        let mut trimmed_area = *self;
        //println!("{trimmed_area}.exclude({exclude_rect})");
        let mut out = Vec::new();

        if !trimmed_area.intersects_with(exclude_rect) {
            //println!("no intersection, not doing anything");
            return out;
        }

        // Exclude the left edge, resulting in an excluded left area
        let left_overlap = exclude_rect.min_x() - trimmed_area.min_x();
        if left_overlap > 0 {
            //println!("left edge {trimmed_area} overlap {left_overlap}");
            out.push(Rect::from_parts(
                trimmed_area.origin,
                Size::new(left_overlap, trimmed_area.height()),
            ));
            trimmed_area.origin.x += left_overlap;
            trimmed_area.size.width -= left_overlap;
        }

        if !trimmed_area.intersects_with(exclude_rect) {
            return out;
        }

        // Exclude the right edge
        let right_overlap = trimmed_area.max_x() - exclude_rect.max_x();
        if right_overlap > 0 {
            //println!("right edge {trimmed_area} overlap {right_overlap}");
            out.push(Rect::from_parts(
                Point::new(exclude_rect.max_x(), trimmed_area.min_y()),
                Size::new(right_overlap, trimmed_area.height()),
            ));
            trimmed_area.size.width -= right_overlap;
        }

        if !trimmed_area.intersects_with(exclude_rect) {
            return out;
        }

        // Exclude the top, resulting in an excluded bottom area
        //println!("top edge {trimmed_area}");
        let top_overlap = trimmed_area.max_y() - exclude_rect.max_y();
        if top_overlap > 0 {
            //println!("top edge {trimmed_area} overlap {top_overlap}");
            let top_rect = Rect::from_parts(
                //Point::new(trimmed_area.min_x(), trimmed_area.min_y() + top_overlap),
                Point::new(trimmed_area.min_x(), exclude_rect.max_y()),
                //Size::new(trimmed_area.width(), trimmed_area.height() - top_overlap),
                Size::new(trimmed_area.width(), top_overlap),
            );
            //println!("\tGot top rect {top_rect}");
            out.push(top_rect);
            //trimmed_area.origin.y += top_overlap;
            trimmed_area.size.height -= top_overlap;
        }

        if !trimmed_area.intersects_with(exclude_rect) {
            return out;
        }

        // Exclude the bottom, resulting in an included top area
        let bottom_overlap = exclude_rect.min_y() - trimmed_area.min_y();
        //println!("bottom overlap {bottom_overlap}, rect {trimmed_area}");
        if bottom_overlap > 0 {
            //println!("bottom edge {trimmed_area} overlap {bottom_overlap}");
            out.push(Rect::from_parts(
                trimmed_area.origin,
                Size::new(trimmed_area.width(), bottom_overlap),
            ));
            trimmed_area.size.height -= bottom_overlap;
        }

        out
    }

    pub fn area_overlapping_with(&self, rect_to_intersect_with: Rect) -> Option<Self> {
        if !self.intersects_with(rect_to_intersect_with) {
            return None;
        }
        let r1 = *self;
        let r2 = rect_to_intersect_with;

        // Handle when the rectangles are identical, otherwise our assertion below will trigger
        if r1 == r2 {
            return Some(r1);
        }

        let origin = Point::new(max(r1.min_x(), r2.min_x()), max(r1.min_y(), r2.min_y()));
        let bottom_right = Point::new(min(r1.max_x(), r2.max_x()), min(r1.max_y(), r2.max_y()));

        /*
        println!(
            "area_overlapping_with {r1} {r2}, intersects? {:?} intersects2 {:?}",
            r1.intersects_with(r2),
            self.intersects_with(rect_to_intersect_with),
        );
        */
        if !(origin.x < bottom_right.x && origin.y < bottom_right.y) {
            //println!("Rects didn't intersect even though we checked above: {r1} {r2}");
            return None;
        }

        /*
        assert!(
            origin.x < bottom_right.x && origin.y < bottom_right.y,
            "Rects didn't intersect even though we checked above"
        );
        */
        let size = Size::new(bottom_right.x - origin.x, bottom_right.y - origin.y);
        Some(Rect::from_parts(origin, size))
    }

    pub fn union(&self, other: Rect) -> Rect {
        let origin = Point::new(
            min(self.min_x(), other.min_x()),
            min(self.min_y(), other.min_y()),
        );
        Rect::from_parts(
            origin,
            Size::new(
                max(self.max_x(), other.max_x()) - origin.x,
                max(self.max_y(), other.max_y()) - origin.y,
            ),
        )
    }

    pub fn translate_point(&self, p: Point) -> Point {
        p - self.origin
    }

    pub fn is_zero(&self) -> bool {
        *self == Rect::zero()
    }

    pub fn is_degenerate(&self) -> bool {
        self.width() == 0 || self.height() == 0
    }

    pub fn area(&self) -> isize {
        self.width() * self.height()
    }
}

impl Display for Rect {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "(({}, {}), ({}, {}))",
            self.min_x(),
            self.min_y(),
            self.width(),
            self.height()
        )
    }
}

impl From<RectU32> for Rect {
    fn from(rect: RectU32) -> Self {
        Self {
            origin: Point::from(rect.origin),
            size: Size::from(&rect.size),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct RectF64 {
    pub origin: PointF64,
    pub size: SizeF64,
}

impl RectF64 {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: PointF64::new(x, y),
            size: SizeF64::new(width, height),
        }
    }

    pub fn from_parts(origin: PointF64, size: SizeF64) -> Self {
        Self { origin, size }
    }

    pub fn zero() -> Self {
        Self {
            origin: PointF64::zero(),
            size: SizeF64::zero(),
        }
    }

    pub fn min_x(&self) -> f64 {
        self.origin.x
    }

    pub fn min_y(&self) -> f64 {
        self.origin.y
    }

    pub fn max_x(&self) -> f64 {
        self.origin.x + self.size.width
    }

    pub fn max_y(&self) -> f64 {
        self.origin.y + self.size.height
    }

    pub fn union(&self, other: Self) -> Self {
        let origin = PointF64::new(
            self.min_x().min(other.min_x()),
            self.min_y().min(other.min_y()),
        );
        let size = SizeF64::new(
            self.max_x().max(other.max_x()),
            self.max_y().max(other.max_y()),
        );
        Self::from_parts(origin, size)
    }
}

impl Display for RectF64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.origin, self.size)
    }
}

impl From<RectF64> for Rect {
    fn from(rect: RectF64) -> Self {
        Self {
            origin: Point::from(rect.origin),
            size: rect.size.into(),
        }
    }
}

#[derive(PartialEq)]
struct TileSegment<'a> {
    viewport_frame: Rect,
    tile_frame: Rect,
    tile: &'a Tile,
}

impl<'a> TileSegment<'a> {
    fn new(viewport_frame: Rect, tile_frame: Rect, tile: &'a Tile) -> Self {
        Self {
            viewport_frame,
            tile_frame,
            tile,
        }
    }
}

impl<'a> Debug for TileSegment<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "TileSegment({} / {} within tile {:?})",
            self.viewport_frame, self.tile_frame, self.tile
        )
    }
}

#[derive(Debug, PartialEq)]
struct TileSegments<'a>(Vec<TileSegment<'a>>);

#[derive(PartialEq)]
struct Tile {
    frame: Rect,
}

impl Tile {
    fn new(frame: Rect) -> Self {
        Self { frame }
    }
    fn tiles_visible_in_viewport(tiles: &Vec<Tile>, viewport_rect: Rect) -> TileSegments {
        TileSegments(
            tiles
                .iter()
                .filter_map(|tile| {
                    /*
                    println!(
                        "\tChecking for intersection with {viewport_rect} and {}",
                        tile.frame
                    );
                    */
                    if let Some(intersection) = viewport_rect.area_overlapping_with(tile.frame) {
                        //println!("\t\t area overlapping {intersection}");
                        let tile_viewport_origin = intersection.origin - viewport_rect.origin;
                        Some(TileSegment::new(
                            Rect::from_parts(tile_viewport_origin, intersection.size),
                            Rect::from_parts(
                                intersection.origin - tile.frame.origin,
                                intersection.size,
                            ),
                            tile,
                        ))
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }
}

impl Debug for Tile {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Tile({})", self.frame)
    }
}

#[cfg(test)]
mod test {
    use crate::println;
    use alloc::vec;
    use alloc::vec::Vec;

    use crate::{Line, Point, PointF64, Rect, Tile, TileSegment, TileSegments};

    #[test]
    fn test_tiles_visible_in_layer() {
        let tiles = vec![
            Tile::new(Rect::new(0, 0, 300, 300)),
            Tile::new(Rect::new(0, 300, 300, 300)),
        ];
        assert_eq!(
            Tile::tiles_visible_in_viewport(&tiles, Rect::new(0, 0, 300, 300)),
            TileSegments(vec![TileSegment::new(
                Rect::new(0, 0, 300, 300),
                Rect::new(0, 0, 300, 300),
                &tiles[0],
            )])
        );
        assert_eq!(
            Tile::tiles_visible_in_viewport(&tiles, Rect::new(0, 10, 300, 300)),
            TileSegments(vec![
                TileSegment::new(
                    Rect::new(0, 0, 300, 290),
                    Rect::new(0, 10, 300, 290),
                    &tiles[0],
                ),
                TileSegment::new(
                    Rect::new(0, 290, 300, 10),
                    Rect::new(0, 0, 300, 10),
                    &tiles[1],
                ),
            ])
        );
    }

    fn test_intersects_with() {
        assert!(!Rect::new(0, 0, 300, 300).intersects_with(Rect::new(0, 300, 300, 300)));
        assert!(!Rect::new(0, 0, 300, 300).intersects_with(Rect::new(0, 300, 300, 300)));
        assert!(Rect::new(0, 0, 300, 300).intersects_with(Rect::new(0, 0, 300, 300)));
    }

    fn test_intersection_with_flipped_ordering(
        r1: Rect,
        r2: Rect,
        expected_intersection: Option<Rect>,
    ) {
        assert_eq!(r1.area_overlapping_with(r2), expected_intersection);
        assert_eq!(r2.area_overlapping_with(r1), expected_intersection);
    }

    #[test]
    fn test_find_intersection() {
        /*
        *----*---------*
        |    |    .    |
        |    |    .    |
        *----*---------*
        */
        test_intersection_with_flipped_ordering(
            Rect::new(0, 0, 100, 100),
            Rect::new(50, 0, 100, 100),
            Some(Rect::new(50, 0, 50, 100)),
        );

        /*
        *----------*
        |          |
        *----------*
        |          |
        | . . . .  |
        |          |
        *----------*
        */
        test_intersection_with_flipped_ordering(
            Rect::new(0, 0, 300, 300),
            Rect::new(0, 150, 300, 300),
            Some(Rect::new(0, 150, 300, 150)),
        );
    }

    #[test]
    fn test_rect_diff() {
        let main = Rect::new(0, 150, 300, 300);
        let exclude = Rect::new(0, 0, 300, 300);
        /*
        ------------------------------
        |                            |
        |                            |
        |                            |
        |                            |
        ------------------------------
        |                            |
        |                            |
        |                            |
        |  -   -   -   -   -   -   - |
        |                            |
        |                            |
        |                            |
        |                            |
        ------------------------------
        */
        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![Rect::new(0, 300, 300, 150)]
        );

        let main = Rect::new(0, 100, 400, 50);
        let exclude = Rect::new(50, 0, 300, 300);
        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![
                // Left edge
                Rect::new(0, 100, 50, 50),
                // Right edge
                Rect::new(350, 100, 50, 50),
            ]
        );

        let main = Rect::new(0, 100, 400, 50);
        let exclude = Rect::new(0, 0, 300, 300);
        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![
                // Right edge
                Rect::new(300, 100, 100, 50)
            ]
        );

        let main = Rect::new(300, 200, 300, 100);
        let exclude = Rect::new(400, 100, 100, 150);
        /*
                           -----------
                           |         |
                     ------|---------|------
                     |     |---------|     |
                     |                     |
                     -----------------------
        */

        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![
                // Left portion
                Rect::new(300, 200, 100, 100),
                // Right portion
                Rect::new(500, 200, 100, 100),
                // Botton portion
                Rect::new(400, 250, 100, 50),
            ]
        );

        let exclude = Rect::new(50, 0, 100, 200);
        let main = Rect::new(0, 50, 200, 50);
        /*
             ----------
             |        |
             |        |
             |        |
             |        |
        --------------------
        |    |        |    |
        |    |        |    |
        |    |        |    |
        --------------------
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             ----------
        */
        for a in main.area_excluding_rect(exclude) {
            //println!("{a}");
        }
        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![
                // Left portion
                Rect::new(0, 50, 50, 50),
                // Right portion
                Rect::new(150, 50, 50, 50),
            ]
        );

        // Same rectangle as above with the regions flipped
        let main = Rect::new(50, 0, 100, 200);
        let exclude = Rect::new(0, 50, 200, 50);
        /*
             ----------
             | (main) |
             |        |
             |        |
             |        |
        --------------------
        |     (exclude)    |
        |                  |
        |                  |
        --------------------
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             |        |
             ----------
        */
        for a in main.area_excluding_rect(exclude) {
            //println!("{a}");
        }
        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![
                // Bottom portion
                Rect::new(50, 100, 100, 100),
                // Top portion
                Rect::new(50, 0, 100, 50),
            ]
        );

        let main = Rect::new(0, 0, 200, 200);
        let exclude = Rect::new(50, 50, 100, 100);
        /*
        ----------------------------
        |       *   (main) *       |
        |       ------------       |
        |       | (exclude)|       |
        |       |          |       |
        |       ------------       |
        |       *          *       |
        ----------------------------
        */
        for a in main.area_excluding_rect(exclude) {
            //println!("{a}");
        }
        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![
                // Left portion
                Rect::new(0, 0, 50, 200),
                // Right portion
                Rect::new(150, 0, 50, 200),
                // Bottom portion
                Rect::new(50, 150, 100, 50),
                // Top portion
                Rect::new(50, 0, 100, 50),
            ]
        );

        /*
        ----------
        | (main) |
        |        |
        |        |
        |    ----------
        |    | (excl) |
        |    |        |
        |    |        |
        |    |        |
        |    |        |
        |    |        |
        |    |        |
        -----|        |
             |        |
             |        |
             |        |
             ----------
        */
        let main = Rect::new(200, 200, 100, 130);
        let exclude = Rect::new(250, 250, 100, 130);
        /*
        for a in main.area_excluding_rect(exclude) {
            println!("{a}");
        }
        */
        assert_eq!(
            main.area_excluding_rect(exclude),
            vec![Rect::new(200, 200, 50, 130), Rect::new(250, 200, 50, 50),],
        );

        /*
        ------------------------------
        |                            |
        |                            |
        |              ------------------------------|
        |              |                             |
        ---------------|                             |
                       |                             |
                       |                             |
        |              ------------------------------|
        */
    }

    #[test]
    fn test_line_intersection() {
        let l1 = Line::new(Point::new(20, 0), Point::new(20, 20));
        let l2 = Line::new(Point::new(0, 0), Point::new(40, 10));
        assert_eq!(l1.intersection(&l2), Some(PointF64::new(20.0, 5.0)));
        //        let a = Line {
        //             start: (1.0, 0.0).into(),
        //             end: (1.0, 1.0).into(),
        //         };
        //         let b = Line {
        //             start: (0.0, 0.0).into(),
        //             end: (2.0, 0.5).into(),
        //         };
        //         let s1 = LineInterval::line_segment(a);
        //         let s2 = LineInterval::line_segment(b);
        //         let relation = LineRelation::DivergentIntersecting((1.0, 0.25).into());
    }

    #[test]
    fn test_inset_by() {
        let r = Rect::new(0, 0, 100, 100);
        let inset = r.inset_by(0, 0, 0, 0);
        assert_eq!(r, inset);

        let inset = r.inset_by(10, 10, 10, 10);
        assert_eq!(inset, Rect::new(10, 10, 80, 80));

        let inset = r.inset_by(10, 10, 40, 10);
        assert_eq!(inset, Rect::new(10, 10, 50, 80));
    }
}

/// For FFI
#[derive(Debug, Clone, Copy)]
pub struct RectU32 {
    pub origin: PointU32,
    pub size: SizeU32,
}

impl RectU32 {
    pub fn from(rect: Rect) -> Self {
        Self {
            origin: PointU32::from(rect.origin),
            size: SizeU32::from(rect.size),
        }
    }

    pub fn zero() -> Self {
        Self::from(Rect::zero())
    }
}

/*
impl Add for Rect {
    type Output = Rect;
    fn add(self, rhs: Self) -> Self::Output {
        Rect::from_parts(self.origin + rhs.origin, self.size + rhs.size)
    }
}
*/

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub p1: Point,
    pub p2: Point,
}

impl Line {
    pub fn new(p1: Point, p2: Point) -> Self {
        Line { p1, p2 }
    }

    pub fn max_x(&self) -> isize {
        max(self.p1.x, self.p2.x)
    }

    pub fn min_x(&self) -> isize {
        min(self.p1.x, self.p2.x)
    }

    pub fn max_y(&self) -> isize {
        max(self.p1.y, self.p2.y)
    }

    pub fn min_y(&self) -> isize {
        min(self.p1.y, self.p2.y)
    }

    fn draw_strip(&self, onto: &mut Box<dyn LikeLayerSlice>, color: Color) {
        // Relative distances in both directions
        let mut delta_x = self.p2.x - self.p1.x;
        let mut delta_y = self.p2.y - self.p1.y;

        // Increment of 0 would imply either vertical or horizontal line
        let inc_x = match delta_x {
            _ if delta_x > 0 => 1,
            _ if delta_x == 0 => 0,
            _ => -1,
        };
        let inc_y = match delta_y {
            _ if delta_y > 0 => 1,
            _ if delta_y == 0 => 0,
            _ => -1,
        };

        //let distance = max(delta_x.abs(), delta_y.abs());
        delta_x = delta_x.abs();
        delta_y = delta_y.abs();
        let distance = max(delta_x, delta_y);

        let mut cursor = self.p1;
        let mut x_err = 0;
        let mut y_err = 0;
        for _ in 0..distance + 1 {
            onto.putpixel(cursor, color);

            x_err += delta_x;
            y_err += delta_y;

            if x_err > distance {
                x_err -= distance;
                cursor.x += inc_x;
            }
            if y_err > distance {
                y_err -= distance;
                cursor.y += inc_y;
            }
        }
    }

    pub fn draw(
        &self,
        onto: &mut Box<dyn LikeLayerSlice>,
        color: Color,
        thickness: StrokeThickness,
    ) {
        if let StrokeThickness::Width(thickness) = thickness {
            // Special casing for straight lines
            // Horizontal line?
            if self.p1.x == self.p2.x {
                for i in 0..thickness {
                    let mut subline = self.clone();
                    subline.p1.x += i;
                    subline.p2.x += i;
                    subline.draw_strip(onto, color);
                }
            }
            // Vertical line?
            else if self.p1.y == self.p2.y {
                for i in 0..thickness {
                    let mut subline = self.clone();
                    subline.p1.y += i;
                    subline.p2.y += i;
                    subline.draw_strip(onto, color);
                }
            } else {
                let off = (thickness / 2) as isize;
                for i in 0..thickness {
                    let mut subline = self.clone();
                    subline.p1.x += off - i;
                    subline.p2.x += off - i;
                    // PT: This would be more intuitive behavior, but I've disabled it to keep
                    // compatibility with the view-border-inset drawing code.
                    //subline.p1.y += off - i;
                    //subline.p2.y += off - i;
                    subline.draw_strip(onto, color);
                }
            }
        } else {
            self.draw_strip(onto, color);
        }
    }

    pub fn intersection(self, other: &Self) -> Option<PointF64> {
        // Ref: https://stackoverflow.com/questions/563198
        let p = self.p1;
        let q = other.p1;
        let r = PointF64::from(self.p2 - self.p1);
        let s = PointF64::from(other.p2 - other.p1);

        //println!("p, q, r, s {p}, {q}, {r:?}, {s:?}");

        let r_cross_s = r.cross(&s);
        //println!("r cross s {r_cross_s}");

        let q_minus_p = q - p;
        let q_minus_p_cross_r = q_minus_p.cross(&r);
        //println!("q_minus_p {q_minus_p}, q_minus_p_cross_r {q_minus_p_cross_r}");

        // Parallel/collinear lines?
        if r_cross_s == 0.0 {
            if q_minus_p_cross_r == 0.0 {
                // Collinear
            } else {
                // Parallel
            }
            return None;
        }

        // Non-parallel/non-collinear lines
        let t = q_minus_p.cross(&s.div(r_cross_s));
        let u = q_minus_p.cross(&r.div(r_cross_s));
        //println!("t {t}, u {u}");

        // are the intersection coordinates both in range?
        let t_in_range = 0.0 <= t && t <= 1.0;
        let u_in_range = 0.0 <= u && u <= 1.0;

        if !t_in_range || !u_in_range {
            // No intersection
            return None;
        }

        // Intersection
        //println!("p {p} r {r:?} t {t}");
        Some(PointF64::new(
            (p.x as f64 + t * r.x as f64),
            (p.y as f64 + t * r.y as f64),
        ))
    }

    pub fn intersects_with(&self, other: &Line) -> bool {
        // TODO(PT): Speed this up
        self.intersection(other).is_some()
    }
}

impl Add<Point> for Line {
    type Output = Line;

    fn add(self, rhs: Point) -> Self::Output {
        Line::new(self.p1 + rhs, self.p2 + rhs)
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{} - {}]", self.p1, self.p2)
    }
}

impl From<LineF64> for Line {
    fn from(value: LineF64) -> Self {
        Line::new(value.p1.into(), value.p2.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineF64 {
    pub p1: PointF64,
    pub p2: PointF64,
}

impl LineF64 {
    pub fn new(p1: PointF64, p2: PointF64) -> Self {
        Self { p1, p2 }
    }

    pub fn min_x(&self) -> f64 {
        self.p1.x.min(self.p2.x)
    }

    pub fn min_y(&self) -> f64 {
        self.p1.y.min(self.p2.y)
    }

    pub fn max_x(&self) -> f64 {
        self.p1.x.max(self.p2.x)
    }

    pub fn max_y(&self) -> f64 {
        self.p1.y.max(self.p2.y)
    }

    pub fn intersection(self, other: &Self) -> Option<PointF64> {
        // Ref: https://stackoverflow.com/questions/563198
        let p = self.p1;
        let q = other.p1;
        let r = self.p2 - self.p1;
        let s = other.p2 - other.p1;

        let r_cross_s = r.cross(&s);

        let q_minus_p = q - p;
        let q_minus_p_cross_r = q_minus_p.cross(&r);

        // Parallel/collinear lines?
        if r_cross_s == 0.0 {
            if q_minus_p_cross_r == 0.0 {
                // Collinear
            } else {
                // Parallel
            }
            return None;
        }

        // Non-parallel/non-collinear lines
        let t = q_minus_p.cross(&s.div(r_cross_s));
        let u = q_minus_p.cross(&r.div(r_cross_s));
        //println!("t {t}, u {u}");

        // are the intersection coordinates both in range?
        let t_in_range = 0.0 <= t && t <= 1.0;
        let u_in_range = 0.0 <= u && u <= 1.0;

        if !t_in_range || !u_in_range {
            // No intersection
            return None;
        }

        // Intersection
        //println!("p {p} r {r:?} t {t}");
        Some(PointF64::new(
            p.x as f64 + t * r.x as f64,
            p.y as f64 + t * r.y as f64,
        ))
    }

    pub fn as_inclusive_bresenham_iterator(&self) -> BresenhamInclusive {
        BresenhamInclusive::new(
            (self.p1.x.round() as isize, self.p1.y.round() as isize),
            (self.p2.x.round() as isize, self.p2.y.round() as isize),
        )
    }

    pub fn draw(&self, onto: &mut Box<dyn LikeLayerSlice>, color: Color) {
        for (x, y) in self.as_inclusive_bresenham_iterator() {
            onto.putpixel(Point::new(x, y), color);
        }
    }

    pub fn compute_rendered_pixels(&self) -> Vec<PointF64> {
        let mut px_locations = vec![];
        for (x, y) in self.as_inclusive_bresenham_iterator() {
            px_locations.push(PointF64::new(x as _, y as _));
        }
        px_locations
    }
}

impl Display for LineF64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{} - {}]", self.p1, self.p2)
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub points: Vec<PointF64>,
}

impl Polygon {
    pub fn new(points: &[PointF64]) -> Self {
        Self {
            points: points.to_vec(),
        }
    }

    pub fn scale_by(&self, scale_x: f64, scale_y: f64) -> Self {
        let scaled_points: Vec<PointF64> = self
            .points
            .iter()
            .map(|&p| PointF64::new(p.x * scale_x, p.y * scale_y))
            .collect();
        Polygon::new(&scaled_points)
    }

    fn lines(&self) -> Vec<LineF64> {
        // Generate bounding lines of the polygon
        let mut lines = vec![];
        for (&point, &next_point) in self.points.iter().tuple_windows() {
            lines.push(LineF64::new(point, next_point));
        }
        // Final line connecting the final and first points
        lines.push(LineF64::new(
            *self.points.last().unwrap(),
            *self.points.first().unwrap(),
        ));
        lines
    }

    pub fn bounding_box(&self) -> RectF64 {
        // Find the bounding box of the polygon
        bounding_box_from_edges(&self.lines())
    }

    pub fn draw_outline(&self, onto: &mut Box<dyn LikeLayerSlice>, color: Color) {
        let lines = self.lines();
        for line in lines.iter() {
            line.draw(onto, color);
        }
    }

    pub fn fill(&self, onto: &mut Box<dyn LikeLayerSlice>, color: Color) {
        scanline_fill_from_edges(onto, color, &self.lines())
    }
}

#[derive(Debug, Clone)]
pub struct PolygonStack {
    pub polygons: Vec<Polygon>,
}

impl PolygonStack {
    pub fn new(polygons: &[Polygon]) -> Self {
        Self {
            polygons: polygons.to_vec(),
        }
    }

    pub fn bounding_box(&self) -> RectF64 {
        let first = match self.polygons.first() {
            None => return RectF64::zero(),
            Some(first) => first,
        };
        let mut bounding_box = first.bounding_box();
        for p in self.polygons[1..].iter() {
            bounding_box = bounding_box.union(p.bounding_box())
        }
        bounding_box
    }

    pub fn lines(&self) -> Vec<LineF64> {
        let mut lines = vec![];
        for p in self.polygons.iter() {
            let mut p_lines = p.lines();
            lines.append(&mut p_lines);
        }
        lines
    }

    pub fn fill(&self, onto: &mut Box<dyn LikeLayerSlice>, color: Color, fill_mode: FillMode) {
        //scanline_fill_from_edges(onto, color, &self.lines())
        onto.fill_polygon_stack(self, color, fill_mode);
    }

    pub fn draw_outline(&self, onto: &mut Box<dyn LikeLayerSlice>, color: Color) {
        panic!("don't use this");
        let lines = self.lines();
        for line in lines.iter() {
            line.draw(onto, color);
        }
    }
}

pub fn bounding_box_from_edges(edges: &[LineF64]) -> RectF64 {
    // Find the bounding box of the polygon
    let min_x = edges.iter().fold(f64::INFINITY, |a, &b| a.min(b.min_x()));
    let min_y = edges.iter().fold(f64::INFINITY, |a, &b| a.min(b.min_y()));
    let max_x = edges.iter().fold(-f64::INFINITY, |a, &b| a.max(b.max_x()));
    let max_y = edges.iter().fold(-f64::INFINITY, |a, &b| a.max(b.max_y()));
    RectF64::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

pub fn scanline_compute_fill_lines_from_edges(edges: &[LineF64]) -> Vec<LineF64> {
    // Ref: http://www.sunshine2k.de/coding/java/Polygon/Filling/FillPolygon.htm
    let mut computed_filled_scanlines = vec![];

    // Drop horizontal lines that'd be collinear with the scanline
    let mut lines = edges.to_vec();
    let mut bounding_box = bounding_box_from_edges(edges);

    // Sort lines in ascending y-order
    let mut sorted_lines: Vec<LineF64> = lines.iter().map(|l| l.clone()).collect();
    sorted_lines.sort_by(|&l1, &l2| l1.min_y().partial_cmp(&l2.min_y()).unwrap());

    for scanline_y in
        (bounding_box.min_y().floor() as isize)..(bounding_box.max_y().ceil() as isize)
    {
        // Find all the edges intersected by this scanline
        let scanline = LineF64::new(
            PointF64::new(bounding_box.min_x() as _, scanline_y as _),
            PointF64::new(bounding_box.max_x() as _, scanline_y as _),
        );
        let mut active_edges_and_intersections: Vec<(LineF64, PointF64)> = sorted_lines
            .iter()
            // Trivially filter lines that don't intersect on the Y axis
            .filter(|l| (scanline_y as f64) >= l.min_y() && (scanline_y as f64) < l.max_y())
            .filter_map(|&l| {
                let intersection = scanline.intersection(&l);
                match intersection {
                    None => None,
                    Some(p) => Some((l, p)),
                }
            })
            .collect();
        // Sort the lines by increasing intersection X-coordinate
        active_edges_and_intersections.sort_by(|l1_and_p, l2_and_p| {
            let p1 = l1_and_p.1;
            let p2 = l2_and_p.1;
            p1.x.partial_cmp(&p2.x).unwrap()
        });

        let mut next_line_is_inside = false;
        for (&left_edge_and_intersection, &right_edge_and_intersection) in
            active_edges_and_intersections.iter().tuple_windows()
        {
            next_line_is_inside = !next_line_is_inside;
            if !next_line_is_inside {
                continue;
            }
            //let endpoint: Point = right_edge_and_intersection.1.into();
            let endpoint = right_edge_and_intersection.1;
            let line = LineF64::new(
                left_edge_and_intersection.1,
                PointF64::new(endpoint.x + 1.0, endpoint.y),
            );
            computed_filled_scanlines.push(line);
        }
    }
    computed_filled_scanlines
}

fn scanline_fill_from_edges(onto: &mut Box<dyn LikeLayerSlice>, color: Color, edges: &[LineF64]) {
    for line in scanline_compute_fill_lines_from_edges(edges).into_iter() {
        for (x, y) in line.as_inclusive_bresenham_iterator() {
            onto.putpixel(Point::new(x, y), color);
        }
        //line.draw(onto, color);
    }
}

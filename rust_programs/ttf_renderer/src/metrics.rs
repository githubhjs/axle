use crate::parse_utils::{BigEndianValue, FromFontBufInPlace, TransmuteFontBufInPlace};
use crate::parser::FontParser;
use agx_definitions::Rect;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct HheaTableRaw {
    version: BigEndianValue<u32>,
    ascent: BigEndianValue<i16>,
    descent: BigEndianValue<i16>,
    line_gap: BigEndianValue<i16>,
    advance_width_max: BigEndianValue<u16>,
    left_side_bearing_min: BigEndianValue<i16>,
    right_side_bearing_min: BigEndianValue<i16>,
    max_extent_x: BigEndianValue<i16>,
    caret_slope_rise: BigEndianValue<i16>,
    caret_slope_run: BigEndianValue<i16>,
    caret_offset: BigEndianValue<i16>,
    reserved1: BigEndianValue<i16>,
    reserved2: BigEndianValue<i16>,
    reserved3: BigEndianValue<i16>,
    reserved4: BigEndianValue<i16>,
    metric_data_format: BigEndianValue<i16>,
    long_hor_metrics_count: BigEndianValue<u16>,
}

impl TransmuteFontBufInPlace for HheaTableRaw {}

#[derive(Debug, Clone)]
pub struct HheaTable {
    pub long_hor_metrics_count: usize,
}

impl FromFontBufInPlace<HheaTableRaw> for HheaTable {
    fn from_in_place_buf(raw: &HheaTableRaw) -> Self {
        Self {
            long_hor_metrics_count: raw.long_hor_metrics_count.into_value() as _,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct LongHorMetricRaw {
    advance_width: BigEndianValue<u16>,
    left_side_bearing: BigEndianValue<i16>,
}

impl TransmuteFontBufInPlace for LongHorMetricRaw {}

#[derive(Debug, Clone)]
pub struct LongHorMetric {
    pub advance_width: usize,
    pub left_side_bearing: isize,
}

impl FromFontBufInPlace<LongHorMetricRaw> for LongHorMetric {
    fn from_in_place_buf(raw: &LongHorMetricRaw) -> Self {
        Self {
            advance_width: raw.advance_width.into_value() as _,
            left_side_bearing: raw.left_side_bearing.into_value() as _,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct VerticalMetricsRaw {
    advance_height: BigEndianValue<u16>,
    top_side_bearing: BigEndianValue<i16>,
}

impl TransmuteFontBufInPlace for VerticalMetricsRaw {}

#[derive(Debug, Clone)]
pub struct VerticalMetrics {
    pub advance_height: usize,
    pub top_side_bearing: isize,
}

impl FromFontBufInPlace<VerticalMetricsRaw> for VerticalMetrics {
    fn from_in_place_buf(raw: &VerticalMetricsRaw) -> Self {
        Self {
            advance_height: raw.advance_height.into_value() as _,
            top_side_bearing: raw.top_side_bearing.into_value() as _,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GlyphMetrics {
    pub advance_width: usize,
    pub advance_height: usize,
    pub left_side_bearing: isize,
    pub top_side_bearing: isize,
}

impl GlyphMetrics {
    pub fn new(
        advance_width: usize,
        advance_height: usize,
        left_side_bearing: isize,
        top_side_bearing: isize,
    ) -> Self {
        Self {
            advance_width,
            advance_height,
            left_side_bearing,
            top_side_bearing,
        }
    }

    pub fn scale(&self, scale_x: f64, scale_y: f64) -> Self {
        Self::new(
            (self.advance_width as f64 * scale_x) as usize,
            (self.advance_height as f64 * scale_y) as usize,
            (self.left_side_bearing as f64 * scale_x) as isize,
            (self.top_side_bearing as f64 * scale_y) as isize,
        )
    }
}

#[derive(Debug, Clone)]
pub struct GlyphRenderMetrics {
    pub bounding_box: Rect,
    horizontal_metrics: Option<LongHorMetric>,
    vertical_metrics: Option<VerticalMetrics>,
}

impl GlyphRenderMetrics {
    pub(crate) fn new(bounding_box: &Rect) -> Self {
        Self {
            bounding_box: bounding_box.clone(),
            horizontal_metrics: None,
            vertical_metrics: None,
        }
    }

    pub(crate) fn set_horizontal_metrics(&mut self, metrics: LongHorMetric) {
        self.horizontal_metrics = Some(metrics)
    }

    pub(crate) fn set_vertical_metrics(&mut self, metrics: VerticalMetrics) {
        self.vertical_metrics = Some(metrics)
    }

    pub fn metrics(&self) -> GlyphMetrics {
        let h = self.horizontal_metrics.as_ref().unwrap();
        let v = self
            .vertical_metrics
            .as_ref()
            .unwrap_or(&VerticalMetrics {
                advance_height: self.bounding_box.height() as _,
                top_side_bearing: 0,
            })
            .clone();
        GlyphMetrics::new(
            h.advance_width,
            v.advance_height,
            h.left_side_bearing,
            v.top_side_bearing,
        )
    }
}

pub(crate) fn parse_horizontal_metrics(parser: &FontParser) -> Vec<LongHorMetric> {
    let hhea: HheaTable = parser.parse_table("hhea");
    let hmtx_offset = parser.table_headers.get("hmtx").unwrap().offset;
    let mut cursor = hmtx_offset;
    let mut glyph_metrics = vec![];
    for _ in 0..hhea.long_hor_metrics_count {
        let glyph_metric = LongHorMetric::from_in_place_buf(parser.read_with_cursor(&mut cursor));
        glyph_metrics.push(glyph_metric);
    }
    glyph_metrics
}

pub(crate) fn parse_vertical_metrics(
    parser: &FontParser,
    glyph_count: usize,
) -> Option<Vec<VerticalMetrics>> {
    let vmtx_offset = match parser.table_headers.get("vmtx") {
        None => return None,
        Some(vmtx_header) => vmtx_header.offset,
    };
    let mut cursor = vmtx_offset;
    let mut glyph_metrics = vec![];
    for _ in 0..glyph_count {
        let glyph_metric = VerticalMetrics::from_in_place_buf(parser.read_with_cursor(&mut cursor));
        glyph_metrics.push(glyph_metric);
    }
    Some(glyph_metrics)
}

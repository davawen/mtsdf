use std::collections::HashMap;

use bitflags::bitflags;
use ttf_parser::{Face, GlyphId, OutlineBuilder, Rect};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Vec2 { x: f32, y: f32 }

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output { Vec2 { x: self.x + rhs.x, y: self.y + rhs.y } }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output { Vec2 { x: self.x - rhs.x, y: self.y - rhs.y } }
}

impl std::ops::Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output { Vec2 { x: -self.x, y: -self.y } }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Self::Output { Vec2 { x: self.x * rhs, y: self.y * rhs } }
}

impl std::ops::Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Self::Output { rhs * self }
}

impl Vec2 {
    /// Returns an orthogonal vector
    /// Orthogonal in the counter clockwise direction
    /// if the parameter is true, and in the clockwise direction if it's false.
    fn orthogonal(self, counter_clockwise: bool) -> Vec2 {
        if counter_clockwise { vec2(-self.y, self.x) } else { vec2(self.y, -self.x) }
    }

    fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    fn cross(self, other: Vec2) -> f32 {
        self.x*other.y - self.y*other.x
    }

    fn length_sqr(self) -> f32 { self.x*self.x + self.y*self.y }
    fn length(self) -> f32 { self.length_sqr().sqrt() }
    fn normalize(self) -> Vec2 {
        let l = self.length();
        self * (1.0 / l)
    }

    fn shoelace(self, other: Vec2) -> f32 {
        (other.x - self.x)*(self.y + other.y)
    }
}

fn vec2(x: f32, y: f32) -> Vec2 { Vec2 { x, y } }

pub fn lerp<T: Copy>(a: T, b: T, t: f32) -> T
    where T: std::ops::Add<T, Output = T> + std::ops::Sub<T, Output = T> + std::ops::Mul<f32, Output = T>
{
    a + (b - a)*t
}


bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Color: u8 {
        const BLACK = 0;
        const RED = 1;
        const GREEN = 2;
        const YELLOW = 3;
        const BLUE = 4;
        const MAGENTA = 5;
        const CYAN = 6;
        const WHITE = 7;
    }
}

#[derive(Clone, Copy, PartialEq)]
struct SignedDistance {
    dist: f32,
    dot: f32
}

impl PartialOrd for SignedDistance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.dist.abs().partial_cmp(&other.dist.abs())
            .map(|c| c.then(self.dot.partial_cmp(&other.dot).unwrap_or(std::cmp::Ordering::Equal)))
    }
}

#[derive(Debug, Clone, Copy)]
struct Edge { 
    segment: Segment,
    color: Color
}

#[derive(Debug, Clone)]
struct Contour {
    edges: Vec<Edge>
}

impl Contour {
    /// Returns the winding number of the contour, 1 or -1
    /// Returns 0 if the contour has no edges
    pub fn winding(&self) -> i32 {
        let total = match self.edges.len() {
            0 => return 0,
            1 => {
                let seg = self.edges[0].segment;
                let (a, b, c) = (seg.sample(0.0), seg.sample(1.0/3.0), seg.sample(2.0/3.0));
                a.shoelace(b) + b.shoelace(c) + c.shoelace(a)
            }
            2 => {
                let (sega, segb) = (self.edges[0].segment, self.edges[1].segment);
                let (a, b, c, d) = (sega.sample(0.0), sega.sample(0.5), segb.sample(0.0), segb.sample(0.5));
                a.shoelace(b) + b.shoelace(c) + c.shoelace(d) + d.shoelace(a)
            }
            _ => {
                let mut sum = 0.0;
                let mut prev = self.edges.last().unwrap().segment.sample(0.0);
                for edge in &self.edges {
                    let p = edge.segment.sample(0.0);
                    sum += prev.shoelace(p);
                    prev = p;
                }
                sum
            }

        };
        
        total.signum() as i32
    }
}

mod build;
mod segment;
mod shape;
mod render;

use segment::*;
use shape::{Shape, ColouredShape};

struct Mtsdf {
    image: image::Rgba32FImage,
    atlas: etagere::AtlasAllocator,
    glyphs: HashMap<char, etagere::AllocId>
}

pub fn generate_mtsdf(face: &Face) -> image::Rgba32FImage {
    let mut atlas = etagere::AtlasAllocator::new(etagere::size2(1000, 300));

    let font_size = 50.0;
    let padding = 2.0;

    let mut image = image::Rgba32FImage::new(1000, 300);
    for c in ('A'..='Z').into_iter().chain('0'..='9').chain('a'..='z').chain('*'..='*') {
        let Some(id) = face.glyph_index(c) else { continue };
        let Some(shape) = Shape::from_glyph(face, id) else { continue };

        let coloured = shape.color_edges(2.0, 0);

        let (width, height) = coloured.rendered_glyph_size(face, font_size, padding);

        eprintln!("{id:?} {c} ({width}x{height})");

        let place = atlas.allocate(etagere::size2(width as i32, height as i32)).unwrap();
        let offset = place.rectangle.min;

        coloured.generate_mtsdf(face, font_size, padding, |(x, y), [r, g, b, a]| {
            let median = r.min(g).max(r.max(g).min(b));
            let median = (median - 0.5)*2.0*font_size;

            let pixel = [
                lerp(1.0, 0.0, (median + 0.5).clamp(0.0, 1.0)),
                lerp(1.0, 0.0, (median + 0.5).clamp(0.0, 1.0)),
                lerp(1.0, 0.0, (median + 0.5).clamp(0.0, 1.0)),
                lerp(1.0, 0.0, (median + 0.5).clamp(0.0, 1.0))
            ];

            // let median = -median;
            // let pixel = match median {
            //     ..-0.5 => [1.0, 1.0, 1.0, 1.0],
            //     -0.5..0.5 => [lerp(1.0, 0.0, median + 0.5), lerp(1.0, 0.0, median + 0.5), lerp(1.0, 0.0, median + 0.5), 1.0],
            //     // 0.5..1.5 => [0.0, 0.0, 0.0, 1.0],
            //     0.5..2.5 => {
            //         let t = (median - 0.5).clamp(0.0, 1.0);
            //         [lerp(0.0, 1.0, t), lerp(0.0, 1.0, t), 0.0, 1.0]
            //     }
            //     2.5.. => {
            //         [1.0, 1.0, 0.0, lerp(1.0, 0.0, (median - 2.5).clamp(0.0, 1.0))]
            //     }
            //     _ => [0.0, 0.0, 0.0, 0.0]
            // };

            image.put_pixel((offset.x as u32) + x, (offset.y as u32) + y, image::Rgba(pixel));
        });
    }

    image
}

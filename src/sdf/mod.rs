use std::collections::HashMap;

use bitflags::bitflags;
use ttf_parser::{Face, GlyphId, OutlineBuilder, Rect};

#[derive(Default, Clone, Copy, PartialEq)]
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

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Clone, Copy)]
struct Edge { 
    segment: Segment,
    color: Color
}

#[derive(Clone)]
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
    let mut image = image::Rgba32FImage::new(1000, 300);
    let mut x = 0;
    let mut y = 0;
    for c in ('A'..='Z').into_iter().chain('0'..='9').chain('*'..='*') {
        let Some(id) = face.glyph_index(c) else { continue };
        eprintln!("{id:?} {c}");
        let Some(shape) = Shape::from_glyph(face, id) else { continue };
        let coloured = shape.color_edges(2.0, 0);

        coloured.generate_mtsdf(&mut image, x, y, 40, 40);
        x += 40;
        if x >= 1000-40 {
            x = 0;
            y += 40;
            if y >= 300-40 {
                panic!("ded")
            }
        }
    }

    image
}

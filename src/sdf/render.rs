use ttf_parser::Face;

use super::{shape::ColouredShape, vec2, Color, Edge, Segment, SignedDistance, Vec2};

#[derive(Clone, Copy, PartialEq)]
pub struct MultiDistance {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32
}

impl MultiDistance {
    fn resolve(&self) -> f32{
        self.r.min(self.g).max(self.r.max(self.g).min(self.b))
    }
}

#[derive(Clone)]
struct PerpEdgeSelector {
    min_true_distance: SignedDistance,
    min_negative_perp_dist: f32,
    min_positive_perp_dist: f32,
    near_edge: Option<Edge>,
    near_edge_t: f32
}

/// Returns true if `distance` was modified and false otherwise
fn get_perpendicular_distance(distance: &mut f32, ep: Vec2, edge_dir: Vec2) -> bool {
    let ts = ep.dot(edge_dir);
    if ts > 0.0 {
        let perp_distance = ep.cross(edge_dir);
        if perp_distance.abs() < distance.abs() {
            *distance = perp_distance;
            return true;
        }
    }
    false
}

impl PerpEdgeSelector {
    fn new() -> Self {
        Self {
            min_true_distance: SignedDistance { dist: std::f32::MAX, dot: 0.0 },
            min_positive_perp_dist: std::f32::MAX,
            min_negative_perp_dist: std::f32::MIN,
            near_edge: None,
            near_edge_t: 0.0
        }
    }

    fn merge(&mut self, other: &Self) {
        if other.min_true_distance < self.min_true_distance {
            self.min_true_distance = other.min_true_distance;
            self.near_edge = other.near_edge;
            self.near_edge_t = other.near_edge_t;
        }

        if other.min_negative_perp_dist > self.min_negative_perp_dist {
            self.min_negative_perp_dist = other.min_negative_perp_dist;
        }
        if other.min_positive_perp_dist < self.min_positive_perp_dist {
            self.min_positive_perp_dist = other.min_positive_perp_dist;
        }
    }

    fn add_edge_true_distance(&mut self, edge: &Edge, dist: SignedDistance, t: f32) {
        if dist < self.min_true_distance {
            self.min_true_distance = dist;
            self.near_edge = Some(*edge);
            self.near_edge_t = t;
        }
    }

    fn add_edge_perp_distance(&mut self, dist: f32) {
        if dist <= 0.0 && dist > self.min_negative_perp_dist {
            self.min_negative_perp_dist = dist;
        } else if dist >= 0.0 && dist < self.min_positive_perp_dist {
            self.min_positive_perp_dist = dist;
        }
    }

    fn add_edge(&mut self, point: Vec2, prev_edge: &Edge, edge: &Edge, next_edge: &Edge) {
        let (distance, t) = edge.segment.signed_distance(point);
        self.add_edge_true_distance(edge, distance, t);

        let ap = point - edge.segment.sample(0.0);
        let bp = point - edge.segment.sample(1.0);
        let a_dir = edge.segment.direction(0.0).normalize();
        let b_dir = edge.segment.direction(1.0).normalize();

        let prev_dir = prev_edge.segment.direction(1.0).normalize();
        let next_dir = next_edge.segment.direction(0.0).normalize();

        let add = ap.dot((prev_dir + a_dir).normalize());
        let bdd = -bp.dot((b_dir + next_dir).normalize());

        if add > 0.0 {
            let mut pd = distance.dist;
            if get_perpendicular_distance(&mut pd, ap, -a_dir) {
                pd = -pd;
                self.add_edge_perp_distance(pd);
            }
        }
        if bdd > 0.0 {
            let mut pd = distance.dist;
            if get_perpendicular_distance(&mut pd, bp, b_dir) {
                self.add_edge_perp_distance(pd);
            }
        }
    }

    fn distance(&self, point: Vec2) -> f32 {
        let min_distance = if self.min_true_distance.dist < 0.0 { self.min_negative_perp_dist } else { self.min_positive_perp_dist };

        if let Some(edge) = self.near_edge {
            let distance = self.min_true_distance;
            let distance = edge.segment.distance_to_perp_dist(distance, point, self.near_edge_t);
            if distance.dist.abs() < min_distance.abs() {
                return distance.dist;
            }
        }

        min_distance
    }

    fn true_distance(&self) -> SignedDistance {
        self.min_true_distance
    }
}

#[derive(Clone)]
struct MTEdgeSelector {
    r: PerpEdgeSelector,
    g: PerpEdgeSelector,
    b: PerpEdgeSelector
}

impl MTEdgeSelector {
    fn new() -> Self {
        Self {
            r: PerpEdgeSelector::new(),
            g: PerpEdgeSelector::new(),
            b: PerpEdgeSelector::new(),
        }
    }

    fn merge(&mut self, other: &Self) {
        self.r.merge(&other.r);
        self.g.merge(&other.g);
        self.b.merge(&other.b);
    }

    fn add_edge(&mut self, point: Vec2, prev_edge: &Edge, edge: &Edge, next_edge: &Edge) {
        let (dist, t) = edge.segment.signed_distance(point);
        if edge.color.contains(Color::RED) { self.r.add_edge_true_distance(edge, dist, t); }
        if edge.color.contains(Color::GREEN) { self.g.add_edge_true_distance(edge, dist, t); }
        if edge.color.contains(Color::BLUE) { self.b.add_edge_true_distance(edge, dist, t); }

        let ap = point - edge.segment.sample(0.0);
        let bp = point - edge.segment.sample(1.0);
        let a_dir = edge.segment.direction(0.0).normalize();
        let b_dir = edge.segment.direction(1.0).normalize();

        let prev_dir = prev_edge.segment.direction(1.0).normalize();
        let next_dir = next_edge.segment.direction(0.0).normalize();

        let add = ap.dot((prev_dir + a_dir).normalize());
        let bdd = -bp.dot((b_dir + next_dir).normalize());

        if add > 0.0 {
            let mut pd = dist.dist;
            if get_perpendicular_distance(&mut pd, ap, -a_dir) {
                pd = -pd;
                if edge.color.contains(Color::RED) { self.r.add_edge_perp_distance(pd); }
                if edge.color.contains(Color::GREEN) { self.g.add_edge_perp_distance(pd); }
                if edge.color.contains(Color::BLUE) { self.b.add_edge_perp_distance(pd); }
            }
        }
        if bdd > 0.0 {
            let mut pd = dist.dist;
            if get_perpendicular_distance(&mut pd, bp, b_dir) {
                if edge.color.contains(Color::RED) { self.r.add_edge_perp_distance(pd); }
                if edge.color.contains(Color::GREEN) { self.g.add_edge_perp_distance(pd); }
                if edge.color.contains(Color::BLUE) { self.b.add_edge_perp_distance(pd); }
            }
        }
    }

    /// Returns the r, g, b perpendicular distance,
    /// as well as the true distance in the alpha channel,
    /// not normalized.
    fn distance(&self, point: Vec2) -> MultiDistance {
        let mut a = self.r.true_distance();
        let a1 = self.g.true_distance();
        let a2 = self.b.true_distance();
        if a1 < a { a = a1 }
        if a2 < a { a = a2 }

        MultiDistance {
            r: self.r.distance(point),
            g: self.g.distance(point),
            b: self.b.distance(point),
            a: a.dist
        }
    }
}

pub fn one_shot_distance(shape: &ColouredShape, p: Vec2) -> MultiDistance {
    let mut selector = MTEdgeSelector::new();

    for c in &shape.contours {
        if c.edges.is_empty() { continue }

        let len = c.edges.len();
        let mut prev_edge = if len >= 2 { &c.edges[len - 2] } else { &c.edges[0] };
        let mut cur_edge = c.edges.last().unwrap();
        for next_edge in &c.edges {
            selector.add_edge(p, prev_edge, cur_edge, next_edge);
            prev_edge = cur_edge;
            cur_edge = next_edge;
        }

    }

    selector.distance(p)
} 

impl ColouredShape {
    /// Returns the glyph size, in pixels, rounded up to the nearest integer coordinate.
    ///
    /// Padding refers to additional empty space to add around the (normally tight) bounding-box.
    /// This is useful to encode additional distance information for outlines, for exemple.
    /// Equal padding is added in every direction.
    pub fn rendered_glyph_size(&self, face: &Face, font_size_px: f32, padding: f32) -> (u32, u32) {
        let units = face.units_per_em() as f32;
        let glyph_width = self.bounds.x_max as f32 - self.bounds.x_min as f32;
        let glyph_height = self.bounds.y_max as f32 - self.bounds.y_min as f32;

        let width = font_size_px*glyph_width/units;
        let height = font_size_px*glyph_height/units;

        ((width + padding).ceil() as u32, (height + padding).ceil() as u32)
    }

    /// Generates an MTSDF of the glyph at the given font size.
    ///
    /// If you need to know the bounds of the generated glyph size before-hand,
    /// use [`ColouredShape::rendered_glyph_size`].
    ///
    /// Calls the passed function with:
    /// - X and Y coordinates, passed as a `(u32, u32)` tuple,
    ///   ranging from the top left corner at `(0, 0)`,
    ///   and the bottom-right corner at `(rendered_glyph_width-1, rendered_glyph_height-1)`.
    /// - RGBA signed distance values as a `[f32; 4]` array,
    ///   normalized in the range 0.0 to 1.0, with 0.5 being the zero.
    ///   To get the true pixel distance, use: `font_size_px*2.0*(value-0.5)`
    ///
    /// The algorithm does not support partially overlapping countours.
    pub fn generate_mtsdf<F: FnMut((u32, u32), [f32; 4])>(&self, face: &Face, font_size_px: f32, padding: f32, mut pixel_write_fun: F) {
        let glyph_width = self.bounds.x_max as f32 - self.bounds.x_min as f32;
        let glyph_height = self.bounds.y_max as f32 - self.bounds.y_min as f32;

        let (width, height) = self.rendered_glyph_size(face, font_size_px, padding);

        let image_pixel_to_face = |x: u32, y: u32| -> Vec2 {
            // We add 0.5 to center the pixels (instead of being in the top-left corner)
            let px = self.bounds.x_min as f32 + ((x as f32 - padding) / (width as f32 - padding*2.0))*glyph_width + 0.5;
            let py = self.bounds.y_min as f32 + (1.0 - ((y as f32 - padding) / (height as f32 - padding*2.0)))*glyph_height + 0.5;
            vec2(px, py)
        };

        let units = face.units_per_em() as f32;
        for y in 0..height {
            for x in 0..width {
                let p = image_pixel_to_face(x, y);

                let mut d = one_shot_distance(self, p);
                d.r = (d.r/units)/2.0 + 0.5;
                d.g = (d.g/units)/2.0 + 0.5;
                d.b = (d.b/units)/2.0 + 0.5;
                d.a = (d.a/units)/2.0 + 0.5;

                let pixel = [d.r, d.g, d.b, d.a];
                (pixel_write_fun)((x, y), pixel);
            }
        }
    }
}

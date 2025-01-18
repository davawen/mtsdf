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
        if self.r <= self.g && self.g <= self.b {
            self.g
        } else if self.g <= self.r && self.r <= self.b {
            self.r
        } else {
            self.b
        }
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
    return false;
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

    // fn reset(&mut self, delta: f32) {
    //     self.min_true_distance.dist += self.min_true_distance.dist.signum() * delta;
    //     self.min_negative_perp_dist = -self.min_true_distance.dist.abs();
    //     self.min_positive_perp_dist = self.min_true_distance.dist.abs();
    //     self.near_edge = None;
    //     self.near_edge_t = 0.0;
    // }

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

        let prev_dir = prev_edge.segment.direction(1.0);
        let next_dir = next_edge.segment.direction(0.0);

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

        // if let Some(edge) = self.near_edge {
        //     let distance = self.min_true_distance;
        //     let distance = edge.segment.distance_to_perp_dist(distance, point, self.near_edge_t);
        //     if distance.dist.abs() < min_distance.abs() {
        //         return distance.dist;
        //     }
        // }

        min_distance
    }

    fn true_distance(&self) -> f32 {
        self.min_true_distance.dist
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

        let prev_dir = prev_edge.segment.direction(1.0);
        let next_dir = next_edge.segment.direction(0.0);

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
        MultiDistance {
            r: self.r.distance(point),
            g: self.g.distance(point),
            b: self.b.distance(point),
            a: self.r.true_distance().min(self.g.true_distance()).min(self.b.true_distance())
        }
    }
}

pub fn one_shot_distance(shape: &ColouredShape, p: Vec2) -> MultiDistance {
    let mut selector = PerpEdgeSelector::new();

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

    let mut d = selector.distance(p);
    // d.a = 1.0;
    let d = MultiDistance { r: d, g: d, b: d, a: 1.0 };
    d
} 

impl ColouredShape {
    /// Generates an MTSDF of the given size from the glyph.
    /// Writes into the output image at the given x and y coordinates.
    /// This algorithm supports overlaps.
    pub fn generate_mtsdf(&self, img: &mut image::Rgba32FImage, offset_x: u32, offset_y: u32, width: u32, height: u32) {
        let glyph_width = self.bounds.x_max as f32 - self.bounds.x_min as f32;
        let glyph_height = self.bounds.y_max as f32 - self.bounds.y_min as f32;

        let image_pixel_to_face = |x: u32, y: u32| -> Vec2 {
            let px = self.bounds.x_min as f32 + (x as f32 / width as f32)*glyph_width + 0.5;
            let py = self.bounds.y_min as f32 + (1.0 - (y as f32 / height as f32))*glyph_height + 0.5;
            vec2(px, py)
        };

        let face_pixel_to_image = |p: Vec2| -> (u32, u32) {
            let x = width as f32 * (((p.x - 0.5) - self.bounds.x_min as f32)/glyph_width);
            let y = height as f32 * (1.0 - ((p.y - 0.5) - self.bounds.y_min as f32)/glyph_height);
            (x.clamp(0.0, width as f32 - 1.0) as u32, y.clamp(0.0, height as f32 - 1.0) as u32)
        };

        // let mut min = std::f32::MAX;
        // let mut max = std::f32::MIN;

        for y in 0..height {
            for x in 0..width {
                let p = image_pixel_to_face(x, y);

                let d = one_shot_distance(self, p);
                // max = max.max(d.r).max(d.g).max(d.b).max(d.a);
                // min = min.min(d.r).min(d.g).min(d.b).min(d.a);

                // let pixel = image::Rgba([d.r.signum()/3.0 + 0.5, d.g.signum()/3.0 + 0.5, d.b.signum()/3.0 + 0.5, 1.0]);
                let pixel = image::Rgba([d.r/100.0 + 0.5, d.g/100.0 + 0.5, d.b/100.0 + 0.5, 1.0]);
                img.put_pixel(offset_x + x, offset_y + y, pixel);
            }
        }

        for c in &self.contours {
            for e in &c.edges {
                for t in 0..=50 {
                    let t = t as f32 / 50.0;
                    let p = e.segment.sample(t);
                    let (x, y) = face_pixel_to_image(p);
                    let pixel = [
                        if e.color.contains(Color::RED) { 1.0 } else { 0.0 },
                        if e.color.contains(Color::GREEN) { 1.0 } else { 0.0 },
                        if e.color.contains(Color::BLUE) { 1.0 } else { 0.0 },
                        1.0
                    ];
                    img.put_pixel(offset_x + x, offset_y + y, image::Rgba(pixel));
                }
            }
        }

        // Center range (so that the zero is mapped to 0.5)
        // min = min.min(-max);
        // max = max.max(-min);
        // min = min.min(-max);
        //
        // let remap = |x: f32| (x - min)/(max - min);
        // for y in 0..height {
        //     for x in 0..width {
        //         let [r, g, b, a] = img.get_pixel(offset_x + x, offset_y + y).0;
        //         let pixel = image::Rgba([remap(r), remap(g), remap(b), a]);
        //         img.put_pixel(offset_x + x, offset_y + y, pixel);
        //     }
        // }
    }
}

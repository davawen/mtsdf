use super::{build::Builder, vec2, Color, Contour, Face, GlyphId, Rect, Vec2};

fn extract_seed_bit(seed: &mut u64) -> u64 {
    let v = *seed & 1;
    *seed >>= 1;
    v
}

fn extract_seed_mod3(seed: &mut u64) -> u64 {
    let v = *seed % 3;
    *seed /= 3;
    v
}

fn init_color(seed: &mut u64) -> Color {
    const COLORS: [Color; 3] = [Color::CYAN, Color::MAGENTA, Color::YELLOW];
    COLORS[extract_seed_mod3(seed) as usize]
}

fn switch_color(color: &mut Color, seed: &mut u64) {
    let bit = extract_seed_bit(seed);
    let shifted = color.bits() << (1+bit);
    *color = Color::from_bits((shifted|shifted>>3) & Color::WHITE.bits()).unwrap();
}

fn switch_color_constrained(color: &mut Color, seed: &mut u64, banned: Color) {
    let combined = *color & banned;
    if combined == Color::RED || combined == Color::GREEN || combined == Color::BLUE {
        *color = combined ^ Color::WHITE;
    } else {
        switch_color(color, seed);
    }
}


/// For each position < n, this function will return -1, 0, or 1,
/// depending on whether the position is closer to the beginning, middle, or end, respectively.
/// It is guaranteed that the output will be balanced in that the total for positions 0 through n-1 will be zero.
fn symmetrical_trichotomy(position: i32, n: i32) -> i32 {
    return (3.0 + 2.875*(position as f32)/(n as f32 - 1.0)-1.4375 + 0.5) as i32 - 3;
}

#[derive(Debug, Clone)]
pub struct Shape {
    contours: Vec<Contour>,
    bounds: Rect
}

fn is_corner(a_dir: Vec2, b_dir: Vec2, threshold: f32) -> bool {
    a_dir.dot(b_dir) <= 0.0 || a_dir.cross(b_dir).abs() > threshold
}

impl Shape {
    pub fn from_glyph(face: &Face, glyph: GlyphId) -> Option<Self> {
        let mut builder = Builder::default();

        if let Some(bounds) = face.outline_glyph(glyph, &mut builder) {
            Some(Shape { contours: builder.contours, bounds })
        } else {
            None
        }
    }

    /// Assigns colors to edges of the shape in accordance to the multi-channel distance field
    /// technique. May split some edges if necessary. `angle` specifies the maximum angle (in
    /// radians) to be considered a corner, for example 3 (~172 degrees). Values below 1/2 PI will
    /// be treated as the external angle.
    /// Necessary for MSDF and MTSDF
    pub fn color_edges(mut self, angle: f32, mut seed: u64) -> ColouredShape {
        let seed = &mut seed;
        let cross_threshold = angle.sin();

        let mut color = init_color(seed);

        // cache corner array across each loop
        let mut corners = vec![];

        for contour in &mut self.contours {
            if contour.edges.is_empty() { continue }
            corners.clear();

            // identify corners as curves that change directions across boundaries
            let mut prev = contour.edges.last().unwrap().segment.direction(1.0);
            for (i, edge) in contour.edges.iter().enumerate() {
                if is_corner(prev.normalize(), edge.segment.direction(0.0).normalize(), cross_threshold) {
                    corners.push(i);
                }
                prev = edge.segment.direction(1.0);
            }

            // smooth contour
            if corners.is_empty() {
                switch_color(&mut color, seed);
                for edge in &mut contour.edges {
                    edge.color = color;
                }
            } else if corners.len() == 1 { // teardrop shape
                let mut colors = [Color::BLACK; 3];
                switch_color(&mut color, seed);
                colors[0] = color;
                colors[1] = Color::WHITE;
                switch_color(&mut color, seed);
                colors[2] = color;

                let corner = corners[0];
                let m = contour.edges.len();
                if m >= 3 { // okay -> color edges appropriately
                    for i in 0..m {
                        contour.edges[(corner + i) % m].color = colors[(1 + symmetrical_trichotomy(i as i32, m as i32)) as usize];
                    }
                } else if m == 2 { // 1 or 2 edges -> we need to split edges
                    let a = contour.edges[corner].segment.split_in_three();
                    let b = contour.edges[1 - corner].segment.split_in_three();

                    // color every 2 edges with the same color
                    let colors = colors.into_iter().flat_map(|c| [c, c]);

                    contour.edges = a.into_iter().chain(b)
                        .zip(colors).map(|(s, c)| s.colored(c))
                        .collect();
                } else if m == 1 {
                    contour.edges = contour.edges[0].segment.split_in_three().into_iter()
                        .zip(colors).map(|(s, c)| s.colored(c))
                        .collect();
                }
            } else { // there are multiple corners, but no need to split
                switch_color(&mut color, seed);
                let initial_color = color;

                let mut spline = 0;
                let m = contour.edges.len();
                let corners_len = corners.len();
                let start = corners[0];
                for i in 0..m {
                    let idx = (start + i) % m;
                    if spline + 1 < corners_len && corners[spline + 1] == idx {
                        spline += 1;
                        switch_color_constrained(&mut color, seed, if spline == corners_len-1 { initial_color } else { Color::BLACK });
                    }
                    contour.edges[idx].color = color;
                }
            }
        }

        ColouredShape {
            contours: self.contours,
            bounds: self.bounds
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColouredShape {
    pub contours: Vec<Contour>,
    pub bounds: Rect
}

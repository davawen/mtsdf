use std::f32::consts::PI;

use super::{lerp, Color, Edge, SignedDistance, Vec2};

#[derive(Debug, Clone, Copy)]
pub enum Segment {
    Line(Vec2, Vec2),
    Quad(Vec2, Vec2, Vec2),
    Cubic(Vec2, Vec2, Vec2, Vec2)
}

impl Segment {
    pub fn white_edge(self) -> Edge {
        self.colored(Color::WHITE)
    }

    pub fn colored(self, color: Color) -> Edge {
        Edge { segment: self, color }
    }

    /// Sample the segment at the given percentage
    pub fn sample(&self, t: f32) -> Vec2 {
        match self {
            &Segment::Line(a, b) => lerp(a, b, t),
            &Segment::Quad(a, b, c) => lerp(
                lerp(a, b, t),
                lerp(b, c, t),
                t
            ),
            &Segment::Cubic(a, b, c, d) => {
                let p12 = lerp(b, c, t);
                lerp(lerp(lerp(a, b, t), p12, t), lerp(p12, lerp(c, d, t), t), t)
            }
        }
    }

    /// Returns the direction the edge has at the point specified by the parameter.
    pub fn direction(&self, t: f32) -> Vec2 {
        match self {
            &Segment::Line(a, b) => b - a,
            &Segment::Quad(a, b, c) => {
                let tangent = lerp(b - a, c - b, t);
                if tangent.x == 0.0 && tangent.y == 0.0 { c - a }
                else { tangent }
            },
            &Segment::Cubic(a, b, c, d) => {
                let tangent = lerp(
                    lerp(b - a, c - b, t),
                    lerp(c - b, d - c, t),
                    t
                );

                if tangent.x == 0.0 && tangent.y == 0.0 {
                    if t == 0.0 { c - a }
                    else if t == 1.0 { d - b }
                    else { tangent }
                } else { tangent }
            },
        }
    }

    /// Split this segment into three equal parts
    pub fn split_in_three(self) -> [Self; 3] {
        match &self {
            &Segment::Line(a, b) => {
                let third = self.sample(1.0/3.0);
                let two = self.sample(2.0/3.0);
                [Segment::Line(a, third), Segment::Line(third, two), Segment::Line(two, b)]
            }
            &Segment::Quad(a, b, c) => {
                let third = self.sample(1.0/3.0);
                let two = self.sample(2.0/3.0);
                [
                    Segment::Quad(a, lerp(a, b, 1.0/3.0), third),
                    Segment::Quad(third, lerp(lerp(a, b, 5.0/9.0), lerp(b, c, 4.0/9.0), 0.5), two),
                    Segment::Quad(two, lerp(b, c, 2.0/3.0), c)
                ]
            }
            &Segment::Cubic(a, b, c, d) => {
                let third = self.sample(1.0/3.0);
                let two = self.sample(2.0/3.0);

                let first = Segment::Cubic(
                    a,
                    if a == b { a } else { lerp(a, b, 1.0/3.0) },
                    lerp(lerp(a, b, 1.0/3.0), lerp(b, c, 1.0/3.0), 1.0/3.0),
                    third
                );

                let second = Segment::Cubic(
                    third,
                    lerp(
                        lerp(lerp(a, b, 1.0/3.0), lerp(b, c, 1.0/3.0), 1.0/3.0),
                        lerp(lerp(b, c, 1.0/3.0), lerp(c, d, 1.0/3.0), 1.0/3.0),
                        2.0/3.0
                    ),
                    lerp(
                        lerp(lerp(a, b, 2.0/3.0), lerp(b, c, 2.0/3.0), 2.0/3.0),
                        lerp(lerp(b, c, 2.0/3.0), lerp(c, d, 2.0/3.0), 2.0/3.0),
                        1.0/3.0
                    ),
                    two
                );

                let third = Segment::Cubic(
                    two,
                    lerp(lerp(b, c, 2.0/3.0), lerp(c, d, 2.0/3.0), 2.0/3.0),
                    if c == d { d } else { lerp(c, d, 2.0/3.0) },
                    d
                );

                [first, second, third]
            }
        }
    }

    /// Returns the closest signed distance and the t value corresponding
    /// to the closest point in the curve.
    pub fn signed_distance(&self, p: Vec2) -> (SignedDistance, f32) {
        match self {
            &Segment::Line(p0, p1) => {
                let aq = p - p0;
                let ab = p1 - p0;
                let t = aq.dot(ab) / ab.length_sqr();
                let eq = if t <= 0.5 { p0 } else { p1 } - p;
                let endpoint_dist = eq.length();
                if t > 0.0 && t < 1.0 {
                    let ortho_dist = ab.orthogonal(false).normalize().dot(aq);
                    if ortho_dist.abs() < endpoint_dist {
                        return (SignedDistance { dist: ortho_dist, dot: 0.0 }, t);
                    }
                }

                (SignedDistance { dist: aq.cross(ab).signum()*endpoint_dist, dot: ab.normalize().dot(eq.normalize()).abs() }, t)
            }
            &Segment::Quad(p0, p1, p2) => {
                let qa = p0 - p;
                let ab = p1 - p0;
                let br = p2 - p1 - ab;
                let a = br.length_sqr();
                let b = 3.0 * ab.dot(br);
                let c = 2.0 * ab.length_sqr() + qa.dot(br);
                let d = qa.dot(ab);

                let roots = roots::find_roots_cubic(a, b, c, d);
                let (num_solutions, solutions) = match roots {
                    roots::Roots::No(_) => (0, [0.0, 0.0, 0.0]),
                    roots::Roots::One([x0]) => (1, [x0, 0.0, 0.0]),
                    roots::Roots::Two([x0, x1]) => (2, [x0, x1, 0.0]),
                    roots::Roots::Three([x0, x1, x2]) => (3, [x0, x1, x2]),
                    _ => unreachable!()
                };

                let ep_dir = self.direction(0.0);
                let mut min_dist = ep_dir.cross(qa).signum() * qa.length();
                let mut t = -qa.dot(ep_dir) / ep_dir.length_sqr();

                let ep_dir = self.direction(1.0);
                { 
                    let distance = (p2 - p).length();
                    if distance < min_dist.abs() {
                        min_dist = ep_dir.cross(p2 - p).signum() * distance;
                        t = (p - p1).dot(ep_dir) / ep_dir.length_sqr();
                    }
                }

                for i in 0..num_solutions {
                    if solutions[i] > 0.0 && solutions[i] < 1.0 {
                        let qe = qa + 2.0*solutions[i]*ab + solutions[i]*solutions[i]*br;
                        let distance = qe.length();
                        if distance <= min_dist.abs() {
                            min_dist = (ab + solutions[i]*br).cross(qe).signum() * distance;
                            t = solutions[i];
                        }
                    }
                }

                let dist = min_dist;
                if t >= 0.0 && t <= 1.0 {
                    (SignedDistance { dist, dot: 0.0 }, t)
                } else if t < 0.0 {
                    (SignedDistance { dist, dot: self.direction(0.0).normalize().dot(qa.normalize()).abs() }, t)
                } else {
                    (SignedDistance { dist, dot: self.direction(1.0).normalize().dot((p2 - p).normalize()).abs() }, t)
                }
            }
            &Segment::Cubic(p0, p1, p2, p3) => {
                let qa = p0 - p;
                let ab = p1 - p0;
                let br = p2 - p1 - ab;
                let r#as = (p3 - p2) - (p2 - p1) - br;

                let ep_dir = self.direction(0.0);
                let mut min_distance = ep_dir.cross(qa).signum() * qa.length();
                let mut t = -qa.dot(ep_dir)/ep_dir.length_sqr();

                let ep_dir = self.direction(1.0);
                {
                    let distance = (p3 - p).length();
                    if distance < min_distance.abs() {
                        min_distance = ep_dir.cross(p3 - p).signum() * distance;
                        t = (ep_dir - (p3 - p)).dot(ep_dir) / ep_dir.length_sqr();
                    }
                }

                let mut param = t;

                // Iterative minimum distance search
                const CUBIC_SEARCH_STARTS: usize = 4;
                const CUBIC_SEARCH_STEPS: usize = 4;
                for i in 0..=CUBIC_SEARCH_STARTS {
                    let mut t = (i as f32) / CUBIC_SEARCH_STARTS as f32;
                    let mut qe = qa + 3.0*t*ab + 3.0*t*t*br + t*t*t*r#as;
                    for _ in 0..CUBIC_SEARCH_STEPS {
                        //  import t
                        let d1 = 3.0*ab + 6.0*t*br + 3.0*t*t*r#as;
                        let d2 = 6.0*br + 6.0*t*r#as;

                        t -= qe.dot(d1)/(d1.length_sqr() + qe.dot(d2));
                        if t <= 0.0 || t >= 1.0 { break }

                        qe = qa + 3.0*t*ab + 3.0*t*t*br + t*t*t*r#as;
                        let distance = qe.length();
                        if distance < min_distance.abs() {
                            min_distance = d1.cross(qe).signum() * distance;
                            param = t;
                        }
                    }
                }

                let dist = min_distance;
                if param >= 0.0 && param <= 1.0 {
                    (SignedDistance { dist, dot: 0.0 }, param)
                } else if param < 0.0 {
                    (SignedDistance { dist, dot: self.direction(0.0).normalize().dot(qa.normalize()).abs() }, param)
                } else {
                    (SignedDistance { dist, dot: self.direction(1.0).normalize().dot((p3 - p).normalize()).abs() }, param)
                }
            }
        }
    }

    pub fn distance_to_perp_dist(&self, dist: SignedDistance, p: Vec2, t: f32) -> SignedDistance {
        if t < 0.0 {
            let dir = self.direction(0.0).normalize();
            let aq = p - self.sample(0.0);
            let ts = aq.dot(dir);
            if ts < 0.0 {
                let perp_dist = aq.cross(dir);
                if perp_dist.abs() <= dist.dist.abs() {
                    return SignedDistance {
                        dist: perp_dist, dot: 0.0
                    };
                }
            }
        } else if t > 1.0 {
            let dir = self.direction(1.0).normalize();
            let bq = p - self.sample(1.0);
            let ts = bq.dot(dir);
            if ts > 0.0 {
                let perp_dist = bq.cross(dir);
                if perp_dist.abs() <= dist.dist.abs() {
                    return SignedDistance {
                        dist: perp_dist, dot: 0.0
                    }
                }
            }
        }

        dist
    }
}


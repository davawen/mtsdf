use super::{Contour, Vec2, OutlineBuilder, vec2, Segment};

#[derive(Default)]
pub struct Builder {
    pub contours: Vec<Contour>,
    current: Option<Contour>,
    cur_pos: Vec2
}

impl OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.current = Some(Contour{ edges: vec![] });
        self.cur_pos = vec2(x, y);
    }

    fn close(&mut self) {
        let current = self.current.take().unwrap();
        self.contours.push(current);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let current = self.current.as_mut().unwrap();
        let next = vec2(x, y);
        current.edges.push(Segment::Line(self.cur_pos, next).white_edge());

        self.cur_pos = next;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let current = self.current.as_mut().unwrap();
        
        let next = vec2(x, y);
        current.edges.push(Segment::Quad(self.cur_pos, vec2(x1, y1), next).white_edge());

        self.cur_pos = next;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let current = self.current.as_mut().unwrap();
        
        let next = vec2(x, y);
        current.edges.push(Segment::Cubic(self.cur_pos, vec2(x1, y1), vec2(x2, y2), next).white_edge());

        self.cur_pos = next;
    }
}

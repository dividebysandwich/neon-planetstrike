//! The builder the bases are laid out with: points, boxes, ramps, walls with openings,
//! floor slabs and fixtures, all given in the base's frame.

use glam::{Vec2, Vec3};

use super::facade::{Face, Sec};
use super::*;

pub(super) struct Builder {
    pub(super) origin: Vec3,
    pub(super) q: u8,
    pub(super) out: BaseLayout,
    /// The frames within the base's the builder is in, outermost first (`at`).
    pub(super) frames: Vec<Frame>,
    /// Named nodes of the walk graph (`mark`).
    pub(super) marks: Vec<(&'static str, usize)>,
}

/// A frame within the base's: its origin there (x, z) and floor height, quarter turns, and
/// whether it is mirrored (its +X the other way; applied before the turn).
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) o: Vec2,
    pub(super) y: f32,
    pub(super) turn: u8,
    pub(super) flip: bool,
}

pub(super) fn rot(q: u8, v: Vec2) -> Vec2 {
    match q % 4 {
        0 => v,
        1 => Vec2::new(v.y, -v.x),
        2 => Vec2::new(-v.x, -v.y),
        _ => Vec2::new(-v.y, v.x),
    }
}

/// An opening in a wall: along it from its start (`off`, `w`), from `lo` up to `hi` above the
/// wall's foot.
#[derive(Clone, Copy)]
pub(super) struct Gap {
    pub(super) off: f32,
    pub(super) w: f32,
    pub(super) lo: f32,
    pub(super) hi: f32,
}

pub(super) fn door(off: f32, w: f32, h: f32) -> Gap {
    Gap {
        off,
        w,
        lo: 0.0,
        hi: h,
    }
}

impl Builder {
    /// A point in the base's frame to the world.
    pub(super) fn p(&self, x: f32, y: f32, z: f32) -> Vec3 {
        let (mut v, mut y) = (Vec2::new(x, z), y);
        for f in self.frames.iter().rev() {
            if f.flip {
                v.x = -v.x;
            }
            v = rot(f.turn, v) + f.o;
            y += f.y;
        }
        let r = rot(self.q, v);
        self.origin + Vec3::new(r.x, y, r.y)
    }

    pub(super) fn yaw(&self, local: f32) -> f32 {
        let mut yaw = local;
        for f in self.frames.iter().rev() {
            if f.flip {
                yaw = -yaw;
            }
            yaw += f.turn as f32 * std::f32::consts::FRAC_PI_2;
        }
        yaw + self.q as f32 * std::f32::consts::FRAC_PI_2
    }

    /// A direction of the frame (0 +Z, 1 +X, 2 -Z, 3 -X) in the world.
    pub(super) fn dir(&self, local: u8) -> u8 {
        let mut d = local % 4;
        for f in self.frames.iter().rev() {
            if f.flip && d % 2 == 1 {
                d = (d + 2) % 4;
            }
            d = (d + f.turn) % 4;
        }
        (d + self.q) % 4
    }

    /// Builds in a frame within the current one: at `o` (x, z) on floor `y`, turned `turn`
    /// quarters, mirrored if `flip`.
    pub(super) fn at<R>(
        &mut self,
        o: Vec2,
        y: f32,
        turn: u8,
        flip: bool,
        f: impl FnOnce(&mut Builder) -> R,
    ) -> R {
        self.frames.push(Frame { o, y, turn, flip });
        let r = f(self);
        self.frames.pop();
        r
    }

    /// A node of the walk graph (where bots walk to; the frame's point).
    pub(super) fn node(&mut self, x: f32, y: f32, z: f32) -> usize {
        let p = self.p(x, y, z);
        self.out.graph.nodes.push(p);
        self.out.graph.nodes.len() - 1
    }

    /// A straight walk between two nodes (either way).
    pub(super) fn link(&mut self, a: usize, b: usize) {
        self.out.graph.links.push((a as u32, b as u32));
    }

    /// Nodes at the frame's (x, z) points on floor `y`, linked one to the next, the first to
    /// node `from` (if any); the nodes.
    pub(super) fn chain(&mut self, from: Option<usize>, y: f32, pts: &[(f32, f32)]) -> Vec<usize> {
        let mut last = from;
        let mut out = Vec::new();
        for &(x, z) in pts {
            let n = self.node(x, y, z);
            if let Some(l) = last {
                self.link(l, n);
            }
            last = Some(n);
            out.push(n);
        }
        out
    }

    /// Names a node, for `marked`.
    pub(super) fn mark(&mut self, name: &'static str, node: usize) {
        self.marks.push((name, node));
    }

    pub(super) fn marked(&self, name: &str) -> usize {
        self.marks
            .iter()
            .rev()
            .find(|m| m.0 == name)
            .unwrap_or_else(|| panic!("no node {name}"))
            .1
    }

    /// The walk from one marked node to another.
    pub(super) fn walk(&self, from: &str, to: &str) -> Vec<Vec3> {
        self.out
            .graph
            .path(self.marked(from), self.marked(to))
            .unwrap_or_else(|| panic!("no way from {from} to {to}"))
    }

    pub(super) fn world_box(&self, min: Vec3, max: Vec3) -> Collider {
        let a = self.p(min.x, min.y, min.z);
        let b = self.p(max.x, max.y, max.z);
        Collider::Box {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub(super) fn boxed(&mut self, min: Vec3, max: Vec3, style: Style) {
        let c = self.world_box(min, max);
        self.out.colliders.push(c);
        self.out.styles.push(style);
    }

    /// A ramp climbing towards local direction `dir` (0 +Z, 1 +X, 2 -Z, 3 -X).
    pub(super) fn ramp(&mut self, min: Vec3, max: Vec3, dir: u8) {
        let a = self.p(min.x, min.y, min.z);
        let b = self.p(max.x, max.y, max.z);
        self.out.colliders.push(Collider::Ramp {
            min: a.min(b),
            max: a.max(b),
            dir: self.dir(dir),
        });
        self.out.styles.push(Style::Ramp);
    }

    /// A wall from `a` to `b` (along x or z), `t` thick, `y0`..`y1` high, with openings.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wall(
        &mut self,
        a: Vec2,
        b: Vec2,
        t: f32,
        y0: f32,
        y1: f32,
        gaps: &[Gap],
        style: Style,
    ) {
        let along_x = (b.y - a.y).abs() < 1e-3;
        let len = if along_x { b.x - a.x } else { b.y - a.y };
        let mut cuts = gaps.to_vec();
        cuts.sort_by(|x, y| x.off.total_cmp(&y.off));
        // Pieces: (start, end, bottom, top) along the wall.
        let mut pieces: Vec<(f32, f32, f32, f32)> = Vec::new();
        let mut at = 0.0;
        for g in &cuts {
            if g.off > at {
                pieces.push((at, g.off, y0, y1));
            }
            if g.lo > 0.0 {
                pieces.push((g.off, g.off + g.w, y0, y0 + g.lo));
            }
            if y0 + g.hi < y1 {
                pieces.push((g.off, g.off + g.w, y0 + g.hi, y1));
            }
            at = g.off + g.w;
        }
        if at < len {
            pieces.push((at, len, y0, y1));
        }
        for (s, e, lo, hi) in pieces {
            let (min, max) = if along_x {
                (
                    Vec3::new(a.x + s, lo, a.y - t / 2.0),
                    Vec3::new(a.x + e, hi, a.y + t / 2.0),
                )
            } else {
                (
                    Vec3::new(a.x - t / 2.0, lo, a.y + s),
                    Vec3::new(a.x + t / 2.0, hi, a.y + e),
                )
            };
            self.boxed(min, max, style);
        }
    }

    /// A floor slab over the rectangle, with holes cut (rectangles, x0, z0, x1, z1).
    pub(super) fn slab(
        &mut self,
        r: (f32, f32, f32, f32),
        y: f32,
        holes: &[(f32, f32, f32, f32)],
        style: Style,
    ) {
        // Split the rectangle by the holes' x and z edges into a grid, keep cells not in a hole.
        let mut xs = vec![r.0, r.2];
        let mut zs = vec![r.1, r.3];
        for h in holes {
            xs.extend([h.0.clamp(r.0, r.2), h.2.clamp(r.0, r.2)]);
            zs.extend([h.1.clamp(r.1, r.3), h.3.clamp(r.1, r.3)]);
        }
        xs.sort_by(f32::total_cmp);
        xs.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        zs.sort_by(f32::total_cmp);
        zs.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        for i in 0..xs.len() - 1 {
            // Merge runs along z into one box per column.
            let mut run: Option<f32> = None;
            for j in 0..zs.len() {
                let solid = j < zs.len() - 1 && {
                    let (cx, cz) = ((xs[i] + xs[i + 1]) / 2.0, (zs[j] + zs[j + 1]) / 2.0);
                    !holes
                        .iter()
                        .any(|h| cx > h.0 && cx < h.2 && cz > h.1 && cz < h.3)
                };
                match (solid, run) {
                    (true, None) => run = Some(zs[j]),
                    (false, Some(z0)) => {
                        self.boxed(
                            Vec3::new(xs[i], y - SLAB, z0),
                            Vec3::new(xs[i + 1], y, zs[j]),
                            style,
                        );
                        run = None;
                    }
                    _ => {}
                }
            }
        }
    }

    pub(super) fn lamp(&mut self, x: f32, y: f32, z: f32) {
        let pos = self.p(x, y, z);
        self.out.lamps.push(Lamp { pos, aim: None });
    }

    pub(super) fn flood(&mut self, at: (f32, f32, f32), aim: (f32, f32, f32)) {
        let pos = self.p(at.0, at.1, at.2);
        let aim = self.p(aim.0, aim.1, aim.2);
        self.out.lamps.push(Lamp {
            pos,
            aim: Some(aim),
        });
    }

    pub(super) fn terminal(&mut self, kind: TerminalKind, x: f32, y: f32, z: f32, yaw: f32) {
        let pos = self.p(x, y, z);
        let yaw = self.yaw(yaw);
        self.out.terminals.push(Terminal {
            kind,
            pos,
            yaw,
            pad: None,
        });
    }

    pub(super) fn decor(&mut self, x: f32, y: f32, z: f32, size: Vec3) {
        self.boxed(
            Vec3::new(x - size.x / 2.0, y, z - size.z / 2.0),
            Vec3::new(x + size.x / 2.0, y + size.y, z + size.z / 2.0),
            Style::Decor,
        );
    }

    /// A drawn-only piece along face `f` from `s` to `e`, its section `sec` (out from the face,
    /// height) round it; mitred where it reaches the face's corners.
    pub(super) fn run(&mut self, f: &Face, s: f32, e: f32, sec: Sec, finish: Finish) {
        let (s, e) = (s.max(0.0), e.min(f.len));
        if e - s < 0.05 {
            return;
        }
        let (ms, me) = (s <= 1e-3, e >= f.len - 1e-3);
        let mut pts = [Vec3::ZERO; 8];
        for (k, &(o, y)) in sec.iter().enumerate() {
            let a = f.at(s - if ms { o } else { 0.0 }, o);
            let b = f.at(e + if me { o } else { 0.0 }, o);
            pts[k] = self.p(a.x, y, a.y);
            pts[k + 4] = self.p(b.x, y, b.y);
        }
        self.out.details.push(Detail { pts, finish });
    }

    /// `run` along the whole face but for the stretches `cuts`.
    pub(super) fn runs(&mut self, f: &Face, cuts: &[(f32, f32)], sec: Sec, finish: Finish) {
        let mut cuts = cuts.to_vec();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut at = 0.0;
        for (s, e) in cuts {
            self.run(f, at, s, sec, finish);
            at = at.max(e);
        }
        self.run(f, at, f.len, sec, finish);
    }
}

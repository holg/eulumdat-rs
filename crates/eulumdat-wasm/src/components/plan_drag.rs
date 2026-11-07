//! Moving luminaires and sensors on a plan by dragging (Interior designer
//! and outdoor area designer).
//!
//! A layout is computed (grid, automatic placement, …); the items the user
//! moved are kept as [`Moves`] on top of it, valid for the layout they were
//! made in (`key`). A different layout (another count, another placement)
//! drops them.

/// What a drag on the plan moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveWhat {
    Luminaires,
    Sensors,
    /// Emergency luminaires (Interior designer).
    Emergency,
}

/// Items moved by hand: (index in the computed layout, x, y) in metres.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Moves {
    /// The layout the moves belong to (e.g. "3x3", "auto:9").
    pub key: String,
    pub list: Vec<(usize, f64, f64)>,
}

impl Moves {
    /// The moves for layout `key` (none if they belong to another one).
    pub fn for_key(&self, key: &str) -> &[(usize, f64, f64)] {
        if self.key == key {
            &self.list
        } else {
            &[]
        }
    }

    /// Item `i` of layout `key` is now at (x, y).
    pub fn set(&mut self, key: &str, i: usize, x: f64, y: f64) {
        if self.key != key {
            self.key = key.to_string();
            self.list.clear();
        }
        match self.list.iter_mut().find(|e| e.0 == i) {
            Some(e) => (e.1, e.2) = (x, y),
            None => self.list.push((i, x, y)),
        }
    }

    /// Put the moved items of layout `key` at their places.
    pub fn apply(&self, key: &str, positions: &mut [(f64, f64)]) {
        for &(i, x, y) in self.for_key(key) {
            if let Some(p) = positions.get_mut(i) {
                *p = (x, y);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Link value `key~i:x:y,i:x:y` (cm precision), `None` when nothing moved.
    pub fn encode(&self) -> Option<String> {
        if self.list.is_empty() {
            return None;
        }
        let items: Vec<String> = self
            .list
            .iter()
            .map(|(i, x, y)| format!("{i}:{x:.2}:{y:.2}"))
            .collect();
        Some(format!("{}~{}", self.key, items.join(",")))
    }

    /// Read a link value written by [`Self::encode`] (bad parts are skipped).
    pub fn decode(s: &str) -> Self {
        let Some((key, items)) = s.split_once('~') else {
            return Self::default();
        };
        let list = items
            .split(',')
            .filter_map(|it| {
                let mut p = it.split(':');
                let i = p.next()?.parse().ok()?;
                let x: f64 = p.next()?.parse().ok()?;
                let y: f64 = p.next()?.parse().ok()?;
                (x.is_finite() && y.is_finite()).then_some((i, x, y))
            })
            .collect();
        Self {
            key: key.to_string(),
            list,
        }
    }
}

/// Index of the position nearest to `p`, if it is within `max` (m).
pub fn nearest(positions: &[(f64, f64)], p: (f64, f64), max: f64) -> Option<usize> {
    positions
        .iter()
        .enumerate()
        .map(|(i, q)| (i, (q.0 - p.0).hypot(q.1 - p.1)))
        .filter(|&(_, d)| d <= max)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The viewBox point under the pointer for the first `<svg>` in
/// `container` (viewBox `w` × `h`, `preserveAspectRatio` "meet", the
/// default: scaled uniformly and centred).
pub fn viewbox_point(
    container: &web_sys::Element,
    client_x: f64,
    client_y: f64,
    w: f64,
    h: f64,
) -> Option<(f64, f64)> {
    let svg = container.query_selector("svg").ok().flatten()?;
    let rect = svg.get_bounding_client_rect();
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return None;
    }
    let scale = (rect.width() / w).min(rect.height() / h);
    let off_x = (rect.width() - w * scale) / 2.0;
    let off_y = (rect.height() - h * scale) / 2.0;
    Some((
        (client_x - rect.left() - off_x) / scale,
        (client_y - rect.top() - off_y) / scale,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_belong_to_their_layout() {
        let mut m = Moves::default();
        m.set("3x3", 4, 1.0, 2.0);
        m.set("3x3", 4, 1.5, 2.5);
        m.set("3x3", 0, 0.5, 0.5);
        assert_eq!(m.list.len(), 2);
        let mut pos = vec![(0.0, 0.0); 9];
        m.apply("3x3", &mut pos);
        assert_eq!(pos[4], (1.5, 2.5));
        assert_eq!(pos[0], (0.5, 0.5));
        // another layout: nothing applies, and a move there starts over
        assert!(m.for_key("4x3").is_empty());
        m.set("4x3", 1, 3.0, 3.0);
        assert_eq!(m.list, vec![(1, 3.0, 3.0)]);
        // an index past the layout is ignored
        m.set("4x3", 99, 1.0, 1.0);
        let mut pos = vec![(0.0, 0.0); 12];
        m.apply("4x3", &mut pos);
        assert_eq!(pos[1], (3.0, 3.0));
    }

    #[test]
    fn link_round_trip() {
        let mut m = Moves::default();
        assert_eq!(m.encode(), None);
        m.set("auto:9", 2, 1.234, 5.0);
        m.set("auto:9", 7, 0.0, 8.0);
        let s = m.encode().unwrap();
        assert_eq!(s, "auto:9~2:1.23:5.00,7:0.00:8.00");
        let back = Moves::decode(&s);
        assert_eq!(back.key, "auto:9");
        assert_eq!(back.list, vec![(2, 1.23, 5.0), (7, 0.0, 8.0)]);
        assert_eq!(Moves::decode("junk"), Moves::default());
        assert_eq!(Moves::decode("k~1:x:2,3:1:1").list, vec![(3, 1.0, 1.0)]);
    }

    #[test]
    fn nearest_within_reach() {
        let pos = [(0.0, 0.0), (2.0, 0.0), (2.2, 0.1)];
        assert_eq!(nearest(&pos, (2.15, 0.0), 0.5), Some(2));
        assert_eq!(nearest(&pos, (1.0, 1.0), 0.5), None);
    }
}

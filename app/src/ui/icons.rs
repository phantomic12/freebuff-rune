//! Icons, drawn as vector shapes in a 16-point box so they stay sharp at any scale. The shell
//! used Lucide glyphs; these are the same silhouettes, drawn by Rune's stroke rasteriser.

use rune::{Color, El, Shape, canvas};

/// Points of a circle as a polyline: Rune strokes polylines, so a circle is one closed ring.
fn ring(cx: f32, cy: f32, r: f32) -> Vec<[f32; 2]> {
    let n = 32;
    (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<[f32; 2]> {
    vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]]
}

/// A stroked icon: `parts` are polylines in a 16×16 box, drawn `size` points square.
pub fn lines(c: Color, size: f32, parts: &[Vec<[f32; 2]>]) -> El {
    let mut sh = Shape::new().view_box(16.0, 16.0);
    for p in parts {
        sh = sh.stroke(p.clone(), 1.5, c);
    }
    canvas(sh).size(size, size)
}

/// A filled icon: same box, filled polygons instead of strokes.
pub fn solid(c: Color, size: f32, parts: &[Vec<[f32; 2]>]) -> El {
    let mut sh = Shape::new().view_box(16.0, 16.0);
    for p in parts {
        sh = sh.fill(p.clone(), c);
    }
    canvas(sh).size(size, size)
}

pub fn chevron_left(c: Color, size: f32) -> El {
    lines(c, size, &[vec![[10.2, 4.4], [5.8, 8.0], [10.2, 11.6]]])
}

pub fn chevron_right(c: Color, size: f32) -> El {
    lines(c, size, &[vec![[5.8, 4.4], [10.2, 8.0], [5.8, 11.6]]])
}

pub fn chevron_down(c: Color, size: f32) -> El {
    lines(c, size, &[vec![[4.4, 6.4], [8.0, 10.0], [11.6, 6.4]]])
}

pub fn plus(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[vec![[8.0, 3.4], [8.0, 12.6]], vec![[3.4, 8.0], [12.6, 8.0]]],
    )
}

pub fn cross(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            vec![[4.6, 4.6], [11.4, 11.4]],
            vec![[11.4, 4.6], [4.6, 11.4]],
        ],
    )
}

pub fn search(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[ring(7.0, 7.0, 4.1), vec![[10.1, 10.1], [13.2, 13.2]]],
    )
}

pub fn home(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            vec![[2.8, 7.8], [8.0, 3.4], [13.2, 7.8]],
            vec![[4.7, 7.4], [4.7, 12.8], [11.3, 12.8], [11.3, 7.4]],
        ],
    )
}

/// The two-pane window mark: the rail's second button.
pub fn panels(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[rect(2.8, 3.4, 10.4, 9.2), vec![[8.0, 3.4], [8.0, 12.6]]],
    )
}

/// Four squares: the agents/TODOs surface.
pub fn squares(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            rect(3.0, 3.0, 4.0, 4.0),
            rect(9.0, 3.0, 4.0, 4.0),
            rect(3.0, 9.0, 4.0, 4.0),
            rect(9.0, 9.0, 4.0, 4.0),
        ],
    )
}

pub fn gift(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            rect(3.0, 6.6, 10.0, 6.4),
            vec![[8.0, 6.6], [8.0, 13.0]],
            vec![[3.0, 8.8], [13.0, 8.8]],
            vec![[8.0, 6.4], [6.0, 4.2], [4.4, 6.0], [8.0, 6.4]],
            vec![[8.0, 6.4], [10.0, 4.2], [11.6, 6.0], [8.0, 6.4]],
        ],
    )
}

pub fn gear(c: Color, size: f32) -> El {
    let mut parts = vec![ring(8.0, 8.0, 3.1)];
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let (s, co) = (a.sin(), a.cos());
        parts.push(vec![
            [8.0 + 4.6 * co, 8.0 + 4.6 * s],
            [8.0 + 6.0 * co, 8.0 + 6.0 * s],
        ]);
    }
    lines(c, size, &parts)
}

pub fn arrow_up(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            vec![[8.0, 12.6], [8.0, 3.6]],
            vec![[4.2, 7.2], [8.0, 3.4], [11.8, 7.2]],
        ],
    )
}

pub fn stop(c: Color, size: f32) -> El {
    solid(
        c,
        size,
        &[vec![[5.2, 5.2], [10.8, 5.2], [10.8, 10.8], [5.2, 10.8]]],
    )
}

pub fn terminal(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            rect(2.6, 3.6, 10.8, 8.8),
            vec![[5.2, 6.6], [7.0, 8.0], [5.2, 9.4]],
            vec![[8.4, 9.6], [10.6, 9.6]],
        ],
    )
}

pub fn folder(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[vec![
            [2.8, 12.4],
            [2.8, 4.4],
            [6.6, 4.4],
            [7.8, 6.2],
            [13.2, 6.2],
            [13.2, 12.4],
            [2.8, 12.4],
        ]],
    )
}

/// The split-view mark at the conversation's top right.
pub fn split(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[rect(2.8, 3.4, 10.4, 9.2), vec![[9.8, 3.4], [9.8, 12.6]]],
    )
}

pub fn list(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            vec![[5.4, 4.6], [13.0, 4.6]],
            vec![[5.4, 8.0], [13.0, 8.0]],
            vec![[5.4, 11.4], [13.0, 11.4]],
            vec![[2.8, 4.6], [3.6, 4.6]],
            vec![[2.8, 8.0], [3.6, 8.0]],
            vec![[2.8, 11.4], [3.6, 11.4]],
        ],
    )
}

pub fn branch(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            vec![[5.4, 4.6], [5.4, 11.2]],
            vec![[5.4, 7.6], [10.0, 7.6], [10.0, 9.6]],
            vec![[5.4, 4.6], [5.4, 4.6]],
        ],
    )
}

pub fn globe(c: Color, size: f32) -> El {
    lines(
        c,
        size,
        &[
            ring(8.0, 8.0, 5.2),
            vec![[2.8, 8.0], [13.2, 8.0]],
            vec![[8.0, 2.8], [8.0, 13.2]],
        ],
    )
}

/// The freebuff mark: two nested notched squares, filled from the shipped logo's polygons.
pub fn logo(c: Color, size: f32) -> El {
    let outer = vec![
        [77.18, 0.0],
        [77.18, 19.3],
        [19.3, 19.3],
        [19.3, 77.19],
        [0.0, 77.19],
        [0.0, 0.0],
    ];
    let inner = vec![
        [63.62, 24.89],
        [63.62, 44.19],
        [44.32, 44.19],
        [44.32, 77.19],
        [25.03, 77.19],
        [25.03, 24.89],
    ];
    let sh = Shape::new()
        .view_box(77.18, 77.19)
        .fill(outer, c)
        .fill(inner, c);
    canvas(sh).size(size, size)
}

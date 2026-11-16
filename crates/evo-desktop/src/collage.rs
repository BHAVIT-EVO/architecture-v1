//! Collage layout math (public APIs only; no private space APIs).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }

pub fn layout(count: usize, frame: Rect, gap: i32) -> Vec<Rect> {
    let n = count.clamp(1, 8);
    // simple grid
    let cols = match n {
        1 => 1,
        2 => 2,
        3..=4 => 2,
        5..=6 => 3,
        _ => 4,
    };
    let rows = n.div_ceil(cols);
    let cell_w = (frame.w - gap * (cols as i32 + 1)) / cols as i32;
    let cell_h = (frame.h - gap * (rows as i32 + 1)) / rows as i32;
    let mut out = Vec::new();
    for i in 0..n {
        let r = i / cols;
        let c = i % cols;
        out.push(Rect {
            x: frame.x + gap + c as i32 * (cell_w + gap),
            y: frame.y + gap + r as i32 * (cell_h + gap),
            w: cell_w,
            h: cell_h,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_1_to_8() {
        for n in 1..=8 {
            let r = layout(n, Rect { x:0,y:0,w:1000,h:800 }, 12);
            assert_eq!(r.len(), n);
        }
    }
    #[test]
    fn odd_screen() {
        let r = layout(3, Rect { x:0,y:0,w:1013,h:797 }, 12);
        assert_eq!(r.len(), 3);
    }
}
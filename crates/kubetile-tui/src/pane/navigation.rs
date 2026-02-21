use ratatui::prelude::Rect;

use super::types::{Direction, PaneId};

/// Given current pane Rect and all pane Rects, find the best pane
/// in the given direction (up/down/left/right).
///
/// Algorithm:
/// 1. Filter panes in the correct relative direction
/// 2. Score by overlap on the perpendicular axis
/// 3. Among candidates with overlap > 0, pick the closest
/// 4. If no overlap candidates, pick the nearest by center distance
pub fn find_pane_in_direction(current: (PaneId, Rect), all: &[(PaneId, Rect)], direction: Direction) -> Option<PaneId> {
    let (cur_id, cur) = current;

    let candidates: Vec<_> = all
        .iter()
        .filter(|(id, _)| *id != cur_id)
        .filter(|(_, r)| match direction {
            Direction::Right => r.x >= cur.x + cur.width,
            Direction::Left => r.x + r.width <= cur.x,
            Direction::Down => r.y >= cur.y + cur.height,
            Direction::Up => r.y + r.height <= cur.y,
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let with_overlap: Vec<_> = candidates
        .iter()
        .filter_map(|(id, r)| {
            let overlap = perpendicular_overlap(cur, *r, direction);
            if overlap > 0 {
                let dist = edge_distance(cur, *r, direction);
                Some((*id, dist, overlap))
            } else {
                None
            }
        })
        .collect();

    if !with_overlap.is_empty() {
        return with_overlap.iter().min_by_key(|(_, dist, overlap)| (*dist, -(*overlap as i32))).map(|(id, _, _)| *id);
    }

    let cx = cur.x as i32 + cur.width as i32 / 2;
    let cy = cur.y as i32 + cur.height as i32 / 2;
    candidates
        .iter()
        .min_by_key(|(_, r)| {
            let rx = r.x as i32 + r.width as i32 / 2;
            let ry = r.y as i32 + r.height as i32 / 2;
            (cx - rx).pow(2) + (cy - ry).pow(2)
        })
        .map(|(id, _)| *id)
}

fn perpendicular_overlap(a: Rect, b: Rect, direction: Direction) -> u16 {
    match direction {
        Direction::Left | Direction::Right => {
            let a_start = a.y;
            let a_end = a.y + a.height;
            let b_start = b.y;
            let b_end = b.y + b.height;
            a_end.min(b_end).saturating_sub(a_start.max(b_start))
        }
        Direction::Up | Direction::Down => {
            let a_start = a.x;
            let a_end = a.x + a.width;
            let b_start = b.x;
            let b_end = b.x + b.width;
            a_end.min(b_end).saturating_sub(a_start.max(b_start))
        }
    }
}

fn edge_distance(from: Rect, to: Rect, direction: Direction) -> u16 {
    match direction {
        Direction::Right => to.x.saturating_sub(from.x + from.width),
        Direction::Left => from.x.saturating_sub(to.x + to.width),
        Direction::Down => to.y.saturating_sub(from.y + from.height),
        Direction::Up => from.y.saturating_sub(to.y + to.height),
    }
}

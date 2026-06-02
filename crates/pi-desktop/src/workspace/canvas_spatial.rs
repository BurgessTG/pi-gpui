use std::collections::{HashMap, HashSet};

use super::canvas_model::CanvasDrawingBounds;

const CELL_SIZE: f32 = 512.0;

type CellCoord = (i32, i32);

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct CanvasSpatialIndex {
    cells: HashMap<CellCoord, Vec<usize>>,
    index_cells: Vec<Vec<CellCoord>>,
}

impl CanvasSpatialIndex {
    pub(super) fn rebuild(bounds: &[Option<CanvasDrawingBounds>]) -> Self {
        let mut index = Self::default();
        index.index_cells.resize_with(bounds.len(), Vec::new);
        for (drawing_index, drawing_bounds) in bounds.iter().enumerate() {
            index.set(drawing_index, drawing_bounds.as_ref());
        }
        index
    }

    pub(super) fn push(&mut self, drawing_index: usize, bounds: Option<&CanvasDrawingBounds>) {
        if drawing_index >= self.index_cells.len() {
            self.index_cells.resize_with(drawing_index + 1, Vec::new);
        }
        self.set(drawing_index, bounds);
    }

    pub(super) fn set(&mut self, drawing_index: usize, bounds: Option<&CanvasDrawingBounds>) {
        if drawing_index >= self.index_cells.len() {
            self.index_cells.resize_with(drawing_index + 1, Vec::new);
        }
        self.remove_from_old_cells(drawing_index);

        let Some(bounds) = bounds else {
            return;
        };
        let cells = cells_for_bounds(bounds);
        for cell in &cells {
            self.cells.entry(*cell).or_default().push(drawing_index);
        }
        self.index_cells[drawing_index] = cells;
    }

    pub(super) fn query(&self, bounds: &CanvasDrawingBounds) -> Vec<usize> {
        let mut seen = HashSet::new();
        let mut drawing_indices = cells_for_bounds(bounds)
            .into_iter()
            .filter_map(|cell| self.cells.get(&cell))
            .flat_map(|indices| indices.iter().copied())
            .filter(|drawing_index| seen.insert(*drawing_index))
            .collect::<Vec<_>>();
        drawing_indices.sort_unstable();
        drawing_indices
    }

    fn remove_from_old_cells(&mut self, drawing_index: usize) {
        for cell in self.index_cells[drawing_index].drain(..) {
            let Some(indices) = self.cells.get_mut(&cell) else {
                continue;
            };
            indices.retain(|candidate| *candidate != drawing_index);
            if indices.is_empty() {
                self.cells.remove(&cell);
            }
        }
    }
}

fn cells_for_bounds(bounds: &CanvasDrawingBounds) -> Vec<CellCoord> {
    let min_x = cell_coord(bounds.left);
    let max_x = cell_coord(bounds.right);
    let min_y = cell_coord(bounds.top);
    let max_y = cell_coord(bounds.bottom);
    let mut cells = Vec::with_capacity(((max_x - min_x + 1) * (max_y - min_y + 1)).max(1) as usize);
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            cells.push((x, y));
        }
    }
    cells
}

fn cell_coord(value: f32) -> i32 {
    (value / CELL_SIZE).floor() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(left: f32, top: f32, right: f32, bottom: f32) -> CanvasDrawingBounds {
        CanvasDrawingBounds {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn query_returns_unique_sorted_indices_across_cells() {
        let mut index = CanvasSpatialIndex::default();
        index.push(2, Some(&bounds(500.0, 500.0, 1_100.0, 1_100.0)));
        index.push(0, Some(&bounds(-900.0, -900.0, -860.0, -860.0)));
        index.push(1, Some(&bounds(700.0, 700.0, 730.0, 730.0)));

        assert_eq!(index.query(&bounds(490.0, 490.0, 800.0, 800.0)), vec![1, 2]);
    }

    #[test]
    fn set_moves_indices_between_cells() {
        let mut index = CanvasSpatialIndex::default();
        index.push(0, Some(&bounds(0.0, 0.0, 20.0, 20.0)));
        assert_eq!(index.query(&bounds(0.0, 0.0, 30.0, 30.0)), vec![0]);

        index.set(0, Some(&bounds(2_000.0, 2_000.0, 2_100.0, 2_100.0)));

        assert!(index.query(&bounds(0.0, 0.0, 30.0, 30.0)).is_empty());
        assert_eq!(
            index.query(&bounds(1_900.0, 1_900.0, 2_200.0, 2_200.0)),
            vec![0]
        );
    }
}

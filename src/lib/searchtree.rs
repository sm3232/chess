use core::f32;
use std::ops::IndexMut;

use eframe::egui;


use super::piece::Parity;
#[derive(Debug)]
pub struct SearchTree {
    pub value: Parity,
    pub children: Vec<SearchTree>,
    stack: Vec<usize>,
    __x: f32,
    __y: f32,
    __o: f32,
    pub highlight: bool
}
impl Clone for SearchTree {
    fn clone(&self) -> Self {
        Self {
            __x: self.__x,
            __y: self.__y,
            __o: self.__o,
            children: self.children.clone(),
            value: self.value,
            stack: self.stack.clone(),
            highlight: self.highlight
        }
    }
}

fn remap(v: f32, imin: f32, imax: f32, omin: f32, omax: f32) -> f32 {
    return omin + (v - imin) * (omax - omin) / (imax - imin);
}
pub const ROOT_C: f32 = 3.0;
pub const ROOT_S: f32 = 0.25;
impl SearchTree {
    pub fn new(value: Parity) -> Self {
        return SearchTree {
            value,
            children: Vec::new(),
            __x: 0.0,
            __y: 0.0,
            __o: 0.0,
            stack: Vec::new(),
            highlight: false
        };
    }
    pub fn highlight_last(from: &mut SearchTree) -> () {
        if from.stack.len() == 0 {
            from.children.index_mut(from.children.len() - 1).highlight = true;
        } else {
            let stack = from.stack.clone();
            let mut target: &mut SearchTree = from;
            for s in stack {
                target = &mut target.children[s];
            }
            target.highlight = true;
        }
    }
    pub fn leaf(from: &mut SearchTree, value: Parity) -> () {
        if from.stack.len() == 0 {
            from.children.push(SearchTree::new(value));
            from.stack.push(0);
        } else {
            let stack = from.stack.clone();
            let mut target: &mut SearchTree = from;
            for s in stack {
                target = &mut target.children[s];
            }
            target.children.push(SearchTree::new(value));
            let len = target.children.len() - 1;
            from.stack.push(len);
        }
    }
    pub fn back(from: &mut SearchTree, kill: bool) -> () {
        let last = from.stack.pop();
        if kill {
            let stack = from.stack.clone();
            let mut target: &mut SearchTree = from;
            for s in stack {
                target = &mut target.children[s];
            }
            target.children.remove(last.unwrap_or(0));
        }
    }

    fn remap_xs(root: &mut SearchTree, imin: f32, imax: f32, omin: f32, omax: f32) {
        root.__x = remap(root.__x, imin, imax, omin, omax);
        for child in &mut root.children {
            SearchTree::remap_xs(child, imin, imax, omin, omax);
        }
    }

    fn find_min(root: &SearchTree) -> f32 {
        let mut minx = root.__x;
        for child in &root.children {
            minx = minx.min(SearchTree::find_min(child));
        }
        return minx;
    }
    fn find_max(root: &SearchTree) -> f32 {
        let mut maxx = root.__x;
        for child in &root.children {
            maxx = maxx.max(SearchTree::find_max(child));
        }
        return maxx;
    }
    fn find_ex(root: &SearchTree) -> (f32, f32) {
        return (SearchTree::find_min(root), SearchTree::find_max(root));
    }

    const CIRCULAR_SCALAR: f32 = 0.1;
    fn set_offsets(root: &mut SearchTree, center: f32) -> () {
        root.__y -= (root.__x - center).abs() * Self::CIRCULAR_SCALAR;
        for child in &mut root.children {
            SearchTree::set_offsets(child, center);
        }
    }
    pub fn display(root: &mut SearchTree, ui: &egui::Ui, painter: &egui::Painter) {
            FancyTreeLayout::layout(root);
            let (min_x, max_x) = SearchTree::find_ex(root);
            let min_allowed_x = 50.0;
            let max_allowed_x = painter.clip_rect().max.x - painter.clip_rect().min.x - min_allowed_x;
            SearchTree::remap_xs(root, min_x, max_x, min_allowed_x, max_allowed_x);
            SearchTree::set_offsets(root, (min_allowed_x + max_allowed_x) / 2.0);
        root.draw_tree_recursive(ui, painter, painter.clip_rect().left_top() + egui::Vec2 { x: 0.0, y: 15.0 });
    }
    fn draw_tree_recursive(&mut self, ui: &egui::Ui, painter: &egui::Painter, parent_location: egui::Pos2) {
        let loc = egui::Pos2 {
            x: parent_location.x + self.__x,
            y: parent_location.y + self.__y + self.__o
        };
        for child in &mut self.children {
            let child_loc = egui::Pos2 {
                x: parent_location.x + child.__x,
                y: parent_location.y + child.__y + child.__o
            };
            painter.line_segment(
                [loc, child_loc], 
                egui::Stroke { width: ROOT_S, color: egui::Color32::WHITE }
            );
            child.draw_tree_recursive(ui, painter, parent_location);
        }

        let color = if self.highlight { egui::Color32::YELLOW } else if self.value == Parity::WHITE { egui::Color32::WHITE } else { egui::Color32::BLACK };
        painter.circle_filled(loc, ROOT_C, color);
        painter.circle_stroke(loc, ROOT_C, egui::Stroke::new(ROOT_S, color));
    }
}
impl Default for SearchTree { fn default() -> Self { Self { value: Parity::NONE, children: Vec::new(), __x: 0.0, __y: 0.0, __o: 0.0, stack: Vec::new(), highlight: false } } }

pub struct FancyTreeLayout;

const HSPACE: f32 = 500.0;
const VSPACE: f32 = 100.0;
impl FancyTreeLayout {
    pub fn layout(root: &mut SearchTree) -> () {
        Self::climb(root, 0.0, VSPACE);
        Self::walk(root, 0.0, HSPACE);
        let mut min = f32::INFINITY;
        Self::check_min(root, 0.0, &mut min);
        if min < 0.0 {
            Self::shift(root, -min);
        }
    }
    fn climb(node: &mut SearchTree, current_y: f32, spacing: f32) -> f32 {
        node.__y = current_y;
        let ny = current_y + spacing;
        for child in &mut node.children {
            Self::climb(child, ny, spacing);
        }
        return ny;
    }
    fn walk(node: &mut SearchTree, current_x: f32, spacing: f32) -> f32 {
        if node.children.is_empty() {
            node.__x = current_x;
            return current_x + spacing;
        }
        let mut leftmost = current_x;
        for child in &mut node.children {
            leftmost = Self::walk(child, leftmost, spacing);
        }
        if node.children.len() > 1 {
            let first = node.children.first().unwrap();
            let last = node.children.last().unwrap();
            node.__x = (first.__x + last.__x) / 2.0;
        }
        return leftmost;
    }
    fn check_min(node: &SearchTree, offset: f32, min: &mut f32) -> () {
        let cx = node.__x + offset;
        *min = cx.min(*min);
        for child in &node.children {
            Self::check_min(child, cx, min);
        }
    }
    fn shift(node: &mut SearchTree, offset: f32) -> () {
        node.__x += offset;
        for child in &mut node.children {
            Self::shift(child, offset);
        }
    }
}

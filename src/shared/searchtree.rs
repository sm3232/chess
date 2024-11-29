use core::f32;
use std::{cell::RefCell, rc::Rc};

use eframe::egui;
use voxell_rng::rng::XorShift32;

use crate::cutil::draw::{BLACK_COLOR_VALUE, BOARD_B_COLOR, BOARD_W_COLOR, WHITE_COLOR_VALUE};

use super::piece::Parity;
pub struct SearchTree {
    pub value: Parity,
    children: Vec<Rc<RefCell<SearchTree>>>,
    __x: RefCell<f32>,
    __y: RefCell<f32>,
    __o: RefCell<f32>
}

fn remap(v: f32, imin: f32, imax: f32, omin: f32, omax: f32) -> f32 {
    return omin + (v - imin) * (omax - omin) / (imax - imin);
}
pub const ROOT_C: f32 = 5.0;
pub const ROOT_S: f32 = 1.0;
impl SearchTree {
    pub fn new(value: Parity) -> Self {
        return SearchTree {
            value,
            children: Vec::new(),
            __x: RefCell::new(0.0),
            __y: RefCell::new(0.0),
            __o: RefCell::new(0.0)
        };
    }
    // pub fn push(&mut self, pushee: Rc<SearchTree>) {
        // self.children.push(pushee);
    // }

    pub fn extend(&mut self, value: Parity) -> Rc<RefCell<SearchTree>> {
        let n = Rc::new(RefCell::new(SearchTree::new(value)));
        self.children.push(n.clone());
        return n.clone();
    }
    // pub fn push(&mut self, v: Parity) -> Rc<SearchTree> {
        // let n = Rc::new(RefCell::new(SearchTree::new(v)));
        // self.children.push(n.clone());
        // return n;
    // }
    pub fn root(value: Parity) -> Rc<RefCell<SearchTree>> {
        return Rc::new(RefCell::new(SearchTree {
            value,
            children: Vec::new(),
            __x: RefCell::new(0.0),
            __y: RefCell::new(0.0),
            __o: RefCell::new(0.0)
        }));

    }
    pub fn remove(&mut self, removee: Rc<RefCell<SearchTree>>) {
        self.children.retain(|f| !Rc::ptr_eq(f, &removee));
    }
    pub fn width(&self) -> usize {
        return self.children.len();
    }

    fn remap_xs(root: &Rc<RefCell<SearchTree>>, imin: f32, imax: f32, omin: f32, omax: f32) {
        let rr = root.borrow_mut();
        let cx = *rr.__x.borrow();
        *rr.__x.borrow_mut() = remap(cx, imin, imax, omin, omax);
        for child in &rr.children {
            SearchTree::remap_xs(child, imin, imax, omin, omax);
        }
    }

    fn find_min(root: &Rc<RefCell<SearchTree>>) -> f32 {
        let r = root.borrow_mut();
        let mut minx = *r.__x.borrow();
        for child in &r.children {
            minx = minx.min(SearchTree::find_min(child));
        }
        return minx;
    }
    fn find_max(root: &Rc<RefCell<SearchTree>>) -> f32 {
        let r = root.borrow_mut();
        let mut maxx = *r.__x.borrow();
        for child in &r.children {
            maxx = maxx.max(SearchTree::find_max(child));
        }
        return maxx;
    }
    fn find_ex(root: &Rc<RefCell<SearchTree>>) -> (f32, f32) {
        return (SearchTree::find_min(root), SearchTree::find_max(root));
    }

    pub fn display(root: &Rc<RefCell<SearchTree>>, ui: &egui::Ui, painter: &egui::Painter) {
        FancyTreeLayout::layout(root);
        let (min_x, max_x) = SearchTree::find_ex(root);
        SearchTree::remap_xs(root, min_x, max_x, 50.0, painter.clip_rect().max.x - painter.clip_rect().min.x - 50.0);
        let rng = XorShift32::default();
        root.borrow().draw_tree_recursive(ui, painter, painter.clip_rect().left_top() + egui::Vec2 { x: 0.0, y: 15.0 }, Rc::new(RefCell::new(rng)), 0);
    }
    fn draw_tree_recursive(&self, ui: &egui::Ui, painter: &egui::Painter, parent_location: egui::Pos2, rand: Rc<RefCell<XorShift32>>, siblings: usize) {

        let loc = egui::Pos2 {
            x: parent_location.x + *self.__x.borrow(), 
            y: parent_location.y + *self.__y.borrow() + *self.__o.borrow()
        };
        for child in &self.children {
            let child_ref = child.borrow();
            
            if *child_ref.__o.borrow() == 0.0 {
                if child_ref.children.is_empty() {
                    *child_ref.__o.borrow_mut() = rand.borrow_mut().next_f32() * siblings as f32;
                }
            }
            let child_loc = egui::Pos2 {
                x: parent_location.x + *child_ref.__x.borrow(),
                y: parent_location.y + *child_ref.__y.borrow() + *child_ref.__o.borrow()
            };

            painter.line_segment(
                [loc, child_loc], 
                egui::Stroke { width: ROOT_S, color: egui::Color32::WHITE }
            );

            child_ref.draw_tree_recursive(ui, painter, parent_location, rand.clone(), self.children.len());
        }

        let color = if self.value == Parity::WHITE { egui::Color32::WHITE } else { egui::Color32::BLACK };
        painter.circle_filled(loc, ROOT_C, color);
        painter.circle_stroke(loc, ROOT_C, egui::Stroke::new(ROOT_S, color));
    }
}

pub struct FancyTreeLayout;

impl FancyTreeLayout {
    pub fn layout(root: &Rc<RefCell<SearchTree>>) -> () {
        const HSPACE: f32 = 50.0;
        const VSPACE: f32 = 100.0;
        Self::climb(root, 0.0, VSPACE);
        Self::walk(root, 0.0, HSPACE);
        let mut min = f32::INFINITY;
        Self::check_min(root, 0.0, &mut min);
        if min < 0.0 {
            Self::shift(root, -min);
        }
    }
    fn climb(node: &Rc<RefCell<SearchTree>>, current_y: f32, spacing: f32) -> f32 {
        let node_ref = node.borrow_mut();
        *node_ref.__y.borrow_mut() = current_y;
        let ny = current_y + spacing;
        for child in &node_ref.children {
            Self::climb(child, ny, spacing);
        }
        return ny;
    }
    fn walk(node: &Rc<RefCell<SearchTree>>, current_x: f32, spacing: f32) -> f32 {
        let node_ref = node.borrow_mut();
        if node_ref.children.is_empty() {
            *node_ref.__x.borrow_mut() = current_x;
            return current_x + spacing;
        }
        let mut leftmost = current_x;
        for child in &node_ref.children {
            leftmost = Self::walk(child, leftmost, spacing);
        }
        if !node_ref.children.is_empty() {
            let first = node_ref.children.first().unwrap().borrow();
            let last = node_ref.children.last().unwrap().borrow();
            *node_ref.__x.borrow_mut() = (*first.__x.borrow() + *last.__x.borrow()) / 2.0;
        }
        return leftmost;
    }
    fn check_min(node: &Rc<RefCell<SearchTree>>, offset: f32, min: &mut f32) -> () {
        let node_ref = node.borrow_mut();
        let cx = *node_ref.__x.borrow() + offset;
        *min = cx.min(*min);
        for child in &node_ref.children {
            Self::check_min(child, cx, min);
        }
    }
    fn shift(node: &Rc<RefCell<SearchTree>>, offset: f32) -> () {
        let node_ref = node.borrow_mut();
        *node_ref.__x.borrow_mut() += offset;
        for child in &node_ref.children {
            Self::shift(child, offset);
        }
    }
    /*
    fn walk(&self, tree: &Rc<RefCell<SearchTree>>) -> f32 {
        let mut tree_ref = tree.borrow_mut();

        if tree_ref.children.is_empty() {
            tree_ref.__x = 0.0;
            return 0.0;
        }

        let mut fcs = 0.0;
        let mut prev: Option<&Rc<RefCell<SearchTree>>> = None;

        for child in &tree_ref.children {
            let _lsm = if let Some(pre) = prev {
                let childx = self.walk(child);
                let prevx = self.walk(pre);
                let modsum = childx - prevx;
                fcs += modsum;
                modsum
            } else {
                0.0
            };
            let mut child_ref = child.borrow_mut();
            child_ref.__x = fcs;
            prev = Some(child);
        }
        let cw = tree_ref.children.len() as f32 * self.x;
        tree_ref.__x = cw / 2.0;
        return fcs;
    }
    fn walk_again(&self, tree: &Rc<RefCell<SearchTree>>, depth: f32, modifier: f32) -> () {
        let mut tree_ref = tree.borrow_mut();
        tree_ref.__x += modifier;
        tree_ref.__y = depth * self.y;
        drop(tree_ref);
        for child in &tree.borrow().children {
            self.walk_again(child, depth + 1.0, modifier);
        }
    }

    pub fn layout(&self, root: &Rc<RefCell<SearchTree>>) -> () {
        let modi = self.walk(root);
        self.walk_again(root, 0.0, modi);
    }
    */
}

use core::f32;
use std::{cell::RefCell, rc::Rc, sync::{Arc, Mutex}};

use eframe::egui;
use voxell_rng::rng::XorShift32;


use super::piece::Parity;
pub struct SearchTree {
    pub value: Parity,
    children: Vec<Arc<Mutex<SearchTree>>>,
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

    pub fn extend(&mut self, value: Parity) -> Arc<Mutex<SearchTree>> {
        let n = Arc::new(Mutex::new(SearchTree::new(value)));
        self.children.push(n.clone());
        return n.clone();
    }
    // pub fn push(&mut self, v: Parity) -> Rc<SearchTree> {
        // let n = Arc::new(Mutex::new(SearchTree::new(v)));
        // self.children.push(n.clone());
        // return n;
    // }
    pub fn root(value: Parity) -> Arc<Mutex<SearchTree>> {
        return Arc::new(Mutex::new(SearchTree {
            value,
            children: Vec::new(),
            __x: RefCell::new(0.0),
            __y: RefCell::new(0.0),
            __o: RefCell::new(0.0)
        }));

    }
    pub fn remove(&mut self, removee: Arc<Mutex<SearchTree>>) {
        self.children.retain(|f| !Arc::ptr_eq(f, &removee));
    }
    pub fn width(&self) -> usize {
        return self.children.len();
    }

    fn remap_xs(root: &Arc<Mutex<SearchTree>>, imin: f32, imax: f32, omin: f32, omax: f32) {
        let rr = root.lock().unwrap();
        let cx = *rr.__x.borrow();
        *rr.__x.borrow_mut() = remap(cx, imin, imax, omin, omax);
        for child in &rr.children {
            SearchTree::remap_xs(child, imin, imax, omin, omax);
        }
        drop(rr);
    }

    fn find_min(root: &Arc<Mutex<SearchTree>>) -> f32 {
        let rr = root.lock().unwrap();
        let mut minx = *rr.__x.borrow();
        for child in &rr.children {
            minx = minx.min(SearchTree::find_min(child));
        }
        drop(rr);
        return minx;
    }
    fn find_max(root: &Arc<Mutex<SearchTree>>) -> f32 {
        let rr = root.lock().unwrap();
        let mut maxx = *rr.__x.borrow();
        for child in &rr.children {
            maxx = maxx.max(SearchTree::find_max(child));
        }
        drop(rr);
        return maxx;
    }
    fn find_ex(root: &Arc<Mutex<SearchTree>>) -> (f32, f32) {
        return (SearchTree::find_min(root), SearchTree::find_max(root));
    }

    pub fn display(root: &Arc<Mutex<SearchTree>>, ui: &egui::Ui, painter: &egui::Painter) {
        FancyTreeLayout::layout(root);
        let (min_x, max_x) = SearchTree::find_ex(root);
        SearchTree::remap_xs(root, min_x, max_x, 50.0, painter.clip_rect().max.x - painter.clip_rect().min.x - 50.0);
        let rng = XorShift32::default();
        root.lock().unwrap().draw_tree_recursive(ui, painter, painter.clip_rect().left_top() + egui::Vec2 { x: 0.0, y: 15.0 }, Arc::new(Mutex::new(rng)), 0);
    }
    fn draw_tree_recursive(&self, ui: &egui::Ui, painter: &egui::Painter, parent_location: egui::Pos2, rand: Arc<Mutex<XorShift32>>, siblings: usize) {

        let loc = egui::Pos2 {
            x: parent_location.x + *self.__x.borrow(), 
            y: parent_location.y + *self.__y.borrow() + *self.__o.borrow()
        };
        for child in &self.children {
            let child_ref = child.lock().unwrap();
            
            if *child_ref.__o.borrow() == 0.0 {
                if child_ref.children.is_empty() {
                    *child_ref.__o.borrow_mut() = rand.lock().unwrap().next_f32() * siblings as f32;
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
    pub fn layout(root: &Arc<Mutex<SearchTree>>) -> () {
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
    fn climb(node: &Arc<Mutex<SearchTree>>, current_y: f32, spacing: f32) -> f32 {
        let node_ref = node.lock().unwrap();
        *node_ref.__y.borrow_mut() = current_y;
        let ny = current_y + spacing;
        for child in &node_ref.children {
            Self::climb(child, ny, spacing);
        }
        drop(node_ref);
        return ny;
    }
    fn walk(node: &Arc<Mutex<SearchTree>>, current_x: f32, spacing: f32) -> f32 {
        let node_ref = node.lock().unwrap();
        if node_ref.children.is_empty() {
            *node_ref.__x.borrow_mut() = current_x;
            drop(node_ref);
            return current_x + spacing;
        }
        let mut leftmost = current_x;
        for child in &node_ref.children {
            leftmost = Self::walk(child, leftmost, spacing);
        }
        if node_ref.children.len() > 1 {
            let first = node_ref.children.first().unwrap().lock().unwrap();
            let last = node_ref.children.last().unwrap().lock().unwrap();
            *node_ref.__x.borrow_mut() = (*first.__x.borrow() + *last.__x.borrow()) / 2.0;
            drop(last);
            drop(first);
        }
        drop(node_ref);
        println!("{current_x}");
        return leftmost;
    }
    fn check_min(node: &Arc<Mutex<SearchTree>>, offset: f32, min: &mut f32) -> () {
        let node_ref = node.lock().unwrap();
        let cx = *node_ref.__x.borrow() + offset;
        *min = cx.min(*min);
        for child in &node_ref.children {
            Self::check_min(child, cx, min);
        }
        drop(node_ref);
    }
    fn shift(node: &Arc<Mutex<SearchTree>>, offset: f32) -> () {
        let node_ref = node.lock().unwrap();
        *node_ref.__x.borrow_mut() += offset;
        for child in &node_ref.children {
            Self::shift(child, offset);
        }
        drop(node_ref);
    }
    /*
    fn walk(&self, tree: &Arc<Mutex<SearchTree>>) -> f32 {
        let mut tree_ref = tree.borrow_mut();

        if tree_ref.children.is_empty() {
            tree_ref.__x = 0.0;
            return 0.0;
        }

        let mut fcs = 0.0;
        let mut prev: Option<&Arc<Mutex<SearchTree>>> = None;

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
    fn walk_again(&self, tree: &Arc<Mutex<SearchTree>>, depth: f32, modifier: f32) -> () {
        let mut tree_ref = tree.borrow_mut();
        tree_ref.__x += modifier;
        tree_ref.__y = depth * self.y;
        drop(tree_ref);
        for child in &tree.borrow().children {
            self.walk_again(child, depth + 1.0, modifier);
        }
    }

    pub fn layout(&self, root: &Arc<Mutex<SearchTree>>) -> () {
        let modi = self.walk(root);
        self.walk_again(root, 0.0, modi);
    }
    */
}

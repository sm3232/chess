use core::f32;
use std::{cell::RefCell, ops::{Deref, DerefMut}, rc::Rc, sync::{Arc, Mutex, MutexGuard}};

use eframe::egui;
use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};
use rand_distr::{Distribution, Normal};
use voxell_rng::rng::XorShift32;


use super::piece::Parity;
#[derive(Debug)]
pub struct SearchTree {
    pub value: Parity,
    pub children: Vec<Arc<Mutex<SearchTree>>>,
    __x: RefCell<f32>,
    __y: RefCell<f32>,
    __o: RefCell<f32>
}

fn remap(v: f32, imin: f32, imax: f32, omin: f32, omax: f32) -> f32 {
    return omin + (v - imin) * (omax - omin) / (imax - imin);
}
pub const ROOT_C: f32 = 3.0;
pub const ROOT_S: f32 = 0.25;
impl SearchTree {
    pub fn placeholder(x: f32, y: f32) -> Self {
        Self {
            __x: RefCell::new(x),
            __y: RefCell::new(y),
            __o: RefCell::new(0.0),
            children: Vec::new(),
            value: Parity::NONE
        }
    }
    pub fn kinda_clone_children(children: &Vec<Arc<Mutex<SearchTree>>>) -> Vec<Arc<Mutex<SearchTree>>> {
        let mut v = Vec::new();
        for child in children {
            let cl = child.lock().unwrap();
            v.push(Arc::new(Mutex::new(
                SearchTree::absolutely_cloned(&cl)
            )));
        }
        return v;
    }
    pub fn absolutely_cloned(from: &MutexGuard<'_, Self>) -> Self {
        let r = Self {
            children: SearchTree::kinda_clone_children(&from.children),
            __x: RefCell::new(*from.__x.borrow()),
            __y: RefCell::new(*from.__y.borrow()),
            __o: RefCell::new(*from.__o.borrow()),
            value: from.value
        };
        return r;
    }
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

    pub fn display_nobs(root: &SearchTree, ui: &egui::Ui, painter: &egui::Painter) {
        StaticTreeLayout::layout(root);
        let mut minx = *root.__x.borrow();
        let mut maxx = *root.__x.borrow();
        for child in &root.children {
            minx = minx.min(SearchTree::find_min(child));
            maxx = maxx.max(SearchTree::find_max(child));
        }
        let cx = *root.__x.borrow();
        *root.__x.borrow_mut() = remap(cx, minx, maxx, 50.0, painter.clip_rect().max.x - painter.clip_rect().min.x - 50.0);
        for child in &root.children {
            SearchTree::remap_xs(child, minx, maxx, 50.0, painter.clip_rect().max.x - painter.clip_rect().min.x - 50.0);
        }
        root.draw_tree_recursive_raw(ui, painter, painter.clip_rect().left_top() + egui::Vec2 { x: 0.0, y: 15.0 });
    }
    pub fn display(root: &Arc<Mutex<SearchTree>>, ui: &egui::Ui, painter: &egui::Painter) {
        FancyTreeLayout::layout(root);
        let (min_x, max_x) = SearchTree::find_ex(root);
        SearchTree::remap_xs(root, min_x, max_x, 50.0, painter.clip_rect().max.x - painter.clip_rect().min.x - 50.0);
        let cha = ChaCha8Rng::from_entropy();
        let rand = Normal::new(0.0, 10.0).ok().unwrap();
        root.lock().unwrap().draw_tree_recursive(ui, painter, painter.clip_rect().left_top() + egui::Vec2 { x: 0.0, y: 15.0 }, Arc::new(rand), Arc::new(Mutex::new(cha)), 1);
    }
    fn draw_tree_recursive_raw(&self, ui: &egui::Ui, painter: &egui::Painter, parent_location: egui::Pos2) {
        let loc = egui::Pos2 {
            x: parent_location.x + *self.__x.borrow(), 
            y: parent_location.y + *self.__y.borrow() + *self.__o.borrow()
        };
        for child in &self.children {
            let child_ref = child.lock().unwrap();
            
            let child_loc = egui::Pos2 {
                x: parent_location.x + *child_ref.__x.borrow(),
                y: parent_location.y + *child_ref.__y.borrow() + *child_ref.__o.borrow()
            };

            // if !child_ref.children.is_empty() {
                if self.value != Parity::NONE {
                    painter.line_segment(
                        [loc, child_loc], 
                        egui::Stroke { width: ROOT_S, color: egui::Color32::WHITE }
                    );
                }
                child_ref.draw_tree_recursive_raw(ui, painter, parent_location);
            // }
        }

        if self.value != Parity::NONE {
            let color = if self.value == Parity::WHITE { egui::Color32::WHITE } else { egui::Color32::BLACK };
            painter.circle_filled(loc, ROOT_C, color);
            painter.circle_stroke(loc, ROOT_C, egui::Stroke::new(ROOT_S, color));
        }
    
    }
    fn draw_tree_recursive(&self, ui: &egui::Ui, painter: &egui::Painter, parent_location: egui::Pos2, rand: Arc<Normal<f32>>, cha: Arc<Mutex<ChaCha8Rng>>, depth: usize) {
        let loc = egui::Pos2 {
            x: parent_location.x + *self.__x.borrow(), 
            y: parent_location.y + *self.__y.borrow() + *self.__o.borrow()
        };
        for child in &self.children {
            let child_ref = child.lock().unwrap();
            
            let mut lock = cha.lock().unwrap();
            if *child_ref.__o.borrow() == 0.0 {
                if child_ref.children.is_empty() {

                    *child_ref.__o.borrow_mut() = rand.sample(lock.deref_mut()) * depth as f32;

                }
            }
            drop(lock);
            let child_loc = egui::Pos2 {
                x: parent_location.x + *child_ref.__x.borrow(),
                y: parent_location.y + *child_ref.__y.borrow() + *child_ref.__o.borrow()
            };


            // if !child_ref.children.is_empty() {
                painter.line_segment(
                    [loc, child_loc], 
                    egui::Stroke { width: ROOT_S, color: egui::Color32::WHITE }
                );
                child_ref.draw_tree_recursive(ui, painter, parent_location, rand.clone(), cha.clone(), depth + 1);

            // }
        }

        let color = if self.value == Parity::WHITE { egui::Color32::WHITE } else { egui::Color32::BLACK };
        painter.circle_filled(loc, ROOT_C, color);
        painter.circle_stroke(loc, ROOT_C, egui::Stroke::new(ROOT_S, color));
    }
}

pub struct FancyTreeLayout;
pub struct StaticTreeLayout;

const HSPACE: f32 = 500.0;
const VSPACE: f32 = 100.0;
impl StaticTreeLayout {
    pub fn layout(root: &SearchTree) -> () {
        Self::climb(root, 0.0, VSPACE);
        Self::walk(root, 0.0, HSPACE);
        let mut min = f32::INFINITY;
        Self::check_min(root, 0.0, &mut min);
        if min < 0.0 {
            Self::shift(root, -min);
        }
    }
    fn climb(node: &SearchTree, current_y: f32, spacing: f32) -> f32 {
        *node.__y.borrow_mut() = current_y;
        let ny = current_y + spacing;
        for child in &node.children {
            FancyTreeLayout::climb(child, ny, spacing);
        }
        return ny;
    }
    fn walk(node: &SearchTree, current_x: f32, spacing: f32) -> f32 {
        if node.children.is_empty() {
            *node.__x.borrow_mut() = current_x;
            return current_x + spacing;
        }
        let mut leftmost = current_x;
        for child in &node.children {
            leftmost = FancyTreeLayout::walk(child, leftmost, spacing);
        }
        if node.children.len() > 1 {
            let first = node.children.first().unwrap().lock().unwrap();
            let last = node.children.last().unwrap().lock().unwrap();
            *node.__x.borrow_mut() = (*first.__x.borrow() + *last.__x.borrow()) / 2.0;
            drop(last);
            drop(first);
        }
        return leftmost;
    }
    fn check_min(node: &SearchTree, offset: f32, min: &mut f32) -> () {
        let cx = *node.__x.borrow() + offset;
        *min = cx.min(*min);
        for child in &node.children {
            FancyTreeLayout::check_min(child, cx, min);
        }
    }
    fn shift(node: &SearchTree, offset: f32) -> () {
        *node.__x.borrow_mut() += offset;
        for child in &node.children {
            FancyTreeLayout::shift(child, offset);
        }
    }
}

impl FancyTreeLayout {
    pub fn layout(root: &Arc<Mutex<SearchTree>>) -> () {
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
}

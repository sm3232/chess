use super::motion::Motion;

#[derive(Copy, Debug)]
pub struct EvaluatedMotion {
    pub evaluation: i32,
    pub motion: Motion,
    pub key: u64
}
impl Default for EvaluatedMotion { fn default() -> Self { Self { evaluation: 0, key: 0, motion: Motion::default() } } }
impl Clone for EvaluatedMotion {
    fn clone(&self) -> Self {
        Self {
            evaluation: self.evaluation,
            motion: self.motion,
            key: 0,
        }
    }
}
impl std::fmt::Display for EvaluatedMotion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{} -> {}, ({})", self.motion.from, self.motion.to, self.evaluation);
    }
}
impl PartialEq for EvaluatedMotion {
    fn eq(&self, other: &EvaluatedMotion) -> bool { self.evaluation == other.evaluation }
}
impl Eq for EvaluatedMotion {}
impl PartialOrd for EvaluatedMotion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for EvaluatedMotion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.evaluation.cmp(&other.evaluation) }
}


type Species = EvaluatedMotion;

pub struct Heap {
    pub store: Vec<Species>,
    pub size: usize
}
impl Default for Heap {
    fn default() -> Self { Self { store: Vec::new(), size: 0 } }
}

impl std::ops::Index<usize> for Heap {
    type Output = Species;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output { &self.store[index] }
}

impl std::ops::IndexMut<usize> for Heap {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output { &mut self.store[index] }
}

impl Heap {

    #[inline(always)]
    pub fn parent(i: usize) -> usize { (i - 1) / 2 }

    #[inline(always)]
    pub fn left(i: usize) -> usize { (2 * i) + 1 }
    
    #[inline(always)]
    pub fn right(i: usize) -> usize { (2 * i) + 2 }

    pub fn to_sorted_evaluated_motions(&self) -> Vec<EvaluatedMotion> {
        let mut heap = self.store.clone();
        let mut v: Vec<EvaluatedMotion> = Vec::with_capacity(self.store.len());
        while !heap.is_empty() {
            v.push(heap.pop().unwrap());
        }
        return v;
    }
    pub fn to_sorted_motions(&self) -> Vec<Motion> {
        let mut heap = self.store.clone();
        let mut v: Vec<Motion> = Vec::with_capacity(self.store.len());
        while !heap.is_empty() {
            v.push(heap.pop().unwrap().motion);
        }
        return v;
    }

    pub fn bubble(&mut self, mut i: usize) -> () {
        while i > 0 {
            let p = Heap::parent(i);
            if self[p] >= self[i] { break };
            self.store.swap(p, i);
            i = p;
        }
    }
    pub fn empty(&self) -> bool { self.size == 0 }

    pub fn sift(&mut self, i: usize) -> () {
        let mut max = i;
        let l = Heap::left(i);
        if l < self.size && self[l] > self[max] {
            max = l;
        }
        let r = Heap::right(i);
        if r < self.size && self[r] > self[max] {
            max = r;
        }
        if i != max {
            self.store.swap(i, max);
            self.sift(max);
        }
    }

    pub fn clear(&mut self) -> () {
        self.store.clear();
        self.size = 0;
    }

    pub fn push(&mut self, v: Species) -> () {
        self.size += 1;
        self.store.push(v);
        self.bubble(self.size - 1);
    }

    pub fn peek(&self) -> Species { self[0] }

    pub fn pop(&mut self) -> Species {
        if self.store.is_empty() {
            println!("Heap is empty! Uh oh");
        }
        let r = self.store.swap_remove(0);
        self.size -= 1;
        self.sift(0);
        return r;
    }
}

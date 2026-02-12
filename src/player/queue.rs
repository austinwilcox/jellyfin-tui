use crate::client::models::Item;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn next(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RepeatMode::Off => "Off",
            RepeatMode::All => "All",
            RepeatMode::One => "One",
        }
    }
}

#[derive(Debug)]
pub struct Queue {
    pub items: Vec<Item>,
    pub current: Option<usize>,
    pub repeat: RepeatMode,
    pub shuffle: bool,
    shuffle_order: Vec<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current: None,
            repeat: RepeatMode::Off,
            shuffle: false,
            shuffle_order: Vec::new(),
        }
    }

    pub fn current_item(&self) -> Option<&Item> {
        self.current.and_then(|i| self.items.get(i))
    }

    pub fn replace(&mut self, items: Vec<Item>, start_index: usize) {
        self.items = items;
        self.current = if self.items.is_empty() {
            None
        } else {
            Some(start_index.min(self.items.len().saturating_sub(1)))
        };
        self.rebuild_shuffle();
    }

    pub fn enqueue(&mut self, item: Item) {
        self.items.push(item);
        if self.current.is_none() {
            self.current = Some(0);
        }
        self.rebuild_shuffle();
    }

    #[allow(dead_code)]
    pub fn enqueue_many(&mut self, items: Vec<Item>) {
        let was_empty = self.items.is_empty();
        self.items.extend(items);
        if was_empty && !self.items.is_empty() {
            self.current = Some(0);
        }
        self.rebuild_shuffle();
    }

    pub fn next(&mut self) -> Option<&Item> {
        if self.items.is_empty() {
            return None;
        }

        match self.repeat {
            RepeatMode::One => {
                // Stay on current
            }
            _ => {
                if let Some(cur) = self.current {
                    let next = if self.shuffle {
                        self.next_shuffle_index(cur)
                    } else {
                        cur + 1
                    };

                    if next >= self.items.len() {
                        match self.repeat {
                            RepeatMode::All => self.current = Some(0),
                            _ => {
                                self.current = None;
                                return None;
                            }
                        }
                    } else {
                        self.current = Some(next);
                    }
                } else {
                    self.current = Some(0);
                }
            }
        }

        self.current_item()
    }

    pub fn prev(&mut self) -> Option<&Item> {
        if self.items.is_empty() {
            return None;
        }

        if let Some(cur) = self.current {
            if cur == 0 {
                if self.repeat == RepeatMode::All {
                    self.current = Some(self.items.len() - 1);
                }
            } else {
                self.current = Some(cur - 1);
            }
        } else {
            self.current = Some(0);
        }

        self.current_item()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current = None;
        self.shuffle_order.clear();
    }

    pub fn remove(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.items.remove(index);
        if let Some(cur) = self.current {
            if index < cur {
                self.current = Some(cur - 1);
            } else if index == cur {
                if cur >= self.items.len() {
                    self.current = if self.items.is_empty() {
                        None
                    } else {
                        Some(self.items.len() - 1)
                    };
                }
            }
        }
        self.rebuild_shuffle();
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        if self.shuffle {
            self.rebuild_shuffle();
        }
    }

    fn rebuild_shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        self.shuffle_order = (0..self.items.len()).collect();
        self.shuffle_order.shuffle(&mut rng);
    }

    fn next_shuffle_index(&self, current: usize) -> usize {
        if let Some(pos) = self.shuffle_order.iter().position(|&x| x == current) {
            if pos + 1 < self.shuffle_order.len() {
                self.shuffle_order[pos + 1]
            } else {
                self.items.len() // Signal end
            }
        } else if !self.shuffle_order.is_empty() {
            self.shuffle_order[0]
        } else {
            self.items.len()
        }
    }
}

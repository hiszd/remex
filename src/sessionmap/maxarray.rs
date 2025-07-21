/// An array that has a capacity. If an item is added to the array when it is full, it will remove
/// the oldest item in the array and return it.
pub struct MaxArray<T, const S: usize> {
  items: Vec<T>,
}

impl<T, const S: usize> MaxArray<T, S> {
  pub fn new() -> Self {
    MaxArray {
      items: Vec::<T>::with_capacity(S),
    }
  }

  pub fn push(&mut self, item: T) {
    if self.items.len() == S {
      self.items.remove(0);
    }
    self.items.push(item);
  }

  pub fn pop(&mut self) -> Option<T> {
    match self.items.len() {
      0 => None,
      _ => Some(self.items.remove(0)),
    }
  }

  pub fn len(&self) -> usize { self.items.len() }
}

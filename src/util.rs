use std::{fmt::Debug, hash::Hash};

pub trait Domain: Debug + Eq + Ord + Clone + Hash + From<char> + Into<char> {
  fn separator() -> Self;
}
impl Domain for char {
  fn separator() -> Self {
    '#'
  }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum CharWrap {
  Char(char),
  Separator,
}
impl From<char> for CharWrap {
  fn from(a: char) -> Self {
    if a == char::separator() {
      CharWrap::Separator
    } else {
      CharWrap::Char(a)
    }
  }
}
impl Into<char> for CharWrap {
  fn into(self) -> char {
    match self {
      CharWrap::Char(a) => a,
      CharWrap::Separator => char::separator(),
    }
  }
}
impl Default for CharWrap {
  fn default() -> Self {
    CharWrap::Char(char::default())
  }
}
impl Domain for CharWrap {
  fn separator() -> Self {
    CharWrap::Separator
  }
}

pub(crate) mod extention {
  use std::{
    collections::{BTreeMap, HashMap, HashSet},
    default::Default,
    hash::Hash,
    iter::Extend,
  };

  pub(crate) trait MultiMap {
    type Key: Eq + Hash;
    type Value;

    fn insert_with_check(&mut self, key: Self::Key, values: impl IntoIterator<Item = Self::Value>);

    fn merge(&mut self, other: Self);
  }
  impl<K, V, Collection> MultiMap for HashMap<K, Collection>
  where
    K: Eq + Hash,
    Collection: IntoIterator<Item = V> + Extend<V> + Default,
  {
    type Key = K;
    type Value = V;

    fn insert_with_check(&mut self, key: Self::Key, values: impl IntoIterator<Item = Self::Value>) {
      let vec = self.entry(key).or_default();
      vec.extend(values);
    }

    fn merge(&mut self, other: Self) {
      for (key, values_) in other.into_iter() {
        let values = self.entry(key).or_default();
        values.extend(values_);
      }
    }
  }
  impl<K, V, Collection> MultiMap for BTreeMap<K, Collection>
  where
    K: Eq + Hash + Ord,
    Collection: IntoIterator<Item = V> + Extend<V> + Default,
  {
    type Key = K;
    type Value = V;

    fn insert_with_check(&mut self, key: Self::Key, values: impl IntoIterator<Item = Self::Value>) {
      let vec = self.entry(key).or_default();
      vec.extend(values);
    }

    fn merge(&mut self, other: Self) {
      for (key, values_) in other.into_iter() {
        let values = self.entry(key).or_default();
        values.extend(values_);
      }
    }
  }

  pub(crate) trait ImmutableValueMap {
    type Key: Eq + Hash;
    type Value;

    fn safe_insert(&mut self, key: Self::Key, value: Self::Value);
  }
  impl<K: Eq + Hash, V> ImmutableValueMap for HashMap<K, V> {
    type Key = K;
    type Value = V;

    fn safe_insert(&mut self, key: Self::Key, value: Self::Value) {
      assert!(self.insert(key, value).is_none());
    }
  }

  pub(crate) trait HashSetExt: std::marker::Sized {
    /** expensive method */
    fn subsets(&self) -> Vec<Self>;
  }
  impl<V: Clone + Hash + Eq> HashSetExt for HashSet<V> {
    fn subsets(&self) -> Vec<Self> {
      use std::convert::TryInto;
      let mut subsets = vec![];

      for i in 0..2u64.pow(self.len().try_into().unwrap()) {
        let mut subset = HashSet::new();
        self.iter().enumerate().for_each(|(idx, v)| {
          if i & (1 << idx) != 0 {
            subset.insert(v.clone());
          }
        });

        subsets.push(subset);
      }

      subsets
    }
  }
}

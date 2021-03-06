use super::{recognizable::Recognizable, symbolic_automata::Sfa};
use crate::{
  boolean_algebra::{BoolAlg, Predicate},
  smt2,
  state::{State, StateMachine},
  util::Domain,
};
use smt2parser::concrete::{Constant, Term};
use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
  hash::Hash,
};

pub fn convert<D: Domain>(reg: Regex<D>) -> Regex<char> {
  match reg {
    Regex::Empty => Regex::Empty,
    Regex::Epsilon => Regex::Epsilon,
    Regex::All => Regex::All,
    Regex::Element(e) => Regex::Element(e.into()),
    Regex::Range(l, r) => Regex::Range(l.map(|e| e.into()), r.map(|e| e.into())),
    Regex::Concat(vec) => Regex::Concat(vec.into_iter().map(|r| convert(r)).collect()),
    Regex::Or(vec) => Regex::Or(vec.into_iter().map(|r| convert(r)).collect()),
    Regex::Inter(vec) => Regex::Inter(vec.into_iter().map(|r| convert(r)).collect()),
    Regex::Star(reg) => Regex::Star(Box::new(convert(*reg))),
    Regex::Plus(reg) => Regex::Plus(Box::new(convert(*reg))),
    Regex::Not(reg) => Regex::Not(Box::new(convert(*reg))),
  }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Regex<T: PartialOrd> {
  Empty,
  Epsilon,
  All,
  Element(T),
  Range(Option<T>, Option<T>),
  Concat(Vec<Self>),
  Or(Vec<Self>),
  Inter(Vec<Self>),
  Star(Box<Self>),
  Plus(Box<Self>),
  Not(Box<Self>),
}
impl<T: Domain> Regex<T> {
  pub fn empty() -> Self {
    Regex::Empty
  }

  pub fn epsilon() -> Self {
    Regex::Epsilon
  }

  pub fn all() -> Self {
    Regex::All
  }

  pub fn element(c: char) -> Self {
    Regex::Element(T::from(c))
  }

  pub fn seq(s: &str) -> Self {
    s.chars()
      .map(|c| Regex::Element(T::from(c)))
      .reduce(|reg, el| reg.concat(el))
      .unwrap_or(Regex::Epsilon)
  }

  pub fn range(start: Option<char>, end: Option<char>) -> Self {
    if start.is_none() && end.is_none() {
      Regex::Empty
    } else if start
      .as_ref()
      .and_then(|l| end.as_ref().and_then(|r| Some(*l == *r)))
      .unwrap_or(false)
    {
      Regex::Element(T::from(start.unwrap()))
    } else {
      Regex::Range(start.map(|c| T::from(c)), end.map(|c| T::from(c)))
    }
  }

  pub fn concat(self, other: Regex<T>) -> Self {
    match (self, other) {
      (Regex::Empty, _) | (_, Regex::Empty) => Regex::Empty,
      (Regex::Epsilon, r) | (r, Regex::Epsilon) => r,
      (Regex::Concat(mut v1), Regex::Concat(v2)) => {
        v1.extend(v2);
        Regex::Concat(v1)
      }
      (r, Regex::Concat(v_)) => {
        let mut v = vec![r];
        v.extend(v_);
        Regex::Concat(v)
      }
      (Regex::Concat(mut v), r) => {
        v.push(r);
        Regex::Concat(v)
      }
      (left, right) => Regex::Concat(vec![left, right]),
    }
  }

  pub fn or(self, other: Regex<T>) -> Self {
    match (self, other) {
      (Regex::Empty, r) | (r, Regex::Empty) => r,
      (Regex::Epsilon, Regex::Star(r)) | (Regex::Star(r), Regex::Epsilon) => Regex::Star(r),
      (Regex::Or(mut v1), Regex::Or(v2)) => {
        for r in v2 {
          if !v1.contains(&r) {
            v1.push(r);
          }
        }
        v1.sort();
        Regex::Or(v1)
      }
      (Regex::Or(mut v), r) | (r, Regex::Or(mut v)) => {
        if !v.contains(&r) {
          v.push(r);
          v.sort();
        }
        Regex::Or(v)
      }
      (left, right) => {
        if left == right {
          left
        } else {
          Regex::Or(vec![left, right])
        }
      }
    }
  }

  pub fn inter(self, other: Regex<T>) -> Self {
    match (self, other) {
      (Regex::Epsilon, Regex::Star(_))
      | (Regex::Star(_), Regex::Epsilon)
      | (Regex::Epsilon, Regex::Epsilon) => Regex::Epsilon,
      (Regex::Empty, _) | (_, Regex::Empty) | (Regex::Epsilon, _) | (_, Regex::Epsilon) => {
        Regex::Empty
      }
      (Regex::Inter(mut v1), Regex::Inter(v2)) => {
        for r in v2 {
          if !v1.contains(&r) {
            v1.push(r);
          }
        }
        v1.sort();
        Regex::Inter(v1)
      }
      (Regex::Inter(mut v), r) | (r, Regex::Inter(mut v)) => {
        if !v.contains(&r) {
          v.push(r);
          v.sort();
        }
        Regex::Inter(v)
      }
      (left, right) => {
        if left == right {
          left
        } else {
          Regex::Inter(vec![left, right])
        }
      }
    }
  }

  pub fn star(self) -> Self {
    if let Regex::Empty = self {
      Regex::Epsilon
    } else if let Regex::Epsilon = self {
      Regex::Epsilon
    } else if let Regex::Star(r) = self {
      *r
    } else {
      Regex::Star(Box::new(self))
    }
  }

  pub fn plus(self) -> Self {
    Regex::Plus(Box::new(self))
  }

  pub fn not(self) -> Self {
    if let Regex::Empty = self {
      Regex::all().star()
    } else if let Regex::Not(r) = self {
      *r
    } else {
      Regex::Not(Box::new(self))
    }
  }

  /** with, thompson  --- clushkul, partial derivative */
  pub fn to_sfa<S: State>(self) -> Sfa<T, S> {
    match self {
      Regex::Empty => Sfa::empty(),
      Regex::Epsilon => super::macros::sfa! {
        { initial },
        { -> initial },
        { initial }
      },
      Regex::Element(a) => super::macros::sfa! {
        { initial, final_state },
        {
          -> initial,
          (initial, Predicate::char(a)) -> [final_state]
        },
        { final_state }
      },
      Regex::All => super::macros::sfa! {
        { initial, final_state },
        {
          -> initial,
          (initial, Predicate::all_char()) -> [final_state]
        },
        { final_state }
      },
      Regex::Range(left, right) => super::macros::sfa! {
        { initial, final_state },
        {
          -> initial,
          (initial, Predicate::range(left, right)) -> [final_state]
        },
        { final_state }
      },
      Regex::Concat(v) => v
        .into_iter()
        .map(|r| r.to_sfa())
        .reduce(|res, sfa| res.concat(sfa))
        .unwrap_or(Sfa::empty()),
      Regex::Or(v) => v
        .into_iter()
        .map(|r| r.to_sfa())
        .reduce(|res, sfa| res.or(sfa))
        .unwrap_or(Sfa::empty()),
      Regex::Inter(v) => v
        .into_iter()
        .map(|r| r.to_sfa())
        .reduce(|res, sfa| res.inter(sfa))
        .unwrap_or(Sfa::empty()),
      Regex::Star(r) => r.to_sfa().star(),
      Regex::Plus(r) => r.to_sfa().plus(),
      Regex::Not(r) => r.to_sfa().not(),
    }
  }

  pub fn new(term: &Term) -> Self {
    match term {
      Term::Application {
        qual_identifier,
        arguments,
      } => match &smt2::get_symbol(qual_identifier)[..] {
        "str.to.re" => {
          if let [term] = &arguments[..] {
            if let Term::Constant(Constant::String(s)) = term {
              s.chars().fold(Regex::Epsilon, |reg, c| {
                reg.concat(Regex::Element(T::from(c)))
              })
            } else {
              panic!("Syntax Error")
            }
          } else {
            panic!("Syntax Error")
          }
        }
        "re.++" => arguments
          .into_iter()
          .map(|term| Regex::new(term))
          .reduce(|reg, curr| reg.concat(curr))
          .expect("Syntax Error"),
        "re.union" => arguments
          .into_iter()
          .map(|term| Regex::new(term))
          .reduce(|reg, curr| reg.or(curr))
          .expect("Syntax Error"),
        "re.inter" => arguments
          .into_iter()
          .map(|term| Regex::new(term))
          .reduce(|reg, curr| reg.inter(curr))
          .expect("Syntax Error"),
        "re.*" => {
          if let [term] = &arguments[..] {
            Regex::new(term).star()
          } else {
            panic!("Syntax Error")
          }
        }
        "re.+" => {
          if let [term] = &arguments[..] {
            Regex::new(term).plus()
          } else {
            panic!("Syntax Error")
          }
        }
        "re.range" => {
          if let [start, end] = &arguments[..] {
            if let Term::Constant(Constant::String(start)) = start {
              if let Term::Constant(Constant::String(end)) = end {
                Regex::range(start.chars().next(), end.chars().next())
              } else {
                panic!("Syntax Error")
              }
            } else {
              panic!("Syntax Error")
            }
          } else {
            panic!("Syntax Error")
          }
        }
        _ => panic!("Syntax Error"),
      },
      Term::QualIdentifier(qi) => match &smt2::get_symbol(qi)[..] {
        "re.nostr" => Regex::Epsilon,
        "re.allchar" => Regex::All,
        _ => panic!("Syntax Error"),
      },
      _ => panic!("Syntax Error"),
    }
  }
}
impl Recognizable<char> for Regex<char> {
  fn member(&self, _: &[char]) -> bool {
    unimplemented!()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  type Reg = Regex<char>;

  #[test]
  fn atomics() {
    let empty = Reg::empty();
    assert_eq!(empty, Reg::Empty);

    let eps = Reg::epsilon();
    assert_eq!(eps, Reg::Epsilon);

    let all = Reg::all();
    assert_eq!(all, Reg::All);

    let a = Reg::element('a');
    assert_eq!(a, Reg::Element('a'));

    let empty = Reg::range(None, None);
    assert_eq!(empty, Reg::Empty);
    let a = Reg::range(Some('a'), Some('a'));
    assert_eq!(a, Reg::Element('a'));
    let a_ = Reg::range(Some('a'), None);
    assert_eq!(a_, Regex::Range(Some('a'), None));
    let _c = Reg::range(None, Some('c'));
    assert_eq!(_c, Regex::Range(None, Some('c')));
    let a_c = Reg::range(Some('a'), Some('c'));
    assert_eq!(a_c, Reg::Range(Some('a'), Some('c')));
  }

  #[test]
  fn concat() {
    let ab = Reg::element('a').concat(Reg::element('b'));
    assert_eq!(ab, Reg::Concat(vec![Reg::element('a'), Reg::element('b')]));
    let abab = ab.clone().concat(ab);
    assert_eq!(
      abab,
      Reg::Concat(vec![
        Reg::element('a'),
        Reg::element('b'),
        Reg::element('a'),
        Reg::element('b')
      ])
    );

    let seq = Reg::seq("abab");
    assert_eq!(seq, abab);
  }

  #[test]
  fn or() {
    let ab = Reg::element('a').or(Reg::element('b'));
    assert_eq!(ab, Reg::Or(vec![Reg::element('a'), Reg::element('b')]));
    let ab = ab.clone().or(ab);
    assert_eq!(ab, Reg::Or(vec![Reg::element('a'), Reg::element('b')]));
    let abc = ab.or(Reg::element('c'));
    assert_eq!(
      abc,
      Reg::Or(vec![
        Reg::element('a'),
        Reg::element('b'),
        Reg::element('c')
      ])
    );
  }

  #[test]
  fn inter() {
    let ab = Reg::element('a').inter(Reg::element('b'));
    assert_eq!(ab, Reg::Inter(vec![Reg::element('a'), Reg::element('b')]));
    let ab = ab.clone().inter(ab);
    assert_eq!(ab, Reg::Inter(vec![Reg::element('a'), Reg::element('b')]));
    let abc = ab.inter(Reg::element('c'));
    assert_eq!(
      abc,
      Reg::Inter(vec![
        Reg::element('a'),
        Reg::element('b'),
        Reg::element('c')
      ])
    );
  }

  #[test]
  fn not() {
    let a = Reg::element('a');
    let not_a = a.clone().not();
    assert_eq!(not_a, Reg::Not(Box::new(a)));
  }

  #[test]
  fn star() {
    let abc = Reg::seq("abc");
    let star = abc.clone().star();
    assert_eq!(star, Reg::Star(Box::new(abc)));
  }

  #[test]
  fn plus() {
    let abc = Reg::seq("abc");
    let star = abc.clone().plus();
    assert_eq!(star, Reg::Plus(Box::new(abc)));
  }
}

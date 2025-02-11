//! Rust identifiers, paths, and patterns.
//!
//! Ident: std, fs, File
//! IdentPath: std, std::fs, fs::File, super::fs::File, std::fs::File
//! CanonicalPath: crate::fs::File
//! Pattern: std::fs, std::fs::*

use log::warn;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

use crate::effect::SrcLoc;

use super::util::iter::FreshIter;

fn replace_hyphens(s: &mut String) {
    while let Some(i) = s.find('-') {
        s.replace_range(i..(i + 1), "_");
    }
}

#[test]
fn test_replace_hyphens() {
    let mut s1 = "abcd_efgh_ijkl".to_string();
    let mut s2 = "abcd-efgh_ijkl".to_string();
    let mut s3 = "abcd-efgh-ijkl".to_string();
    let s = s1.clone();
    replace_hyphens(&mut s1);
    replace_hyphens(&mut s2);
    replace_hyphens(&mut s3);
    assert_eq!(s1, s);
    assert_eq!(s2, s);
    assert_eq!(s3, s);
}

/// An Rust name identifier, without colons
/// E.g.: env
/// Should be a nonempty string
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ident(String);
impl Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Ident {
    fn char_ok(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    fn str_ok(s: &str) -> bool {
        let skips = if s.starts_with("r#") { 2 } else { 0 };
        s.chars().skip(skips).all(Self::char_ok) && !s.is_empty()
    }

    pub fn invariant(&self) -> bool {
        Self::str_ok(&self.0)
    }

    pub fn check_invariant(&self) {
        if !self.invariant() {
            warn!("failed invariant! on Ident {}", self);
        }
    }

    pub fn new(s: &str) -> Self {
        Self::new_owned(s.to_string())
    }

    pub fn new_owned(s: String) -> Self {
        let mut result = Self(s);
        replace_hyphens(&mut result.0);
        result.check_invariant();
        result
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A Rust path identifier, with colons
/// E.g.: std::env::var_os
/// Semantically a (possibly empty) sequence of Idents
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentPath(String);
impl Display for IdentPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl IdentPath {
    pub fn invariant(&self) -> bool {
        self.0.is_empty() || self.0.split("::").all(Ident::str_ok)
    }

    pub fn check_invariant(&self) {
        if !self.invariant() {
            warn!("failed invariant! on IdentPath {}", self);
        }
    }

    pub fn new(s: &str) -> Self {
        Self::new_owned(s.to_string())
    }

    pub fn new_owned(s: String) -> Self {
        let mut result = Self(s);
        replace_hyphens(&mut result.0);
        result.check_invariant();
        result
    }

    pub fn new_empty() -> Self {
        Self::new("")
    }

    pub fn from_ident(i: Ident) -> Self {
        let result = Self(i.0);
        result.check_invariant();
        result
    }

    pub fn from_idents(is: impl Iterator<Item = Ident>) -> Self {
        let mut result = Self::new_empty();
        for i in is {
            result.push_ident(&i)
        }
        result.check_invariant();
        result
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push_ident(&mut self, i: &Ident) {
        if !self.is_empty() {
            self.0.push_str("::");
        }
        self.0.push_str(i.as_str());
        self.check_invariant();
    }

    pub fn pop_ident(&mut self) -> Option<Ident> {
        let (s1, s2) = self.0.rsplit_once("::")?;
        let result = Ident::new(s2);
        self.0 = s1.to_string();
        self.check_invariant();
        Some(result)
    }

    pub fn last_ident(&self) -> Option<Ident> {
        let (_, i) = self.0.rsplit_once("::")?;
        Some(Ident::new(i))
    }

    pub fn first_ident(&self) -> Option<Ident> {
        let (i, _) = self.0.split_once("::")?;
        Some(Ident::new(i))
    }

    pub fn append(&mut self, other: &Self) {
        if !other.is_empty() {
            if !self.is_empty() {
                self.0.push_str("::");
            }
            self.0.push_str(other.as_str());
            self.check_invariant();
        }
    }

    /// Iterator over identifiers in the path
    pub fn idents(&self) -> impl Iterator<Item = Ident> + '_ {
        self.0.split("::").map(Ident::new)
    }

    /// O(n) length check
    pub fn len(&self) -> usize {
        self.idents().count()
    }

    /// Iterator over patterns *which match* the path
    /// Current implementation using FreshIter
    pub fn patterns(&self) -> impl Iterator<Item = Pattern> {
        let mut result = String::new();
        let mut results = Vec::new();
        let mut first = true;
        for id in self.idents() {
            if first {
                first = false;
            } else {
                result.push_str("::");
            }
            result.push_str(&id.0);
            results.push(Pattern::new(&result));
        }
        results.drain(..).fresh_iter()
    }

    pub fn matches(&self, pattern: &Pattern) -> bool {
        self.0.starts_with(pattern.as_str())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for IdentPath {
    fn default() -> Self {
        Self::new_empty()
    }
}

/// Type representing a *canonical* path of Rust idents.
/// i.e. from the root
/// Should not be empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanonicalPath {
    ident_path: IdentPath,
    src_loc: SrcLoc,
}

impl Display for CanonicalPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.ident_path.fmt(f)
    }
}

impl CanonicalPath {
    pub fn invariant(&self) -> bool {
        self.ident_path.invariant() && !self.ident_path.is_empty()
    }

    pub fn check_invariant(&self) {
        if !self.invariant() {
            warn!("failed invariant! on CanonicalPath {}", self);
        }
    }

    pub fn new(s: &str) -> Self {
        Self::from_path(IdentPath::new(s), SrcLoc::default())
    }

    pub fn new_owned(s: String, l: SrcLoc) -> Self {
        Self::from_path(IdentPath::new_owned(s), l)
    }

    pub fn from_path(p: IdentPath, s: SrcLoc) -> Self {
        let result = Self { ident_path: p, src_loc: s };
        result.check_invariant();
        result
    }

    pub fn push_ident(&mut self, i: &Ident) {
        self.ident_path.push_ident(i);
        self.check_invariant();
    }

    pub fn pop_ident(&mut self) -> Option<Ident> {
        let result = self.ident_path.pop_ident();
        self.check_invariant();
        result
    }

    pub fn append_path(&mut self, other: &IdentPath) {
        self.ident_path.append(other);
        self.check_invariant();
    }

    pub fn crate_name(&self) -> Ident {
        self.ident_path.idents().next().unwrap()
    }

    pub fn to_path(self) -> IdentPath {
        self.ident_path
    }

    pub fn as_path(&self) -> &IdentPath {
        &self.ident_path
    }

    pub fn as_str(&self) -> &str {
        self.ident_path.as_str()
    }

    // NOTE: The matches definition should align with whatever definition format
    //       we use for our default sinks
    pub fn matches(&self, pattern: &Pattern) -> bool {
        self.ident_path.matches(pattern)
    }

    pub fn remove_src_loc(&mut self) {
        self.src_loc = SrcLoc::default();
    }

    pub fn add_src_loc(&mut self, src_loc: SrcLoc) -> Self {
        self.src_loc = src_loc;

        self.to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TypeKind {
    RawPointer,
    Callable(CallableKind),
    DynTrait,
    Generic,
    UnionFld,
    StaticMut,
    Function,
    #[default]
    // Default case. Types that we have fully resolved
    // and do not need extra information about.
    Plain,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallableKind {
    Closure,
    FnPtr,
    FnOnce,
    Other,
}

/// Type representing a type identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct CanonicalType {
    ty: String,
    ty_kind: TypeKind,
    trait_bounds: Vec<CanonicalPath>,
}
impl Display for CanonicalType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.ty.fmt(f)
    }
}

impl CanonicalType {
    fn char_ok(c: char) -> bool {
        c.is_ascii_alphanumeric() || "_-&*+|!=',;:<>()[]{} ".contains(c)
    }

    pub fn invariant(&self) -> bool {
        self.ty.chars().all(Self::char_ok)
    }

    pub fn check_invariant(&self) {
        if !self.invariant() {
            warn!("failed invariant! on CanonicalType {}", self);
        }
    }

    pub fn new(s: &str) -> Self {
        Self::new_owned_string(s.to_string())
    }

    pub fn new_owned_string(s: String) -> Self {
        Self::new_owned(s, vec![], Default::default())
    }

    pub fn new_owned(s: String, b: Vec<CanonicalPath>, k: TypeKind) -> Self {
        let result = Self { ty: s, trait_bounds: b, ty_kind: k };
        result.check_invariant();
        result
    }

    pub fn as_str(&self) -> &str {
        self.ty.as_str()
    }

    pub fn add_trait_bound(&mut self, trait_bound: CanonicalPath) {
        self.trait_bounds.push(trait_bound)
    }

    pub fn get_trait_bounds(&self) -> &Vec<CanonicalPath> {
        &self.trait_bounds
    }

    pub fn is_raw_ptr(&self) -> bool {
        matches!(self.ty_kind, TypeKind::RawPointer)
    }

    pub fn is_callable(&self) -> bool {
        matches!(&self.ty_kind, TypeKind::Callable(_))
    }

    pub fn is_dyn_trait(&self) -> bool {
        matches!(self.ty_kind, TypeKind::DynTrait)
    }

    pub fn is_generic(&self) -> bool {
        matches!(self.ty_kind, TypeKind::Generic)
    }

    pub fn is_closure(&self) -> bool {
        matches!(&self.ty_kind, TypeKind::Callable(crate::ident::CallableKind::Closure))
    }

    pub fn is_union_field(&self) -> bool {
        matches!(self.ty_kind, TypeKind::UnionFld)
    }

    pub fn is_mut_static(&self) -> bool {
        matches!(self.ty_kind, TypeKind::StaticMut)
    }

    pub fn is_function(&self) -> bool {
        matches!(self.ty_kind, TypeKind::Function)
    }

    pub fn is_fn_ptr(&self) -> bool {
        matches!(&self.ty_kind, TypeKind::Callable(crate::ident::CallableKind::FnPtr))
    }

    pub fn get_callable_kind(&self) -> Option<CallableKind> {
        if let TypeKind::Callable(kind) = &self.ty_kind {
            return Some(kind.clone());
        }
        None
    }
}

/// Type representing a pattern over paths
///
/// Currently supported: only patterns of the form
/// <path>::* (includes <path> itself)
/// The ::* is left implicit and should not be provided
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pattern(IdentPath);
impl Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Pattern {
    pub fn invariant(&self) -> bool {
        self.0.invariant()
    }

    pub fn check_invariant(&self) {
        if !self.invariant() {
            warn!("failed invariant! on Pattern {}", self);
        }
    }

    pub fn new(s: &str) -> Self {
        Self::from_path(IdentPath::new(s))
    }

    pub fn new_owned(s: String) -> Self {
        Self::from_path(IdentPath::new_owned(s))
    }

    pub fn from_ident(i: Ident) -> Self {
        Self::from_path(IdentPath::from_ident(i))
    }

    pub fn first_ident(&self) -> Option<Ident> {
        self.0.first_ident()
    }

    pub fn from_path(p: IdentPath) -> Self {
        let result = Self(p);
        result.check_invariant();
        result
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Return true if the set of paths denoted by self is
    /// a subset of those denoted by other
    pub fn subset(&self, other: &Self) -> bool {
        self.0.matches(other)
    }

    /// Return true if the set of paths denoted by self is
    /// a superset of those denoted by other
    pub fn superset(&self, other: &Self) -> bool {
        other.subset(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_patterns() {
        let p = IdentPath::new("std::fs");
        let pats: Vec<Pattern> = p.patterns().collect();
        let pat1 = Pattern::new("std");
        let pat2 = Pattern::new("std::fs");
        assert_eq!(pats, vec![pat1, pat2])
    }

    #[test]
    fn test_path_matches() {
        let p = IdentPath::new("std::fs");
        let pat1 = Pattern::new("std");
        let pat2 = Pattern::new("std::fs");
        let pat3 = Pattern::new("std::fs::File");
        let pat4 = Pattern::new("std::os");
        assert!(p.matches(&pat1));
        assert!(p.matches(&pat2));
        assert!(!p.matches(&pat3));
        assert!(!p.matches(&pat4));
    }

    #[test]
    fn test_pattern_subset_superset() {
        let pat1 = Pattern::new("std");
        let pat2 = Pattern::new("std::fs");
        let pat3 = Pattern::new("std::fs::File");
        let pat4 = Pattern::new("std::os");

        assert!(pat1.superset(&pat2));
        assert!(pat1.superset(&pat3));
        assert!(pat2.superset(&pat3));

        assert!(pat2.subset(&pat1));
        assert!(pat3.subset(&pat1));
        assert!(pat3.subset(&pat2));

        assert!(pat1.subset(&pat1));
        assert!(pat2.subset(&pat2));
        assert!(pat4.subset(&pat4));

        assert!(!pat1.subset(&pat2));
        assert!(!pat2.subset(&pat4));
        assert!(!pat4.subset(&pat2));
    }
}

use std::collections::HashMap;
use super::types::Ty;

/// A scope in the type environment — variables and their types
#[derive(Debug)]
struct Scope {
    bindings: HashMap<String, Binding>,
    borrows_to_release: Vec<(String, Vec<String>, bool)>, // var_name, field_path, is_mut
}

use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum VarState {
    Active,
    Moved(Span),
    BorrowedMutably(Span),
    BorrowedImmutably(Vec<Span>),
    Partial(HashMap<String, Box<VarState>>),
}

impl VarState {
    pub fn get_path(&self, path: &[String]) -> &VarState {
        if path.is_empty() { return self; }
        if let VarState::Partial(map) = self {
            if let Some(st) = map.get(&path[0]) {
                return st.get_path(&path[1..]);
            }
            return &VarState::Active; // Field not in partial map -> it's active
        }
        self // If it's Moved/Borrowed, all children inherit this state
    }

    pub fn set_path(&mut self, path: &[String], state: VarState) {
        if path.is_empty() {
            *self = state;
            return;
        }
        if !matches!(self, VarState::Partial(_)) {
            *self = VarState::Partial(HashMap::new());
        }
        if let VarState::Partial(map) = self {
            let child = map.entry(path[0].clone()).or_insert_with(|| Box::new(VarState::Active));
            child.set_path(&path[1..], state);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub ty: Ty,
    pub is_mutable: bool,
    pub state: VarState,
}

/// The type environment — a stack of scopes for lexical scoping
#[derive(Debug)]
pub struct TypeEnv {
    scopes: Vec<Scope>,
    /// Named type definitions (structs, enums)
    pub type_defs: HashMap<String, Ty>,
    /// Function signatures
    pub fn_sigs: HashMap<String, Ty>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![Scope { bindings: HashMap::new(), borrows_to_release: Vec::new() }],
            type_defs: HashMap::new(),
            fn_sigs: HashMap::new(),
        };
        // Register built-in function: print
        env.fn_sigs.insert("print".into(), Ty::Function {
            params: vec![Ty::String],
            ret: Box::new(Ty::Unit),
        });
        env
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope { bindings: HashMap::new(), borrows_to_release: Vec::new() });
    }

    pub fn pop_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            // Release any borrows owned by this scope
            let to_release = scope.borrows_to_release;
            for (name, path, is_mut) in to_release {
                self.release_borrow(&name, &path, is_mut);
            }
        }
    }

    pub fn add_scope_borrow(&mut self, name: &str, path: Vec<String>, is_mut: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.borrows_to_release.push((name.to_string(), path, is_mut));
        }
    }

    pub fn release_borrow(&mut self, name: &str, path: &[String], is_mut: bool) {
        if let Some(binding) = self.lookup_mut(name) {
            let state = binding.state.get_path(path).clone();
            if is_mut {
                if let VarState::BorrowedMutably(_) = state {
                    binding.state.set_path(path, VarState::Active);
                }
            } else {
                if let VarState::BorrowedImmutably(mut spans) = state {
                    if spans.len() > 1 {
                        spans.pop();
                        binding.state.set_path(path, VarState::BorrowedImmutably(spans));
                    } else {
                        binding.state.set_path(path, VarState::Active);
                    }
                }
            }
        }
    }

    pub fn define(&mut self, name: &str, ty: Ty, is_mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), Binding { ty, is_mutable, state: VarState::Active });
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        // Search from innermost scope outward
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                return Some(binding);
            }
        }
        None
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Binding> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.bindings.get_mut(name) {
                return Some(binding);
            }
        }
        None
    }

    pub fn define_type(&mut self, name: &str, ty: Ty) {
        self.type_defs.insert(name.to_string(), ty);
    }

    pub fn lookup_type(&self, name: &str) -> Option<&Ty> {
        self.type_defs.get(name)
    }

    pub fn define_fn(&mut self, name: &str, ty: Ty) {
        self.fn_sigs.insert(name.to_string(), ty);
    }

    pub fn lookup_fn(&self, name: &str) -> Option<&Ty> {
        self.fn_sigs.get(name)
    }
}

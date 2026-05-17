use std::collections::HashMap;
use super::types::Ty;

/// A scope in the type environment — variables and their types
#[derive(Debug)]
struct Scope {
    bindings: HashMap<String, Binding>,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub ty: Ty,
    pub is_mutable: bool,
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
            scopes: vec![Scope { bindings: HashMap::new() }],
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
        self.scopes.push(Scope { bindings: HashMap::new() });
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: &str, ty: Ty, is_mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), Binding { ty, is_mutable });
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

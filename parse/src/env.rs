use super::ast::Rule;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug, Default)]
pub struct Env {
    // Rc<> is OK because we are single-threaded and by definition an env can never modify its
    // parent, only lookup. BUT! If we use a Rc, we can no longer modify the env itself when we DO
    // have it around??? Which is a problem because we may encounter future bindings in top-level
    // scopes, so we do want to be able to have a mutable reference. Could use refcell.
    // May want to switch to a vector/arena kind of thing.
    parent: Option<Rc<RefCell<Env>>>,
    bindings: HashMap<Vec<u8>, Vec<u8>>,
}

// Umm... bindngs may need to store exprs to allow rules to store unevaluated things.

impl Env {
    pub fn with_parent(env: Rc<RefCell<Env>>) -> Self {
        Env {
            parent: Some(env),
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub fn with_parent_owned(env: Env) -> Self {
        Env {
            parent: Some(Rc::new(RefCell::new(env))),
            ..Default::default()
        }
    }

    pub fn add_binding<V1: Into<Vec<u8>>, V2: Into<Vec<u8>>>(&mut self, name: V1, value: V2) {
        self.bindings.insert(name.into(), value.into());
    }

    pub fn lookup<'a, V: Into<&'a [u8]>>(&self, name: V) -> Option<Vec<u8>> {
        let x = name.into();
        dbg!(std::str::from_utf8(&x).unwrap());
        eprintln!("{}", self);
        self.bindings
            .get(x)
            .map(|x| x.clone())
            .or_else(|| self.parent.as_ref().and_then(|p| p.borrow().lookup(x)))
    }

    // While this function works, it requires the caller to be aware of when to use it.
    // It would be nicer to have a BuildEnv binding on build edges that always took a rule binding
    // for evaluation (and did not have just `lookup`). That would also need the AST eval thing to
    // change.
    // We would prefer not to encode lifetimes in top-level env because they can be shared in
    // sub-ninja rules etc (although it isn't clear yet how a multi-file parser looks). It is ok
    // however to encode input-related life times in rules and bindings until canonicalization.
    pub fn lookup_for_build<'b, 'c, V: Into<&'c [u8]>>(
        &self,
        rule: &Rule,
        name: V,
    ) -> Option<Vec<u8>> {
        let x = name.into();
        dbg!(std::str::from_utf8(&x).unwrap());
        eprintln!("{}", self);
        self.bindings.get(x).map(|x| x.clone()).or_else(|| {
            // TODO: Deal with  the possibility of recursion.
            let rule_val = rule.bindings.get(x);
            if let Some(rule_val) = rule_val {
                return Some(rule_val.eval_for_build(self, rule));
            } else {
                self.parent.as_ref().and_then(|p| p.borrow().lookup(x))
            }
        })
    }
}

impl std::fmt::Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Env {{\n")?;
        for (k, v) in &self.bindings {
            write!(
                f,
                "  {} -> {},\n",
                std::str::from_utf8(k).unwrap_or("non-utf8"),
                std::str::from_utf8(v).unwrap_or("non-utf8"),
            )?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod test {
    use super::Env;

    #[test]
    fn test_basic() {
        let mut env = Env::default();
        env.add_binding("hello", "there");
        assert_eq!(env.lookup(b"hello".as_ref()), Some(b"there".to_vec()));
        assert_eq!(env.lookup(b"hello2".as_ref()), None);
    }

    #[test]
    fn test_parent() {
        let mut parent = Env::default();
        parent.add_binding("in_parent", "exists");

        let mut env = Env::with_parent_owned(parent);
        env.add_binding("hello", "there");
        assert_eq!(env.lookup(b"hello".as_ref()), Some(b"there".to_vec()));
        assert_eq!(env.lookup(b"in_parent".as_ref()), Some(b"exists".to_vec()));
        assert_eq!(env.lookup(b"not_in_parent".as_ref()), None);
    }
}

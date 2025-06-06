use std::hash::Hash;
use std::ops::Deref;

use ruff_db::parsed::ParsedModule;

/// Ref-counted owned reference to an AST node.
///
/// The type holds an owned reference to the node's ref-counted [`ParsedModule`].
/// Holding on to the node's [`ParsedModule`] guarantees that the reference to the
/// node must still be valid.
///
/// Holding on to any [`AstNodeRef`] prevents the [`ParsedModule`] from being released.
///
/// ## Equality
/// Two `AstNodeRef` are considered equal if their pointer addresses are equal.
///
/// ## Usage in salsa tracked structs
/// It's important that [`AstNodeRef`] fields in salsa tracked structs are tracked fields
/// (attributed with `#[tracked`]). It prevents that the tracked struct gets a new ID
/// every time the AST changes, which in turn, invalidates the result of any query
/// that takes said tracked struct as a query argument or returns the tracked struct as part of its result.
///
/// For example, marking the [`AstNodeRef`] as tracked on `Expression`
/// has the effect that salsa will consider the expression as "unchanged" for as long as it:
///
/// * belongs to the same file
/// * belongs to the same scope
/// * has the same kind
/// * was created in the same order
///
/// This means that changes to expressions in other scopes don't invalidate the expression's id, giving
/// us some form of scope-stable identity for expressions. Only queries accessing the node field
/// run on every AST change. All other queries only run when the expression's identity changes.
#[derive(Clone)]
pub struct AstNodeRef<T> {
    /// Owned reference to the node's [`ParsedModule`].
    ///
    /// The node's reference is guaranteed to remain valid as long as it's enclosing
    /// [`ParsedModule`] is alive.
    parsed: ParsedModule,

    /// Pointer to the referenced node.
    node: std::ptr::NonNull<T>,
}

#[expect(unsafe_code)]
impl<T> AstNodeRef<T> {
    /// Creates a new `AstNodeRef` that references `node`. The `parsed` is the [`ParsedModule`] to
    /// which the `AstNodeRef` belongs.
    ///
    /// ## Safety
    ///
    /// Dereferencing the `node` can result in undefined behavior if `parsed` isn't the
    /// [`ParsedModule`] to which `node` belongs. It's the caller's responsibility to ensure that
    /// the invariant `node belongs to parsed` is upheld.
    pub(super) unsafe fn new(parsed: ParsedModule, node: &T) -> Self {
        Self {
            parsed,
            node: std::ptr::NonNull::from(node),
        }
    }

    /// Returns a reference to the wrapped node.
    pub const fn node(&self) -> &T {
        // SAFETY: Holding on to `parsed` ensures that the AST to which `node` belongs is still
        // alive and not moved.
        unsafe { self.node.as_ref() }
    }
}

impl<T> Deref for AstNodeRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

impl<T> std::fmt::Debug for AstNodeRef<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AstNodeRef").field(&self.node()).finish()
    }
}

impl<T> PartialEq for AstNodeRef<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.parsed == other.parsed {
            // Comparing the pointer addresses is sufficient to determine equality
            // if the parsed are the same.
            self.node.eq(&other.node)
        } else {
            // Otherwise perform a deep comparison.
            self.node().eq(other.node())
        }
    }
}

impl<T> Eq for AstNodeRef<T> where T: Eq {}

impl<T> Hash for AstNodeRef<T>
where
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node().hash(state);
    }
}

#[expect(unsafe_code)]
unsafe impl<T> salsa::Update for AstNodeRef<T> {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_ref = &mut (*old_pointer);

        if old_ref.parsed == new_value.parsed && old_ref.node.eq(&new_value.node) {
            false
        } else {
            *old_ref = new_value;
            true
        }
    }
}

#[expect(unsafe_code)]
unsafe impl<T> Send for AstNodeRef<T> where T: Send {}
#[expect(unsafe_code)]
unsafe impl<T> Sync for AstNodeRef<T> where T: Sync {}

#[cfg(test)]
mod tests {
    use crate::ast_node_ref::AstNodeRef;
    use ruff_db::parsed::ParsedModule;
    use ruff_python_ast::PySourceType;
    use ruff_python_parser::parse_unchecked_source;

    #[test]
    #[expect(unsafe_code)]
    fn equality() {
        let parsed_raw = parse_unchecked_source("1 + 2", PySourceType::Python);
        let parsed = ParsedModule::new(parsed_raw.clone());

        let stmt = &parsed.syntax().body[0];

        let node1 = unsafe { AstNodeRef::new(parsed.clone(), stmt) };
        let node2 = unsafe { AstNodeRef::new(parsed.clone(), stmt) };

        assert_eq!(node1, node2);

        // Compare from different trees
        let cloned = ParsedModule::new(parsed_raw);
        let stmt_cloned = &cloned.syntax().body[0];
        let cloned_node = unsafe { AstNodeRef::new(cloned.clone(), stmt_cloned) };

        assert_eq!(node1, cloned_node);

        let other_raw = parse_unchecked_source("2 + 2", PySourceType::Python);
        let other = ParsedModule::new(other_raw);

        let other_stmt = &other.syntax().body[0];
        let other_node = unsafe { AstNodeRef::new(other.clone(), other_stmt) };

        assert_ne!(node1, other_node);
    }

    #[expect(unsafe_code)]
    #[test]
    fn inequality() {
        let parsed_raw = parse_unchecked_source("1 + 2", PySourceType::Python);
        let parsed = ParsedModule::new(parsed_raw);

        let stmt = &parsed.syntax().body[0];
        let node = unsafe { AstNodeRef::new(parsed.clone(), stmt) };

        let other_raw = parse_unchecked_source("2 + 2", PySourceType::Python);
        let other = ParsedModule::new(other_raw);

        let other_stmt = &other.syntax().body[0];
        let other_node = unsafe { AstNodeRef::new(other.clone(), other_stmt) };

        assert_ne!(node, other_node);
    }

    #[test]
    #[expect(unsafe_code)]
    fn debug() {
        let parsed_raw = parse_unchecked_source("1 + 2", PySourceType::Python);
        let parsed = ParsedModule::new(parsed_raw);

        let stmt = &parsed.syntax().body[0];

        let stmt_node = unsafe { AstNodeRef::new(parsed.clone(), stmt) };

        let debug = format!("{stmt_node:?}");

        assert_eq!(debug, format!("AstNodeRef({stmt:?})"));
    }
}

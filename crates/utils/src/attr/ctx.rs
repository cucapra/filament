use crate::GPosIdx;

/// A trait for attribute stores
pub trait AttrCtx<Attr, Value> {
    /// Check if an attribute is set
    fn has(&self, attr: Attr) -> bool {
        self.get(attr).is_some()
    }

    /// Get the value of an attribute if it is set
    fn get(&self, attr: Attr) -> Option<&Value>;

    /// Get the location of an attribute if it is set
    fn get_loc(&self, attr: Attr) -> Option<GPosIdx>;

    /// Set the value of an attribute
    fn set(&mut self, attr: Attr, value: Value, loc: GPosIdx);

    /// Remove an attribute
    fn remove(&mut self, attr: Attr);
}

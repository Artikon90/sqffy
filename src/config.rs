/// Runtime configuration for a SQffy session.
#[derive(Debug, Clone)]
pub struct Config {
    /// Max rows printed for SELECT unless overridden.
    pub row_limit: usize,
    /// Enable the built-in pager for tall output.
    pub pager: bool,
    /// Truncate wide cell values beyond this width.
    pub max_cell_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            row_limit: 100,
            pager: true,
            max_cell_width: 64,
        }
    }
}

impl Config {
    pub fn with_row_limit(mut self, row_limit: usize) -> Self {
        self.row_limit = row_limit;
        self
    }

    pub fn with_pager(mut self, pager: bool) -> Self {
        self.pager = pager;
        self
    }
}

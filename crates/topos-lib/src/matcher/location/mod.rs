pub mod line_col;

/// This is for text fragments
pub mod html;

#[cfg(feature = "pdf")]
pub mod pdf;
// pub mod json;

// #[non_exhaustive]
pub enum AnyLocation {
    // LineCol(LineColLocation)
}

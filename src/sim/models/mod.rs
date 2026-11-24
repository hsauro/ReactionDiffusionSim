//! The built-in models. Adding one means a new file here and a `ModelKind` variant.

mod barkley;
mod brusselator;
mod gray_scott;
mod lengyel_epstein;

pub use barkley::Barkley;
pub use brusselator::Brusselator;
pub use gray_scott::GrayScott;
pub use lengyel_epstein::LengyelEpstein;

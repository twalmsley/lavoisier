//! Marker-trait encoding: one struct per catalogue combination,
//! macro-generated (R6, experiment step 1a).
//!
//! Limitation found here: stable `macro_rules!` cannot concatenate
//! identifiers (no `paste`-style `${concat(...)}` on stable, and external
//! crates are forbidden), so the macro cannot take the three value lists and
//! emit the cross product with synthesized names. Every combination still
//! needs one hand-written line naming its struct. The macro removes the four
//! `impl` lines per struct, not the per-combination line itself.

use crate::characteristics::*;

/// Generates one unit struct per line plus its four marker-trait impls.
macro_rules! bolts {
    ($($name:ident: $size:ident, $material:ident, $length:ident;)*) => {
        $(
            pub struct $name;
            impl IsBolt for $name {}
            impl $size for $name {}
            impl $material for $name {}
            impl $length for $name {}
        )*
    };
}

bolts! {
    // M6
    M6SteelBolt10mm:  M6,  Steel, Length10mm;
    M6SteelBolt15mm:  M6,  Steel, Length15mm;
    M6SteelBolt20mm:  M6,  Steel, Length20mm;
    M6BrassBolt10mm:  M6,  Brass, Length10mm;
    M6BrassBolt15mm:  M6,  Brass, Length15mm;
    M6BrassBolt20mm:  M6,  Brass, Length20mm;
    // M8
    M8SteelBolt10mm:  M8,  Steel, Length10mm;
    M8SteelBolt15mm:  M8,  Steel, Length15mm;
    M8SteelBolt20mm:  M8,  Steel, Length20mm;
    M8BrassBolt10mm:  M8,  Brass, Length10mm;
    M8BrassBolt15mm:  M8,  Brass, Length15mm;
    M8BrassBolt20mm:  M8,  Brass, Length20mm;
    // M10
    M10SteelBolt10mm: M10, Steel, Length10mm;
    M10SteelBolt15mm: M10, Steel, Length15mm;
    M10SteelBolt20mm: M10, Steel, Length20mm;
    M10BrassBolt10mm: M10, Brass, Length10mm;
    M10BrassBolt15mm: M10, Brass, Length15mm;
    M10BrassBolt20mm: M10, Brass, Length20mm;
    // M12 — added in experiment step 4: six new lines, nothing else changed
    // in this module.
    M12SteelBolt10mm: M12, Steel, Length10mm;
    M12SteelBolt15mm: M12, Steel, Length15mm;
    M12SteelBolt20mm: M12, Steel, Length20mm;
    M12BrassBolt10mm: M12, Brass, Length10mm;
    M12BrassBolt15mm: M12, Brass, Length15mm;
    M12BrassBolt20mm: M12, Brass, Length20mm;
}

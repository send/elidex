//! Writing mode and directionality keyword enums.

use std::fmt;

keyword_enum! {
    /// The CSS `direction` property (CSS Writing Modes Level 3 §2.1).
    ///
    /// Inherited. Sets the inline base direction of an element.
    Direction {
        Ltr => "ltr",
        Rtl => "rtl",
    }
}

keyword_enum! {
    /// The CSS `unicode-bidi` property (CSS Writing Modes Level 3 §2.2).
    ///
    /// Non-inherited. Controls how bidi embedding levels are applied.
    UnicodeBidi {
        Normal => "normal",
        Embed => "embed",
        BidiOverride => "bidi-override",
        Isolate => "isolate",
        IsolateOverride => "isolate-override",
        Plaintext => "plaintext",
    }
}

keyword_enum! {
    /// The CSS `writing-mode` property (CSS Writing Modes Level 4 §3.1).
    ///
    /// Inherited. Determines the block flow direction and inline base direction.
    WritingMode {
        HorizontalTb => "horizontal-tb",
        VerticalRl => "vertical-rl",
        VerticalLr => "vertical-lr",
        SidewaysRl => "sideways-rl",
        SidewaysLr => "sideways-lr",
    }
}

impl WritingMode {
    /// Returns `true` if this writing mode has a horizontal inline axis.
    ///
    /// `horizontal-tb` is the only horizontal writing mode. All vertical
    /// modes (`vertical-rl`, `vertical-lr`, `sideways-rl`, `sideways-lr`)
    /// return `false`.
    #[must_use]
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::HorizontalTb)
    }
}

keyword_enum! {
    /// The CSS `text-orientation` property (CSS Writing Modes Level 3 §5.1).
    ///
    /// Inherited. Controls glyph orientation in vertical writing modes.
    TextOrientation {
        Mixed => "mixed",
        Upright => "upright",
        Sideways => "sideways",
    }
}

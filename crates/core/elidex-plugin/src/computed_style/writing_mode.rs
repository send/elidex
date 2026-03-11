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
    /// The CSS `writing-mode` property (CSS Writing Modes Level 3 §3.1).
    ///
    /// Inherited. Determines the block flow direction and inline base direction.
    // TODO(Phase 4): CSS Writing Modes Level 4 §3.1 adds `sideways-rl` and
    // `sideways-lr` keywords (currently only implemented by Firefox).
    WritingMode {
        HorizontalTb => "horizontal-tb",
        VerticalRl => "vertical-rl",
        VerticalLr => "vertical-lr",
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

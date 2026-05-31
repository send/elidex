//! WHATWG HTML §13.2.6.1 / §13.2.6.5 foreign-content name adjustments.
//!
//! Pure data + transforms applied to a start-tag token *before* it is
//! marshalled into an [`EcsDom`] element (the marshalling itself lives in
//! [`super::insert`]). Three spec adjustments are folded into one entry
//! point, [`adjust_foreign_start_tag`], keyed on the namespace the element
//! will be created in:
//!
//! - the **SVG element tag-name** case table (§13.2.6.5 "any other start
//!   tag") — 37 lowercase tokenizer names mapped to their camel-cased SVG
//!   element names;
//! - **adjust SVG attributes** (§13.2.6.1) — 58 lowercase attribute names
//!   mapped to their camel-cased forms;
//! - **adjust MathML attributes** (§13.2.6.1) — the single `definitionurl`
//!   → `definitionURL` rename;
//! - **adjust foreign attributes** (§13.2.6.1) — the XLink/xml/xmlns
//!   prefix-to-namespace binding. The namespace *binding* is deferred to
//!   `#11-xml-namespace` (the `EcsDom` `Attributes` map is a flat string
//!   map with no per-attribute namespace); the prefixed names are already
//!   correctly cased, so under the current model the step is a documented
//!   no-op with a single home (see [`adjust_foreign_start_tag`]).

use elidex_ecs::Namespace;

/// Apply the §13.2.6.1 / §13.2.6.5 name adjustments for a start-tag token
/// that is about to be inserted as a foreign element in `namespace`,
/// mutating the tag `name` and `attrs` in place.
///
/// Mirrors the spec's "any other start tag" sequence (§13.2.6.5) and the
/// `<math>` / `<svg>` "in body" entry (§13.2.6.4.7): for an SVG element the
/// tag name is case-corrected and SVG attributes are adjusted; for a MathML
/// element MathML attributes are adjusted; then foreign attributes are
/// adjusted for both. `namespace` is the namespace the element will be
/// created in (the adjusted current node's namespace at an "any other start
/// tag", or the literal MathML / SVG namespace at a `<math>` / `<svg>`
/// entry).
pub(super) fn adjust_foreign_start_tag(
    namespace: Namespace,
    name: &mut String,
    attrs: &mut [(String, String)],
) {
    match namespace {
        Namespace::MathMl => adjust_mathml_attributes(attrs),
        Namespace::Svg => {
            if let Some(corrected) = svg_element_name(name) {
                *name = corrected.to_string();
            }
            adjust_svg_attributes(attrs);
        }
        // The HTML branch is unreachable: foreign elements are only ever
        // created in the SVG or MathML namespace. Left total for exhaustive
        // matching rather than guarded with a panic.
        Namespace::Html => {}
    }
    // Adjust foreign attributes (§13.2.6.1): bind `xlink:` / `xml:` /
    // `xmlns` prefixed attribute names to their namespaces. The binding is
    // deferred to `#11-xml-namespace` — the flat `Attributes` map has no
    // per-attribute namespace — and the prefixed names are already correctly
    // cased, so there is nothing to rewrite here yet. This comment is the
    // single home for the binding when it lands.
}

/// §13.2.6.5 "any other start tag" SVG element tag-name case table: map a
/// lowercase tokenizer tag name to its camel-cased SVG element name, or
/// `None` when the name needs no correction (most SVG elements are already
/// all-lowercase, e.g. `circle`, `path`, `g`, `svg`).
fn svg_element_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "altglyph" => "altGlyph",
        "altglyphdef" => "altGlyphDef",
        "altglyphitem" => "altGlyphItem",
        "animatecolor" => "animateColor",
        "animatemotion" => "animateMotion",
        "animatetransform" => "animateTransform",
        "clippath" => "clipPath",
        "feblend" => "feBlend",
        "fecolormatrix" => "feColorMatrix",
        "fecomponenttransfer" => "feComponentTransfer",
        "fecomposite" => "feComposite",
        "feconvolvematrix" => "feConvolveMatrix",
        "fediffuselighting" => "feDiffuseLighting",
        "fedisplacementmap" => "feDisplacementMap",
        "fedistantlight" => "feDistantLight",
        "fedropshadow" => "feDropShadow",
        "feflood" => "feFlood",
        "fefunca" => "feFuncA",
        "fefuncb" => "feFuncB",
        "fefuncg" => "feFuncG",
        "fefuncr" => "feFuncR",
        "fegaussianblur" => "feGaussianBlur",
        "feimage" => "feImage",
        "femerge" => "feMerge",
        "femergenode" => "feMergeNode",
        "femorphology" => "feMorphology",
        "feoffset" => "feOffset",
        "fepointlight" => "fePointLight",
        "fespecularlighting" => "feSpecularLighting",
        "fespotlight" => "feSpotLight",
        "fetile" => "feTile",
        "feturbulence" => "feTurbulence",
        "foreignobject" => "foreignObject",
        "glyphref" => "glyphRef",
        "lineargradient" => "linearGradient",
        "radialgradient" => "radialGradient",
        "textpath" => "textPath",
        _ => return None,
    })
}

/// §13.2.6.1 "adjust SVG attributes": case-correct each SVG attribute name
/// in `attrs` that is not all-lowercase. The token's attribute names arrive
/// ASCII-lowercased from the tokenizer (§13.2.5.33), so the lookup keys are
/// lowercase.
fn adjust_svg_attributes(attrs: &mut [(String, String)]) {
    for (name, _) in attrs.iter_mut() {
        if let Some(corrected) = svg_attribute_name(name) {
            *name = corrected.to_string();
        }
    }
}

/// §13.2.6.1 "adjust MathML attributes": rename `definitionurl` to
/// `definitionURL` (the only MathML attribute that is not all-lowercase).
fn adjust_mathml_attributes(attrs: &mut [(String, String)]) {
    for (name, _) in attrs.iter_mut() {
        if name == "definitionurl" {
            *name = "definitionURL".to_string();
        }
    }
}

/// §13.2.6.1 SVG attribute case table: map a lowercase tokenizer attribute
/// name to its camel-cased SVG attribute name, or `None` when no correction
/// is needed.
fn svg_attribute_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "attributename" => "attributeName",
        "attributetype" => "attributeType",
        "basefrequency" => "baseFrequency",
        "baseprofile" => "baseProfile",
        "calcmode" => "calcMode",
        "clippathunits" => "clipPathUnits",
        "diffuseconstant" => "diffuseConstant",
        "edgemode" => "edgeMode",
        "filterunits" => "filterUnits",
        "glyphref" => "glyphRef",
        "gradienttransform" => "gradientTransform",
        "gradientunits" => "gradientUnits",
        "kernelmatrix" => "kernelMatrix",
        "kernelunitlength" => "kernelUnitLength",
        "keypoints" => "keyPoints",
        "keysplines" => "keySplines",
        "keytimes" => "keyTimes",
        "lengthadjust" => "lengthAdjust",
        "limitingconeangle" => "limitingConeAngle",
        "markerheight" => "markerHeight",
        "markerunits" => "markerUnits",
        "markerwidth" => "markerWidth",
        "maskcontentunits" => "maskContentUnits",
        "maskunits" => "maskUnits",
        "numoctaves" => "numOctaves",
        "pathlength" => "pathLength",
        "patterncontentunits" => "patternContentUnits",
        "patterntransform" => "patternTransform",
        "patternunits" => "patternUnits",
        "pointsatx" => "pointsAtX",
        "pointsaty" => "pointsAtY",
        "pointsatz" => "pointsAtZ",
        "preservealpha" => "preserveAlpha",
        "preserveaspectratio" => "preserveAspectRatio",
        "primitiveunits" => "primitiveUnits",
        "refx" => "refX",
        "refy" => "refY",
        "repeatcount" => "repeatCount",
        "repeatdur" => "repeatDur",
        "requiredextensions" => "requiredExtensions",
        "requiredfeatures" => "requiredFeatures",
        "specularconstant" => "specularConstant",
        "specularexponent" => "specularExponent",
        "spreadmethod" => "spreadMethod",
        "startoffset" => "startOffset",
        "stddeviation" => "stdDeviation",
        "stitchtiles" => "stitchTiles",
        "surfacescale" => "surfaceScale",
        "systemlanguage" => "systemLanguage",
        "tablevalues" => "tableValues",
        "targetx" => "targetX",
        "targety" => "targetY",
        "textlength" => "textLength",
        "viewbox" => "viewBox",
        "viewtarget" => "viewTarget",
        "xchannelselector" => "xChannelSelector",
        "ychannelselector" => "yChannelSelector",
        "zoomandpan" => "zoomAndPan",
        _ => return None,
    })
}

pub mod keyframes;
pub mod font_face;
pub mod page;
pub mod supports;
pub mod counter_style;
pub mod namespace;
pub mod import;
pub mod media;
pub mod style;

use media::MediaRule;
use import::ImportRule;
use style::StyleRule;
use keyframes::KeyframesRule;
use font_face::FontFaceRule;
use page::PageRule;
use supports::SupportsRule;
use counter_style::CounterStyleRule;
use namespace::NamespaceRule;
use crate::traits::ToCss;
use crate::printer::Printer;
use crate::declaration::DeclarationHandler;
use crate::vendor_prefix::VendorPrefix;
use crate::prefixes::Feature;
use crate::targets::Browsers;
use std::collections::HashMap;
use crate::selector::{is_equivalent, get_prefix, get_necessary_prefixes};

#[derive(Debug, PartialEq)]
pub enum CssRule {
  Media(MediaRule),
  Import(ImportRule),
  Style(StyleRule),
  Keyframes(KeyframesRule),
  FontFace(FontFaceRule),
  Page(PageRule),
  Supports(SupportsRule),
  CounterStyle(CounterStyleRule),
  Namespace(NamespaceRule)
}

impl ToCss for CssRule {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> std::fmt::Result where W: std::fmt::Write {
    match self {
      CssRule::Media(media) => media.to_css(dest),
      CssRule::Import(import) => import.to_css(dest),
      CssRule::Style(style) => style.to_css(dest),
      CssRule::Keyframes(keyframes) => keyframes.to_css(dest),
      CssRule::FontFace(font_face) => font_face.to_css(dest),
      CssRule::Page(font_face) => font_face.to_css(dest),
      CssRule::Supports(supports) => supports.to_css(dest),
      CssRule::CounterStyle(counter_style) => counter_style.to_css(dest),
      CssRule::Namespace(namespace) => namespace.to_css(dest)
    }
  }
}

#[derive(Debug, PartialEq)]
pub struct CssRuleList(pub Vec<CssRule>);

impl CssRuleList {
  pub(crate) fn minify(&mut self, targets: Option<Browsers>, handler: &mut DeclarationHandler, important_handler: &mut DeclarationHandler) {
    let mut keyframe_rules = HashMap::new();
    let mut rules = Vec::new();
    for mut rule in self.0.drain(..) {
      match &mut rule {
        CssRule::Keyframes(keyframes) => {
          keyframes.minify(handler, important_handler);

          macro_rules! set_prefix {
            ($keyframes: ident) => {
              if $keyframes.vendor_prefix.contains(VendorPrefix::None) {
                if let Some(targets) = targets {
                  $keyframes.vendor_prefix = Feature::AtKeyframes.prefixes_for(targets)
                }
              }
            };
          }
  
          // If there is an existing rule with the same name and identical keyframes,
          // merge the vendor prefixes from this rule into it.
          if let Some(existing_idx) = keyframe_rules.get(&keyframes.name) {
            if let Some(CssRule::Keyframes(existing)) = &mut rules.get_mut(*existing_idx) {
              if existing.keyframes == keyframes.keyframes {
                existing.vendor_prefix |= keyframes.vendor_prefix;
                set_prefix!(existing);
                continue;
              }
            }
          }
  
          set_prefix!(keyframes);
          keyframe_rules.insert(keyframes.name.clone(), rules.len());
        },
        CssRule::Media(media) => media.minify(targets, handler, important_handler),
        CssRule::Supports(supports) => supports.minify(targets, handler, important_handler),
        CssRule::Style(style) => {
          style.minify(handler, important_handler);

          if let Some(targets) = targets {
            style.vendor_prefix = get_prefix(&style.selectors);
            if style.vendor_prefix.contains(VendorPrefix::None) {
              style.vendor_prefix = get_necessary_prefixes(&style.selectors, targets);
            }
          }

          if let Some(CssRule::Style(last_style_rule)) = rules.last_mut() {
            // Merge declarations if the selectors are equivalent, and both are compatible with all targets.
            if style.selectors == last_style_rule.selectors && style.is_compatible(targets) && last_style_rule.is_compatible(targets) {
              last_style_rule.declarations.declarations.extend(style.declarations.declarations.drain(..));
              last_style_rule.declarations.minify(handler, important_handler);
              continue
            } else if style.declarations == last_style_rule.declarations {
              // Append the selectors to the last rule if the declarations are the same, and all selectors are compatible.
              if style.is_compatible(targets) && last_style_rule.is_compatible(targets) {
                last_style_rule.selectors.0.extend(style.selectors.0.drain(..));
                continue
              }

              // If both selectors are potentially vendor prefixable, and they are 
              // equivalent minus prefixes, add the prefix to the last rule.
              if !style.vendor_prefix.is_empty() && 
                !last_style_rule.vendor_prefix.is_empty() &&
                is_equivalent(&style.selectors, &last_style_rule.selectors)
              {
                // If the new rule is unprefixed, replace the prefixes of the last rule.
                // Otherwise, add the new prefix.
                if style.vendor_prefix.contains(VendorPrefix::None) {
                  last_style_rule.vendor_prefix = style.vendor_prefix;
                } else {
                  last_style_rule.vendor_prefix |= style.vendor_prefix;
                }
                continue
              }
            }
          }
        },
        _ => {}
      }

      rules.push(rule)
    }

    self.0 = rules;
  }
}

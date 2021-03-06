/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
use element_state::ElementState;
use properties::{self, ServoComputedValues};
use selector_matching::{USER_OR_USER_AGENT_STYLESHEETS, QUIRKS_MODE_STYLESHEET};
use selectors::Element;
use selectors::parser::{ParserContext, SelectorImpl};
use stylesheets::Stylesheet;

pub trait ElementExt: Element {
    fn is_link(&self) -> bool;
}

pub trait SelectorImplExt : SelectorImpl + Sized {
    type ComputedValues: properties::ComputedValues;

    fn each_pseudo_element<F>(mut fun: F)
        where F: FnMut(<Self as SelectorImpl>::PseudoElement);

    /// This function determines if a pseudo-element is eagerly cascaded or not.
    ///
    /// Eagerly cascaded pseudo-elements are "normal" pseudo-elements (i.e.
    /// `::before` and `::after`). They inherit styles normally as another
    /// selector would do.
    ///
    /// Non-eagerly cascaded ones skip the cascade process entirely, mostly as
    /// an optimisation since they are private pseudo-elements (like
    /// `::-servo-details-content`). This pseudo-elements are resolved on the
    /// fly using global rules (rules of the form `*|*`), and applying them to
    /// the parent style.
    ///
    /// If you're implementing a public selector that the end-user might
    /// customize, then you probably need doing the whole cascading process and
    /// return true in this function for that pseudo.
    ///
    /// But if you are implementing a private pseudo-element, please consider if
    /// it might be possible to skip the cascade for it.
    fn is_eagerly_cascaded_pseudo_element(pseudo: &<Self as SelectorImpl>::PseudoElement) -> bool;

    #[inline]
    fn each_eagerly_cascaded_pseudo_element<F>(mut fun: F)
        where F: FnMut(<Self as SelectorImpl>::PseudoElement) {
        Self::each_pseudo_element(|pseudo| {
            if Self::is_eagerly_cascaded_pseudo_element(&pseudo) {
                fun(pseudo)
            }
        })
    }

    #[inline]
    fn each_non_eagerly_cascaded_pseudo_element<F>(mut fun: F)
        where F: FnMut(<Self as SelectorImpl>::PseudoElement) {
        Self::each_pseudo_element(|pseudo| {
            if !Self::is_eagerly_cascaded_pseudo_element(&pseudo) {
                fun(pseudo)
            }
        })
    }

    fn pseudo_class_state_flag(pc: &Self::NonTSPseudoClass) -> ElementState;

    fn get_user_or_user_agent_stylesheets() -> &'static [Stylesheet<Self>];

    fn get_quirks_mode_stylesheet() -> Option<&'static Stylesheet<Self>>;
}

#[derive(Clone, Debug, PartialEq, Eq, HeapSizeOf, Hash)]
pub enum PseudoElement {
    Before,
    After,
    Selection,
    DetailsSummary,
    DetailsContent,
}

impl PseudoElement {
    #[inline]
    pub fn is_eagerly_cascaded(&self) -> bool {
        match *self {
            PseudoElement::Before |
            PseudoElement::After |
            PseudoElement::Selection |
            PseudoElement::DetailsSummary => true,
            PseudoElement::DetailsContent => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, HeapSizeOf, Hash)]
pub enum NonTSPseudoClass {
    AnyLink,
    Link,
    Visited,
    Active,
    Focus,
    Hover,
    Enabled,
    Disabled,
    Checked,
    Indeterminate,
    ServoNonZeroBorder,
    ReadWrite,
    ReadOnly
}

impl NonTSPseudoClass {
    pub fn state_flag(&self) -> ElementState {
        use element_state::*;
        use self::NonTSPseudoClass::*;
        match *self {
            Active => IN_ACTIVE_STATE,
            Focus => IN_FOCUS_STATE,
            Hover => IN_HOVER_STATE,
            Enabled => IN_ENABLED_STATE,
            Disabled => IN_DISABLED_STATE,
            Checked => IN_CHECKED_STATE,
            Indeterminate => IN_INDETERMINATE_STATE,
            ReadOnly | ReadWrite => IN_READ_WRITE_STATE,

            AnyLink |
            Link |
            Visited |
            ServoNonZeroBorder => ElementState::empty(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, HeapSizeOf)]
pub struct ServoSelectorImpl;

impl SelectorImpl for ServoSelectorImpl {
    type PseudoElement = PseudoElement;
    type NonTSPseudoClass = NonTSPseudoClass;

    fn parse_non_ts_pseudo_class(context: &ParserContext,
                                 name: &str) -> Result<NonTSPseudoClass, ()> {
        use self::NonTSPseudoClass::*;
        let pseudo_class = match_ignore_ascii_case! { name,
            "any-link" => AnyLink,
            "link" => Link,
            "visited" => Visited,
            "active" => Active,
            "focus" => Focus,
            "hover" => Hover,
            "enabled" => Enabled,
            "disabled" => Disabled,
            "checked" => Checked,
            "indeterminate" => Indeterminate,
            "read-write" => ReadWrite,
            "read-only" => ReadOnly,
            "-servo-nonzero-border" => {
                if !context.in_user_agent_stylesheet {
                    return Err(());
                }
                ServoNonZeroBorder
            },
            _ => return Err(())
        };

        Ok(pseudo_class)
    }

    fn parse_pseudo_element(context: &ParserContext,
                            name: &str) -> Result<PseudoElement, ()> {
        use self::PseudoElement::*;
        let pseudo_element = match_ignore_ascii_case! { name,
            "before" => Before,
            "after" => After,
            "selection" => Selection,
            "-servo-details-summary" => {
                if !context.in_user_agent_stylesheet {
                    return Err(())
                }
                DetailsSummary
            },
            "-servo-details-content" => {
                if !context.in_user_agent_stylesheet {
                    return Err(())
                }
                DetailsContent
            },
            _ => return Err(())
        };

        Ok(pseudo_element)
    }
}

impl SelectorImplExt for ServoSelectorImpl {
    type ComputedValues = ServoComputedValues;

    #[inline]
    fn is_eagerly_cascaded_pseudo_element(pseudo: &PseudoElement) -> bool {
        pseudo.is_eagerly_cascaded()
    }

    #[inline]
    fn each_pseudo_element<F>(mut fun: F)
        where F: FnMut(PseudoElement) {
        fun(PseudoElement::Before);
        fun(PseudoElement::After);
        fun(PseudoElement::DetailsContent);
        fun(PseudoElement::DetailsSummary);
        fun(PseudoElement::Selection);
    }

    #[inline]
    fn pseudo_class_state_flag(pc: &NonTSPseudoClass) -> ElementState {
        pc.state_flag()
    }

    #[inline]
    fn get_user_or_user_agent_stylesheets() -> &'static [Stylesheet<Self>] {
        &*USER_OR_USER_AGENT_STYLESHEETS
    }

    #[inline]
    fn get_quirks_mode_stylesheet() -> Option<&'static Stylesheet<Self>> {
        Some(&*QUIRKS_MODE_STYLESHEET)
    }
}

impl<E: Element<Impl=ServoSelectorImpl>> ElementExt for E {
    fn is_link(&self) -> bool {
        self.match_non_ts_pseudo_class(NonTSPseudoClass::AnyLink)
    }
}

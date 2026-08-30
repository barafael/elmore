//! The `Html<Msg>` element tree and its builder, plus tag/attribute constants.

/// A single kind of browser event, as exposed by the builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Click,
    Input,
    Change,
    Submit,
    KeyUp,
}

/// An HTML attribute: a static name plus an owned value.
#[derive(Debug, Clone)]
pub struct Attr {
    pub name: &'static str,
    pub value: String,
}

impl Attr {
    fn new(name: &'static str, value: impl Into<String>) -> Self {
        Attr { name, value: value.into() }
    }
}

/// A semantic node in the tree: either an element or a run of text.
pub enum Node<Msg> {
    Element(Tag, Vec<Attr>, Vec<Html<Msg>>),
    Text(String),
}

/// The set of supported element tags. Deliberately short.
#[derive(Debug, Clone, Copy)]
pub enum Tag {
    Div,
    Span,
    P,
    H1,
    H2,
    H3,
    Button,
    Input,
    Label,
    TextArea,
    Ul,
    Li,
    Section,
    Select,
    Option,
    Output,
    Pre,
    Form,
    Iframe,
    Nav,
}

impl Tag {
    /// DOM name of the tag (lowercase). Only used by the wasm renderer, so
    /// native builds see it as dead code.
    #[allow(dead_code)]
    pub(crate) fn name(self) -> &'static str {
        use Tag::*;
        match self {
            Div => "div",
            Span => "span",
            P => "p",
            H1 => "h1",
            H2 => "h2",
            H3 => "h3",
            Button => "button",
            Input => "input",
            Label => "label",
            TextArea => "textarea",
            Ul => "ul",
            Li => "li",
            Section => "section",
            Select => "select",
            Option => "option",
            Output => "output",
            Pre => "pre",
            Form => "form",
            Iframe => "iframe",
            Nav => "nav",
        }
    }
}

/// A bound event handler attached to an element.
///
/// `Simple` events (click, submit) fire with no payload. `WithValue` events
/// (input, change) carry the element's current value as a `String`. `Checked`
/// events (change on a checkbox or radio) carry the new checked state as a
/// `bool`. `Key` events (key up) carry the pressed key's name (e.g.
/// `"Enter"`, `"a"`).
///
/// Handlers sit behind a `Box` and are *moved out* of the tree by the render
/// that arms them, so nothing needs shared ownership of a closure.
pub enum Bind<Msg> {
    Simple(Event, Box<dyn Fn() -> Msg>),
    WithValue(Event, Box<dyn Fn(String) -> Msg>),
    Checked(Event, Box<dyn Fn(bool) -> Msg>),
    Key(Event, Box<dyn Fn(String) -> Msg>),
}

/// An owned HTML tree used by [`App::view`].
///
/// This is a plain data structure (no borrows, no lifetimes beyond `Msg`) so
/// it can be produced fresh each frame and handed to the runtime, which
/// reconciles it against the live DOM.
///
/// It is deliberately *not* `Clone`: it may own closures and is rebuilt from
/// scratch on every update anyway.
pub struct Html<Msg> {
    pub node: Node<Msg>,
    pub binds: Vec<Bind<Msg>>,
    /// Stable identity among siblings; see [`Html::key`].
    pub key: Option<String>,
}

impl<Msg> Html<Msg>
where
    Msg: 'static,
{
    pub fn div() -> Self {
        Self::element(Tag::Div)
    }
    pub fn span() -> Self {
        Self::element(Tag::Span)
    }
    pub fn p() -> Self {
        Self::element(Tag::P)
    }
    pub fn h1() -> Self {
        Self::element(Tag::H1)
    }
    pub fn h2() -> Self {
        Self::element(Tag::H2)
    }
    pub fn h3() -> Self {
        Self::element(Tag::H3)
    }
    pub fn button() -> Self {
        Self::element(Tag::Button)
    }
    pub fn input() -> Self {
        Self::element(Tag::Input)
    }
    /// A multi-line text field. Its `on_input` works exactly like an input's.
    pub fn textarea() -> Self {
        Self::element(Tag::TextArea)
    }
    pub fn label() -> Self {
        Self::element(Tag::Label)
    }
    pub fn ul() -> Self {
        Self::element(Tag::Ul)
    }
    pub fn li() -> Self {
        Self::element(Tag::Li)
    }
    pub fn section() -> Self {
        Self::element(Tag::Section)
    }
    pub fn select() -> Self {
        Self::element(Tag::Select)
    }
    pub fn option() -> Self {
        Self::element(Tag::Option)
    }
    /// The `<output>` element — handy for echoing a slider's live value.
    pub fn output() -> Self {
        Self::element(Tag::Output)
    }
    pub fn pre() -> Self {
        Self::element(Tag::Pre)
    }
    pub fn form() -> Self {
        Self::element(Tag::Form)
    }
    pub fn iframe() -> Self {
        Self::element(Tag::Iframe)
    }
    pub fn nav() -> Self {
        Self::element(Tag::Nav)
    }

    fn element(tag: Tag) -> Self {
        Html { node: Node::Element(tag, Vec::new(), Vec::new()), binds: Vec::new(), key: None }
    }

    /// A plain text node.
    pub fn text_node(text: impl Into<String>) -> Self {
        Html { node: Node::Text(text.into()), binds: Vec::new(), key: None }
    }

    /// Set this element's text content (replaces any existing children).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        if let Node::Element(_, _, children) = &mut self.node {
            children.clear();
            children.push(Html::text_node(text));
        }
        self
    }

    /// Set a static attribute (e.g. `type`, `disabled`, `placeholder`).
    pub fn attr(mut self, name: &'static str, value: impl Into<String>) -> Self {
        if let Node::Element(_, attrs, _) = &mut self.node {
            attrs.push(Attr::new(name, value));
        }
        self
    }

    /// Shorthand for `class`.
    pub fn class(self, class: impl Into<String>) -> Self {
        self.attr("class", class)
    }

    /// Shorthand for `id`.
    pub fn id(self, id: impl Into<String>) -> Self {
        self.attr("id", id)
    }

    /// Shorthand for `type` (mostly on `<input>`).
    pub fn input_type(self, ty: impl Into<String>) -> Self {
        self.attr("type", ty)
    }

    /// Shorthand for `value` (mostly on `<input>`).
    pub fn value(self, value: impl Into<String>) -> Self {
        self.attr("value", value)
    }

    /// Shorthand for `placeholder`.
    pub fn placeholder(self, value: impl Into<String>) -> Self {
        self.attr("placeholder", value)
    }

    /// Shorthand for `src` (on `<iframe>`; `<img>` when it exists).
    pub fn src(self, value: impl Into<String>) -> Self {
        self.attr("src", value)
    }

    /// Shorthand for `disabled`. Only emits the attribute when `true`, so
    /// `.disabled(false)` leaves the element enabled.
    pub fn disabled(self, value: bool) -> Self {
        if value {
            self.attr("disabled", "")
        } else {
            self
        }
    }

    /// Shorthand for the `checked` attribute of checkboxes and radios. Only
    /// emits the attribute when `true`. On a live element the runtime syncs
    /// the `checked` *property* to match, exactly like `value`.
    pub fn checked(self, value: bool) -> Self {
        if value {
            self.attr("checked", "")
        } else {
            self
        }
    }

    /// Give this element a **key**: a stable identity among its siblings
    /// across renders.
    ///
    /// The renderer matches keyed children by key instead of by position, so
    /// reordering *moves* the existing DOM nodes (focus, scroll positions,
    /// and CSS transitions ride along) instead of rewriting their contents,
    /// and inserting or removing in the middle leaves the other items
    /// untouched. Use this for any list the user can reorder or edit.
    ///
    /// Keys must be unique among their siblings.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Add a static child element.
    pub fn child(mut self, child: Html<Msg>) -> Self {
        if let Node::Element(_, _, children) = &mut self.node {
            children.push(child);
        }
        self
    }

    /// Add any number of children. Accepts arrays, `Vec`s, and anything else
    /// that iterates `Html` items — so rendering a collection is just
    /// `.children(items.iter().map(render_one))`.
    pub fn children(mut self, children: impl IntoIterator<Item = Html<Msg>>) -> Self {
        if let Node::Element(_, _, cs) = &mut self.node {
            cs.extend(children);
        }
        self
    }

    /// Attach a click handler.
    pub fn on_click(mut self, f: impl Fn() -> Msg + 'static) -> Self {
        self.binds.push(Bind::Simple(Event::Click, Box::new(f)));
        self
    }

    /// Attach an input handler (text fields). The callback receives the current
    /// `value` of the field as a `String`.
    pub fn on_input(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.binds.push(Bind::WithValue(Event::Input, Box::new(f)));
        self
    }

    /// Attach a change handler (e.g. `<select>`).
    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.binds.push(Bind::WithValue(Event::Change, Box::new(f)));
        self
    }

    /// Attach a toggle handler to a checkbox or radio input. Fires on
    /// `change`; the callback receives the new checked state.
    pub fn on_toggle(mut self, f: impl Fn(bool) -> Msg + 'static) -> Self {
        self.binds.push(Bind::Checked(Event::Change, Box::new(f)));
        self
    }

    /// Attach a submit handler (on forms). Submit events are *always*
    /// `preventDefault`ed — bound or not — so a form never navigates; your
    /// `Msg` is the only thing that happens.
    pub fn on_submit(mut self, f: impl Fn() -> Msg + 'static) -> Self {
        self.binds.push(Bind::Simple(Event::Submit, Box::new(f)));
        self
    }

    /// Attach a key-up handler. The callback receives the *name* of the key
    /// that was released (e.g. `"Enter"`, `"Escape"`, `"a"`).
    pub fn on_key_up(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.binds.push(Bind::Key(Event::KeyUp, Box::new(f)));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_replaces_children() {
        let html: Html<()> = Html::div().child(Html::span()).text("hi");
        match html.node {
            Node::Element(Tag::Div, _, children) => {
                assert_eq!(children.len(), 1);
                assert!(matches!(children[0].node, Node::Text(ref t) if t == "hi"));
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn text_node_is_a_leaf() {
        let html: Html<()> = Html::text_node("t");
        assert!(matches!(html.node, Node::Text(ref t) if t == "t"));
        assert!(html.binds.is_empty());
    }

    #[test]
    fn disabled_false_emits_no_attribute() {
        let html: Html<()> = Html::button().disabled(false);
        match html.node {
            Node::Element(_, attrs, _) => assert!(attrs.is_empty()),
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn disabled_true_sets_attribute() {
        let html: Html<()> = Html::button().disabled(true);
        match html.node {
            Node::Element(_, attrs, _) => {
                assert_eq!(attrs.len(), 1);
                assert_eq!(attrs[0].name, "disabled");
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn checked_false_emits_no_attribute() {
        let html: Html<()> = Html::input().checked(false);
        match html.node {
            Node::Element(_, attrs, _) => assert!(attrs.is_empty()),
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn checked_true_sets_attribute() {
        let html: Html<()> = Html::input().checked(true);
        match html.node {
            Node::Element(_, attrs, _) => {
                assert_eq!(attrs.len(), 1);
                assert_eq!(attrs[0].name, "checked");
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn toggle_handler_is_attached() {
        let html = Html::input().on_toggle(|_| ());
        assert_eq!(html.binds.len(), 1);
    }

    #[test]
    fn shorthands_map_to_attributes() {
        let html: Html<()> = Html::input()
            .class("c")
            .id("i")
            .input_type("text")
            .placeholder("p")
            .value("v");
        match html.node {
            Node::Element(_, attrs, _) => {
                let names: Vec<_> = attrs.iter().map(|a| (a.name, a.value.as_str())).collect();
                assert_eq!(
                    names,
                    [("class", "c"), ("id", "i"), ("type", "text"), ("placeholder", "p"), ("value", "v")]
                );
            }
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn children_extend_in_order() {
        let html: Html<()> = Html::ul().children([Html::li(), Html::li()]).child(Html::li());
        match html.node {
            Node::Element(Tag::Ul, _, children) => assert_eq!(children.len(), 3),
            _ => panic!("expected an element"),
        }
    }

    #[test]
    fn handlers_are_attached() {
        let html = Html::button().on_click(|| ()).on_input(|_| ());
        assert_eq!(html.binds.len(), 2);
    }

    #[test]
    fn tag_names_are_lowercase() {
        assert_eq!(Tag::Div.name(), "div");
        assert_eq!(Tag::Button.name(), "button");
        assert_eq!(Tag::TextArea.name(), "textarea");
        assert_eq!(Tag::Select.name(), "select");
        assert_eq!(Tag::Option.name(), "option");
        assert_eq!(Tag::Form.name(), "form");
    }
}

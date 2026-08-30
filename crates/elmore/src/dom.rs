//! DOM reconciliation and event routing via `web-sys`.
//!
//! Each render, [`render_into`] walks the fresh [`Html`] tree against the
//! live DOM, updating elements in place and creating or removing nodes only
//! where the structure actually changed. Children carrying a `.key(..)` are
//! matched by key across renders, so reorders move nodes instead of
//! rewriting them. Because elements keep their identity, focus, the caret,
//! scroll positions, and clicks in flight all survive — no special handling
//! anywhere.
//!
//! Events are routed by delegation: five listeners live on the mount for the
//! app's whole lifetime. Each render arms a fresh handler table (stored in
//! the [`Runtime`], fully typed — no erasure); elements point at their row
//! via an expando property. When an event bubbles to the mount, [`route`]
//! walks from the target up and calls each matching handler.
//!
//! Handlers are consumed out of the tree (`mem::take`) and re-armed on every
//! render, so they always close over the current model's data; the previous
//! tree kept for diffing has hollow `binds`, which are never read.

use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, KeyboardEvent, Node as DomNode, Text, window};

use crate::App;
use crate::html::{Attr, Bind, Event as HtmlEvent, Html, Node};
use crate::runtime::Runtime;

/// Expando property linking a live element to its row in the handler table.
const HANDLER_PROP: &str = "__elmore_handlers";

/// The DOM events the runtime delegates on. All of these bubble, so one
/// listener per event on the mount sees every event in the app.
const EVENT_NAMES: [&str; 5] = [
    event_name(HtmlEvent::Click),
    event_name(HtmlEvent::Input),
    event_name(HtmlEvent::Change),
    event_name(HtmlEvent::Submit),
    event_name(HtmlEvent::KeyUp),
];

/// One element's handlers for this generation, paired with their event kind.
/// Messages are erased per closure (each captured handler pushes its own
/// `Msg`), so a row is `Msg`-agnostic.
pub(crate) type HandlerRow = Vec<(HtmlEvent, Box<dyn Fn(&Event)>)>;

/// Bind the five app-lifetime delegation listeners onto the mount. The mount
/// element is never replaced by reconciliation, so these survive every
/// render. The closures are leaked once here; this is by design (see the
/// `runtime` module docs).
pub(crate) fn install_event_routing<A: App>(state: &'static Runtime<A>) {
    for name in EVENT_NAMES {
        let cb = Closure::wrap(Box::new(move |ev: Event| route(&ev, state)) as Box<dyn FnMut(Event)>);
        let _ = state.mount.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref());
        std::mem::forget(cb);
    }
}

/// The delegation listener: walk from the event target up to the mount and
/// call every matching handler along the way, in bubbling order.
fn route<A: App>(ev: &Event, state: &'static Runtime<A>) {
    // Forms never navigate: a submit's only effect may be messages, whether
    // or not the form carries an `on_submit`. Without this, an unbound form
    // (or a bare Enter in one of its fields) would replace the page.
    if ev.type_() == "submit" {
        ev.prevent_default();
    }

    let mut node = ev.target().and_then(|t| t.dyn_into::<DomNode>().ok());
    while let Some(current) = node {
        if let Some(el) = current.dyn_ref::<Element>() {
            if same_node(el, &state.mount) {
                return;
            }
            if let Some(id) = handler_id(el) {
                // Handlers only push messages and schedule a frame — they
                // never re-enter this table — so calling them under the
                // borrow is safe.
                let table = state.handlers.borrow();
                if let Some(row) = table.get(id) {
                    for (event, armed) in row {
                        if ev.type_() == event_name(*event) {
                            armed(ev);
                        }
                    }
                }
            }
        }
        node = current.parent_node();
    }
}

/// Reconcile `new` (the fresh `view` tree) into the mount, using `prev` (the
/// tree rendered last render) to know which attributes have disappeared.
pub(crate) fn render_into<A: App>(state: &'static Runtime<A>, new: Html<A::Message>) {
    let doc: Document = window().unwrap().document().unwrap();

    // Take the previous tree out for the walk; if rendering panics halfway,
    // `prev` stays empty and the next render degrades to a full rebuild.
    let old_tree = state.prev.borrow_mut().take();

    // Retire last render's handlers (dropping them); this render arms a
    // fresh table. Nothing is running here, so dropping is safe.
    state.handlers.borrow_mut().clear();

    let mut new_root = [new];
    let old_root: Option<&[Html<A::Message>]> = old_tree.as_ref().map(std::slice::from_ref);
    sync_children(&state.mount, &doc, &mut new_root, old_root, state);

    // The tree just rendered becomes the next render's "previous" tree. Its
    // `binds` are hollow now (consumed into the handler table); only
    // attributes and children are ever diffed against it.
    let [rendered] = new_root;
    *state.prev.borrow_mut() = Some(rendered);
}

/// Reconcile `parent`'s children against `new`, reusing existing DOM nodes
/// wherever possible.
///
/// Children carrying a `.key(..)` are matched by key across renders — so a
/// reorder *moves* nodes instead of morphing them, and inserting or removing
/// in the middle leaves the other items untouched. Unkeyed children match
/// positionally. Nodes the new tree doesn't reference are removed.
fn sync_children<A: App>(
    parent: &Element,
    doc: &Document,
    new: &mut [Html<A::Message>],
    old: Option<&[Html<A::Message>]>,
    state: &'static Runtime<A>,
) {
    // Snapshot the live children; `old` (the tree that produced them) is in
    // the same order, so `old[j]` describes `existing[j]`.
    let mut existing: Vec<DomNode> = Vec::new();
    let mut walker = parent.first_child();
    while let Some(node) = walker {
        walker = node.next_sibling();
        existing.push(node);
    }
    let old = old.unwrap_or(&[]);

    // Decide which existing node each new child reuses.
    let mut used = vec![false; existing.len()];
    let mut reuse: Vec<Option<usize>> = vec![None; new.len()];

    // Pass 1 — keyed children match by key, wherever they were.
    for (i, new_child) in new.iter().enumerate() {
        let Some(key) = new_child.key.as_deref() else { continue };
        for (j, old_child) in old.iter().enumerate() {
            if !used[j] && old_child.key.as_deref() == Some(key) {
                reuse[i] = Some(j);
                used[j] = true;
                break;
            }
        }
    }
    // Pass 2 — unkeyed children fall back to their position, but never steal
    // a slot belonging to a keyed node. (`old.get(i)` bounds `i`: the old
    // tree mirrors the live children, so `used[i]` is in range behind it.)
    for i in 0..new.len() {
        if reuse[i].is_none()
            && new[i].key.is_none()
            && old.get(i).is_some_and(|o| o.key.is_none())
            && !used[i]
        {
            reuse[i] = Some(i);
            used[i] = true;
        }
    }

    // Morph what can be morphed; build the rest.
    let mut desired: Vec<DomNode> = Vec::with_capacity(new.len());
    for i in 0..new.len() {
        let node = match reuse[i] {
            Some(j) if try_morph(doc, &existing[j], &mut new[i], old.get(j), state) => {
                existing[j].clone()
            }
            _ => build(doc, &mut new[i], state).expect("an element or text node"),
        };
        desired.push(node);
    }

    // Place the desired nodes in order. Walking a cursor through the current
    // children: matching nodes advance it, others are inserted before it, and
    // whatever the cursor still points at when `desired` runs out is stale.
    // (`insert_before` also *moves* nodes already attached elsewhere.)
    let mut cursor = parent.first_child();
    for want in &desired {
        if matches!(&cursor, Some(c) if same_node(c, want)) {
            cursor = want.next_sibling();
        } else {
            let _ = parent.insert_before(want, cursor.as_ref());
        }
    }
    while let Some(extra) = cursor {
        let next = extra.next_sibling();
        let _ = parent.remove_child(&extra);
        cursor = next;
    }
}

/// Try to update `dom` in place to match `new`. Returns `false` when the
/// shapes are incompatible — text versus element, or different tags — and
/// the caller must build a fresh node instead.
fn try_morph<A: App>(
    doc: &Document,
    dom: &DomNode,
    new: &mut Html<A::Message>,
    old: Option<&Html<A::Message>>,
    state: &'static Runtime<A>,
) -> bool {
    match (&mut new.node, dom.node_type()) {
        // Text over text: update only when the string changed.
        (Node::Text(t), DomNode::TEXT_NODE) => {
            if let Ok(text) = dom.clone().dyn_into::<Text>()
                && text.data() != *t
            {
                text.set_data(t);
            }
            true
        }

        // Element over element: reuse when the tag matches.
        (Node::Element(tag, attrs, children), DomNode::ELEMENT_NODE) => {
            let Ok(el) = dom.clone().dyn_into::<Element>() else {
                return false;
            };
            if !el.tag_name().eq_ignore_ascii_case(tag.name()) {
                return false;
            }
            let old_attrs = match old {
                Some(Html { node: Node::Element(_, oa, _), .. }) => Some(oa.as_slice()),
                _ => None,
            };
            sync_attrs(&el, attrs, old_attrs);
            arm_handlers(&el, std::mem::take(&mut new.binds), state);

            let old_children = match old {
                Some(Html { node: Node::Element(_, _, oc), .. }) => Some(oc.as_slice()),
                _ => None,
            };
            sync_children(&el, doc, children, old_children, state);

            // Form-field values are DOM properties; sync them after the
            // children so a `<select>`'s options exist first.
            sync_props(&el, attrs, old_attrs);
            true
        }

        // Shape changed (text <-> element): not morphable.
        _ => false,
    }
}

/// Apply attributes: set every one the new tree carries, remove the ones it
/// no longer does (that half needs the previous tree — hence `old`).
fn sync_attrs(el: &Element, new: &[Attr], old: Option<&[Attr]>) {
    if let Some(old) = old {
        for oa in old {
            if !new.iter().any(|na| na.name == oa.name) {
                let _ = el.remove_attribute(oa.name);
            }
        }
    }
    for na in new {
        // Skip no-op writes: re-setting an attribute that already holds the
        // same value is pure churn — and on an `<iframe>`, some browsers
        // treat a re-set `src` as a navigation.
        if el.get_attribute(na.name).as_deref() != Some(na.value.as_str()) {
            let _ = el.set_attribute(na.name, &na.value);
        }
    }
}

/// Sync the property-backed attributes of form fields: `value` (inputs,
/// textareas, selects) and `checked` (checkboxes, radios). Setting the
/// attribute would be ignored on a live element once the user has interacted
/// with it; the property is the truth.
fn sync_props(el: &Element, attrs: &[Attr], old_attrs: Option<&[Attr]>) {
    sync_value_prop(el, attrs, old_attrs);
    sync_checked_prop(el, attrs, old_attrs);
}

/// Sync the `value` *property* of form fields. Equal values are skipped so a
/// focused field's caret never moves.
///
/// A field whose `value` attribute just disappeared is cleared — the stale
/// property would otherwise keep showing ghost text — but a field that was
/// never controlled (no `value` in either tree) is left alone.
fn sync_value_prop(el: &Element, attrs: &[Attr], old_attrs: Option<&[Attr]>) {
    fn find_value(attrs: &[Attr]) -> Option<&Attr> {
        attrs.iter().find(|a| a.name == "value")
    }
    let want = match find_value(attrs) {
        Some(value) => value.value.clone(),
        None => {
            let was_controlled = old_attrs.is_some_and(|old| find_value(old).is_some());
            if !was_controlled {
                return;
            }
            String::new()
        }
    };
    if let Some(field) = as_field(el) {
        set_field_value(field, &want);
    }
}

/// Sync the `checked` *property* of checkbox/radio inputs, with the same
/// controlled semantics as `value`: the attribute present means checked, an
/// attribute that just disappeared unchecks, and an input never controlled
/// in either tree is untouched.
fn sync_checked_prop(el: &Element, attrs: &[Attr], old_attrs: Option<&[Attr]>) {
    fn has_checked(attrs: &[Attr]) -> bool {
        attrs.iter().any(|a| a.name == "checked")
    }
    let Ok(input) = el.clone().dyn_into::<HtmlInputElement>() else {
        return;
    };
    let want = if has_checked(attrs) {
        true
    } else {
        if !old_attrs.is_some_and(|old| has_checked(old)) {
            return;
        }
        false
    };
    if input.checked() != want {
        input.set_checked(want);
    }
}

/// A form field's `value` property, whichever element hosts it.
enum Field {
    Input(HtmlInputElement),
    TextArea(HtmlTextAreaElement),
    Select(HtmlSelectElement),
}

/// Detect which form field (if any) an element is.
fn as_field(el: &Element) -> Option<Field> {
    if let Ok(el) = el.clone().dyn_into::<HtmlInputElement>() {
        Some(Field::Input(el))
    } else if let Ok(el) = el.clone().dyn_into::<HtmlTextAreaElement>() {
        Some(Field::TextArea(el))
    } else if let Ok(el) = el.clone().dyn_into::<HtmlSelectElement>() {
        Some(Field::Select(el))
    } else {
        None
    }
}

/// Read a field's current `value` property.
fn field_value(field: &Field) -> String {
    match field {
        Field::Input(el) => el.value(),
        Field::TextArea(el) => el.value(),
        Field::Select(el) => el.value(),
    }
}

/// Write a field's `value` property, skipping it when already equal so a
/// focused field's caret never moves.
fn set_field_value(field: Field, want: &str) {
    if field_value(&field) != want {
        match field {
            Field::Input(el) => el.set_value(want),
            Field::TextArea(el) => el.set_value(want),
            Field::Select(el) => el.set_value(want),
        }
    }
}

/// Build a fresh subtree for `html` (used where nothing can be reused).
fn build<A: App>(doc: &Document, html: &mut Html<A::Message>, state: &'static Runtime<A>) -> Option<DomNode> {
    match &mut html.node {
        Node::Text(t) => Some(doc.create_text_node(t).into()),
        Node::Element(tag, attrs, children) => {
            let el: Element = doc.create_element(tag.name()).unwrap();
            for a in attrs.iter() {
                let _ = el.set_attribute(a.name, &a.value);
            }
            arm_handlers(&el, std::mem::take(&mut html.binds), state);
            for child in children.iter_mut() {
                if let Some(n) = build(doc, child, state) {
                    let _ = el.append_child(&n);
                }
            }
            sync_props(&el, attrs, None);
            Some(el.into())
        }
    }
}

/// Consume `binds` and register their handlers in the runtime's table,
/// pointing `el` at its row. Elements without binds get their pointer
/// cleared, so a stale row is never found.
fn arm_handlers<A: App>(el: &Element, binds: Vec<Bind<A::Message>>, state: &'static Runtime<A>) {
    if binds.is_empty() {
        let _ = Reflect::set(el.as_ref(), &JsValue::from_str(HANDLER_PROP), &JsValue::UNDEFINED);
        return;
    }

    let mut row: HandlerRow = Vec::new();
    for bind in binds {
        match bind {
            // (Submit's preventDefault happens unconditionally in `route`.)
            Bind::Simple(event, f) => {
                row.push((event, Box::new(move |_: &Event| state.push(f()))));
            }
            Bind::WithValue(event, f) => {
                row.push((event, Box::new(move |ev: &Event| state.push(f(target_value(ev))))));
            }
            Bind::Checked(event, f) => {
                row.push((event, Box::new(move |ev: &Event| state.push(f(target_checked(ev))))));
            }
            Bind::Key(event, f) => {
                row.push((event, Box::new(move |ev: &Event| state.push(f(key_of(ev))))));
            }
        }
    }

    let mut table = state.handlers.borrow_mut();
    table.push(row);
    let id = table.len() - 1;
    drop(table);
    Reflect::set(el.as_ref(), &JsValue::from_str(HANDLER_PROP), &JsValue::from_f64(id as f64)).ok();
}

/// Name of the DOM event for a builder [`HtmlEvent`].
const fn event_name(ev: HtmlEvent) -> &'static str {
    use HtmlEvent::*;
    match ev {
        Click => "click",
        Input => "input",
        Change => "change",
        Submit => "submit",
        KeyUp => "keyup",
    }
}

fn handler_id(el: &Element) -> Option<usize> {
    Reflect::get(el.as_ref(), &JsValue::from_str(HANDLER_PROP))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|f| f as usize)
}

/// Identity comparison for two DOM nodes.
fn same_node(a: &DomNode, b: &DomNode) -> bool {
    JsValue::from(a.clone()) == JsValue::from(b.clone())
}

/// The element the app mounts into, looked up by id.
pub(crate) fn root_element(root_id: &str) -> Element {
    window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(root_id)
        .expect("elmore: mount element not found")
}

/// Read the `.value` of the event target when it is an input, select, or textarea.
fn target_value(ev: &Event) -> String {
    let target = match ev.target() {
        Some(t) => t,
        None => return String::new(),
    };
    let Ok(el) = target.dyn_into::<Element>() else {
        return String::new();
    };
    as_field(&el).map(|f| field_value(&f)).unwrap_or_default()
}

/// Read the `.checked` of the event target when it is a checkbox-style input.
fn target_checked(ev: &Event) -> bool {
    let target = match ev.target() {
        Some(t) => t,
        None => return false,
    };
    let Ok(el) = target.dyn_into::<Element>() else {
        return false;
    };
    el.dyn_ref::<HtmlInputElement>()
        .map(|input| input.checked())
        .unwrap_or(false)
}

/// Read the name of the released key (`KeyboardEvent.key`).
fn key_of(ev: &Event) -> String {
    ev.dyn_ref::<KeyboardEvent>()
        .map(|ke| ke.key())
        .unwrap_or_default()
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::{HtmlSelectElement, window};

    use crate::App;
    use crate::command::Command;
    use crate::html::Html;
    use crate::runtime::{Runtime, run};

    use super::{install_event_routing, render_into};

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Ping,
        Key(String),
        Value(String),
        Submitted,
    }

    #[derive(Default)]
    struct TestApp;

    impl App for TestApp {
        type Message = TestMsg;
        type Model = ();

        fn update(&mut self, _msg: TestMsg, _model: &mut ()) -> Option<Command<TestMsg>> {
            None
        }

        fn view(&self, _model: &()) -> Html<TestMsg> {
            Html::div()
        }
    }

    /// A fresh mount attached to the document, plus a runtime for any app.
    fn mount_as<A: App>(id: &str) -> &'static Runtime<A> {
        let doc = window().unwrap().document().unwrap();
        let el = doc.create_element("div").unwrap();
        el.set_id(id);
        doc.body().unwrap().append_child(&el).unwrap();
        let state: &'static Runtime<A> =
            Box::leak(Box::new(Runtime::new(el, window().unwrap())));
        // Pushes schedule frames; give the runtime its callback so the
        // boot invariant holds here too.
        state.install_frame_callback();
        state
    }

    /// The usual mount: one for the standard `TestApp`.
    fn mount(id: &str) -> &'static Runtime<TestApp> {
        mount_as(id)
    }

    #[wasm_bindgen_test]
    fn builds_the_tree() {
        let state = mount("t-builds");
        render_into(state, Html::div().class("a").child(Html::span().text("hi")));
        assert_eq!(state.mount.inner_html(), r#"<div class="a"><span>hi</span></div>"#);
    }

    #[wasm_bindgen_test]
    fn removed_attributes_are_removed() {
        let state = mount("t-attr");
        render_into(state, Html::button().disabled(true));
        let btn = state.mount.first_element_child().unwrap();
        assert_eq!(btn.get_attribute("disabled").as_deref(), Some(""));

        render_into(state, Html::button());
        assert_eq!(btn.get_attribute("disabled"), None);
    }

    #[wasm_bindgen_test]
    fn text_updates_in_place() {
        let state = mount("t-text");
        render_into(state, Html::div().child(Html::span().text("a")));
        render_into(state, Html::div().child(Html::span().text("b")));
        assert_eq!(state.mount.text_content().as_deref(), Some("b"));
        assert_eq!(state.mount.child_nodes().length(), 1);
    }

    #[wasm_bindgen_test]
    fn tag_change_replaces_the_node() {
        let state = mount("t-replace");
        render_into(state, Html::div().text("old"));
        render_into(state, Html::p().text("new"));
        assert_eq!(state.mount.inner_html(), "<p>new</p>");
    }

    #[wasm_bindgen_test]
    fn select_value_is_set_as_a_property() {
        let state = mount("t-select");
        let select = Html::select()
            .children([
                Html::option().value("red").text("red"),
                Html::option().value("green").text("green"),
            ])
            .value("green");
        render_into(state, select);
        let el = state.mount.first_element_child().unwrap();
        let select: HtmlSelectElement = el.dyn_into().unwrap();
        assert_eq!(select.value(), "green");
    }

    #[wasm_bindgen_test]
    fn clicks_are_delegated_to_the_sink() {
        let state = mount("t-click");
        install_event_routing(state);
        render_into(state, Html::button().on_click(|| TestMsg::Ping));
        let btn = state.mount.first_element_child().unwrap();
        let btn: web_sys::HtmlElement = btn.dyn_into().unwrap();
        btn.click();
        assert_eq!(state.sink.borrow().len(), 1);
    }

    #[wasm_bindgen_test]
    fn keyed_reorder_moves_nodes() {
        let state = mount("t-keyed");
        let item = |k: &str, t: &str| Html::li().key(k).text(t);
        render_into(state, Html::ul().children([item("a", "A"), item("b", "B")]));
        let ul = state.mount.first_element_child().unwrap();
        let a_before = ul.first_element_child().unwrap();
        assert_eq!(ul.inner_html(), "<li>A</li><li>B</li>");

        // Swap the order; same keys, same content.
        render_into(state, Html::ul().children([item("b", "B"), item("a", "A")]));
        let ul = state.mount.first_element_child().unwrap();
        assert_eq!(ul.inner_html(), "<li>B</li><li>A</li>");

        // The "A" node *moved* — it is the same node, now second.
        let a_after = ul.child_nodes().item(1).unwrap();
        assert_eq!(a_after.text_content().as_deref(), Some("A"));
        assert!(
            wasm_bindgen::JsValue::from(a_before) == wasm_bindgen::JsValue::from(a_after),
            "keyed nodes keep their identity across reorders"
        );
    }

    #[wasm_bindgen_test]
    fn keyed_removal_leaves_siblings_alone() {
        let state = mount("t-keyed-rm");
        let item = |k: &str| Html::li().key(k).text(k);
        render_into(
            state,
            Html::ul().children([item("a"), item("b"), item("c")]),
        );
        let ul = state.mount.first_element_child().unwrap();
        let c_before = ul.child_nodes().item(2).unwrap();

        // Remove the middle item; the others must not be disturbed.
        render_into(state, Html::ul().children([item("a"), item("c")]));
        let ul = state.mount.first_element_child().unwrap();
        assert_eq!(ul.inner_html(), "<li>a</li><li>c</li>");
        let c_after = ul.child_nodes().item(1).unwrap();
        assert!(
            wasm_bindgen::JsValue::from(c_before) == wasm_bindgen::JsValue::from(c_after),
            "untouched keyed siblings keep their identity"
        );
    }

    #[wasm_bindgen_test]
    fn unkeyed_children_still_match_positionally() {
        let state = mount("t-unkeyed");
        render_into(state, Html::ul().children([Html::li().text("x"), Html::li().text("y")]));
        render_into(state, Html::ul().children([Html::li().text("x"), Html::li().text("z")]));
        let ul = state.mount.first_element_child().unwrap();
        assert_eq!(ul.inner_html(), "<li>x</li><li>z</li>");
        assert_eq!(ul.child_nodes().length(), 2);
    }

    #[derive(Default)]
    struct CounterApp;

    enum CounterMsg {
        Inc,
    }

    impl App for CounterApp {
        type Message = CounterMsg;
        type Model = u32;

        fn update(&mut self, _msg: CounterMsg, model: &mut u32) -> Option<Command<CounterMsg>> {
            *model += 1;
            None
        }

        fn view(&self, model: &u32) -> Html<CounterMsg> {
            Html::div()
                .class("counter")
                .children([
                    Html::span().text(model.to_string()),
                    Html::button().text("+1").on_click(|| CounterMsg::Inc),
                ])
        }
    }

    /// The whole loop, end to end: `run` boots and renders, a real click is
    /// delegated into the sink, the scheduled frame runs `update`, and the
    /// re-render shows the new model.
    #[wasm_bindgen_test]
    async fn boots_and_round_trips_a_click() {
        let doc = window().unwrap().document().unwrap();
        // A pristine `#root` every run: `run` hangs app-lifetime listeners
        // on the mount itself, so it must never be reused.
        if let Some(old) = doc.get_element_by_id("root") {
            old.remove();
        }
        let el = doc.create_element("div").unwrap();
        el.set_id("root");
        doc.body().unwrap().append_child(&el).unwrap();

        run::<CounterApp>();

        let root = doc.get_element_by_id("root").unwrap();
        assert_eq!(
            root.inner_html(),
            r#"<div class="counter"><span>0</span><button>+1</button></div>"#
        );

        let btn: web_sys::HtmlElement = root
            .first_element_child()
            .unwrap()
            .last_element_child()
            .unwrap()
            .dyn_into()
            .unwrap();
        btn.click();

        // The click becomes a render only after the next animation frame.
        gloo_timers::future::TimeoutFuture::new(100).await;

        let text = root
            .first_element_child()
            .unwrap()
            .first_element_child()
            .unwrap()
            .text_content()
            .unwrap();
        assert_eq!(text, "1");
    }

    #[wasm_bindgen_test]
    fn focus_and_caret_survive_re_render() {
        let state = mount("t-focus");
        render_into(state, Html::input().value("a"));
        let input: web_sys::HtmlInputElement =
            state.mount.first_element_child().unwrap().dyn_into().unwrap();
        input.focus();

        render_into(state, Html::input().value("ab"));
        let again: web_sys::HtmlInputElement =
            state.mount.first_element_child().unwrap().dyn_into().unwrap();

        // Same node, still focused, and the new value flowed through.
        let doc = window().unwrap().document().unwrap();
        let active = doc.active_element().unwrap();
        assert!(
            wasm_bindgen::JsValue::from(again.clone()) == wasm_bindgen::JsValue::from(active),
            "the re-rendered input keeps focus"
        );
        assert_eq!(again.value(), "ab");

        // A re-render with an *unchanged* value must not move the caret.
        again.set_selection_range(1, 1);
        render_into(state, Html::input().value("ab"));
        let once_more: web_sys::HtmlInputElement =
            state.mount.first_element_child().unwrap().dyn_into().unwrap();
        assert_eq!(once_more.selection_start().unwrap(), Some(1));
    }

    #[wasm_bindgen_test]
    fn keyup_delivers_the_key_name() {
        let state = mount("t-keyup");
        install_event_routing(state);
        render_into(state, Html::input().on_key_up(TestMsg::Key));

        let init = web_sys::KeyboardEventInit::new();
        init.set_key("Enter");
        init.set_bubbles(true);
        let ev = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keyup", &init).unwrap();
        state
            .mount
            .first_element_child()
            .unwrap()
            .dispatch_event(&ev)
            .unwrap();

        assert_eq!(
            state.sink.borrow().front(),
            Some(&TestMsg::Key("Enter".to_string()))
        );
    }

    #[wasm_bindgen_test]
    fn submit_is_prevented_even_without_a_handler() {
        let state = mount("t-submit-bare");
        install_event_routing(state);
        // A form the app renders but never binds: it still must not navigate.
        render_into(state, Html::form().child(Html::button().input_type("submit")));

        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        init.set_cancelable(true);
        let ev = web_sys::Event::new_with_event_init_dict("submit", &init).unwrap();
        state
            .mount
            .first_element_child()
            .unwrap()
            .dispatch_event(&ev)
            .unwrap();

        assert!(ev.default_prevented());
        assert!(state.sink.borrow().is_empty());
    }

    #[wasm_bindgen_test]
    fn submit_handler_fires_on_a_prevented_event() {
        let state = mount("t-submit-bound");
        install_event_routing(state);
        render_into(state, Html::form().on_submit(|| TestMsg::Submitted));

        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        init.set_cancelable(true);
        let ev = web_sys::Event::new_with_event_init_dict("submit", &init).unwrap();
        state
            .mount
            .first_element_child()
            .unwrap()
            .dispatch_event(&ev)
            .unwrap();

        assert!(ev.default_prevented());
        assert_eq!(state.sink.borrow().front(), Some(&TestMsg::Submitted));
    }

    #[wasm_bindgen_test]
    fn a_disappearing_value_clears_the_property() {
        let state = mount("t-value-gone");
        render_into(state, Html::input().value("stale"));
        let input: web_sys::HtmlInputElement =
            state.mount.first_element_child().unwrap().dyn_into().unwrap();
        assert_eq!(input.value(), "stale");

        // The attribute is gone; the property must not keep ghost text.
        render_into(state, Html::input());
        assert_eq!(input.value(), "");
    }

    #[wasm_bindgen_test]
    fn never_controlled_fields_are_left_alone() {
        let state = mount("t-value-free");
        render_into(state, Html::input());
        let input: web_sys::HtmlInputElement =
            state.mount.first_element_child().unwrap().dyn_into().unwrap();
        input.set_value("typed by the user");

        // No `value` attribute before or after: re-renders must not clobber
        // an uncontrolled field.
        render_into(state, Html::input().class("again"));
        assert_eq!(input.value(), "typed by the user");
    }

    enum TickMsg {
        Start,
        Done,
    }

    #[derive(Default)]
    struct TickApp;

    impl App for TickApp {
        type Message = TickMsg;
        type Model = bool;

        fn update(&mut self, msg: TickMsg, model: &mut bool) -> Option<Command<TickMsg>> {
            match msg {
                TickMsg::Start => Some(Command::Timeout { millis: 20, msg: TickMsg::Done }),
                TickMsg::Done => {
                    *model = true;
                    None
                }
            }
        }

        fn view(&self, model: &bool) -> Html<TickMsg> {
            Html::div().text(if *model { "done" } else { "pending" })
        }
    }

    /// `Command::Timeout` → message → frame: effects really do come back.
    #[wasm_bindgen_test]
    async fn timeouts_become_messages() {
        let state = mount_as::<TickApp>("t-timeout");
        render_into(state, Html::div().text("pending"));

        state.push(TickMsg::Start);
        state.frame();
        assert_eq!(state.mount.inner_html(), "<div>pending</div>");

        gloo_timers::future::TimeoutFuture::new(80).await;
        // The timeout pushed its message and scheduled a frame; run one
        // manually so the test doesn't race the browser's rAF.
        state.frame();
        assert_eq!(state.mount.inner_html(), "<div>done</div>");
    }

    /// One frame drains the whole sink: a batch of messages yields a single
    /// `update`-per-message pass and exactly one render.
    #[wasm_bindgen_test]
    fn one_frame_processes_a_batch_of_messages() {
        let state = mount_as::<CounterApp>("t-batch");
        // Boot to model 0.
        render_into(state, CounterApp.view(&0));

        for _ in 0..3 {
            state.push(CounterMsg::Inc);
        }
        state.frame();

        assert_eq!(
            state.mount.inner_html(),
            r#"<div class="counter"><span>3</span><button>+1</button></div>"#
        );
        // Everything was drained in that one frame; nothing left pending.
        assert!(state.sink.borrow().is_empty());
    }

    #[wasm_bindgen_test]
    fn keyed_nodes_morph_in_place_when_content_changes() {
        let state = mount("t-keymorph");
        let item = |k: &str, t: &str| Html::li().key(k).text(t);
        render_into(state, Html::ul().children([item("a", "A"), item("b", "B")]));
        let ul = state.mount.first_element_child().unwrap();
        let a_before = ul.first_element_child().unwrap();

        // Same keys, but B's text changed: the keyed "a" node must keep its
        // identity (morph, not rebuild) while the tree updates below it.
        render_into(state, Html::ul().children([item("a", "A"), item("b", "B2")]));
        let ul = state.mount.first_element_child().unwrap();
        assert_eq!(ul.inner_html(), "<li>A</li><li>B2</li>");
        let a_after = ul.first_element_child().unwrap();
        assert!(
            wasm_bindgen::JsValue::from(a_before) == wasm_bindgen::JsValue::from(a_after),
            "a keyed node keeps its identity when content below it changes"
        );
    }

    /// `WithValue` routing: a `change` event delivers the field's current
    /// value as the message payload (here a `<select>`).
    #[wasm_bindgen_test]
    fn change_events_deliver_the_fields_value() {
        let state = mount("t-change");
        install_event_routing(state);
        render_into(
            state,
            Html::select()
                .children([
                    Html::option().value("a").text("A"),
                    Html::option().value("b").text("B"),
                ])
                .on_change(TestMsg::Value),
        );

        let sel: web_sys::HtmlSelectElement =
            state.mount.first_element_child().unwrap().dyn_into().unwrap();
        sel.set_value("b");

        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        let ev = web_sys::Event::new_with_event_init_dict("change", &init).unwrap();
        sel.dispatch_event(&ev).unwrap();

        assert_eq!(state.sink.borrow().front(), Some(&TestMsg::Value("b".to_string())));
    }

    enum PulseMsg {
        Start,
        Double,
        Stop,
        Tick,
    }

    #[derive(Default)]
    struct PulseApp;

    impl App for PulseApp {
        type Message = PulseMsg;
        type Model = u32; // tick count

        fn update(&mut self, msg: PulseMsg, model: &mut u32) -> Option<Command<PulseMsg>> {
            match msg {
                PulseMsg::Start => Some(Command::Every {
                    id: "pulse",
                    millis: 15,
                    msg: Box::new(|| PulseMsg::Tick),
                }),
                // Same id as Start: must be a no-op, not a second interval.
                PulseMsg::Double => Some(Command::Every {
                    id: "pulse",
                    millis: 15,
                    msg: Box::new(|| PulseMsg::Tick),
                }),
                PulseMsg::Stop => Some(Command::Cancel { id: "pulse" }),
                PulseMsg::Tick => {
                    *model += 1;
                    Command::none()
                }
            }
        }

        fn view(&self, model: &u32) -> Html<PulseMsg> {
            Html::div().text(model.to_string())
        }
    }

    /// `Command::Every` keeps firing while subscribed, a duplicate subscribe
    /// is a no-op, and `Command::Cancel` ends the interval for good.
    #[wasm_bindgen_test]
    async fn an_interval_fires_until_cancelled_and_duplicates_are_noops() {
        let state = mount_as::<PulseApp>("t-pulse");
        render_into(state, Html::div().text("0"));

        // Subscribe twice under the same id up front; only one interval runs.
        state.push(PulseMsg::Double);
        state.push(PulseMsg::Start);
        state.frame();

        // Let it tick for a while.
        gloo_timers::future::TimeoutFuture::new(90).await;
        state.frame();
        let first = state.mount.inner_html();
        let n1: u32 = first.trim_start_matches("<div>").trim_end_matches("</div>").parse().unwrap();
        assert!(n1 >= 1, "interval fired at least once, got {n1}");

        // Keep counting over another window. With a single interval and a
        // 15ms period this adds ~6; a duplicated interval would add ~12, so
        // the guard catches a broken no-op while staying way off a slow CI.
        gloo_timers::future::TimeoutFuture::new(90).await;
        state.frame();
        let n2: u32 = state
            .mount
            .inner_html()
            .trim_start_matches("<div>")
            .trim_end_matches("</div>")
            .parse()
            .unwrap();
        let delta = n2 - n1;
        assert!(
            (1..=10).contains(&delta),
            "single-rate growth expected (1..=10), doubled interval would give ~12; got delta {delta}"
        );

        // Cancel: the interval must stop, so the count freezes.
        state.push(PulseMsg::Stop);
        state.frame();
        gloo_timers::future::TimeoutFuture::new(90).await;
        state.frame();
        let n3: u32 = state
            .mount
            .inner_html()
            .trim_start_matches("<div>")
            .trim_end_matches("</div>")
            .parse()
            .unwrap();
        assert_eq!(n3, n2, "cancelling the interval stops the tick count");
    }
}

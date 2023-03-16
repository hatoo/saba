//! https://www.w3.org/TR/css-box-3/
//! https://www.w3.org/TR/css-layout-api-1/
//! https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/layout/layout_view.h

use crate::browser::Browser;
use crate::common::ui::UiObject;
use crate::renderer::css::cssom::*;
use crate::renderer::html::dom::*;
use crate::renderer::layout::computed_style::*;
use crate::renderer::layout::layout_object::*;
use alloc::rc::{Rc, Weak};
use core::cell::RefCell;

fn create_layout_object<U: UiObject>(
    browser: Weak<RefCell<Browser<U>>>,
    node: &Option<Rc<RefCell<Node>>>,
    parent_obj: &Option<Rc<RefCell<LayoutObject<U>>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject<U>>>> {
    match node {
        Some(n) => {
            let layout_object =
                Rc::new(RefCell::new(LayoutObject::new(browser.clone(), n.clone())));
            if let Some(parent) = parent_obj {
                layout_object
                    .borrow_mut()
                    .style
                    .inherit(&parent.borrow().style);
            }

            // apply CSS rules to LayoutObject.
            for rule in &cssom.rules {
                if layout_object.borrow().is_node_selected(&rule.selector) {
                    layout_object
                        .borrow_mut()
                        .set_style(rule.declarations.clone());
                }
            }

            if layout_object.borrow().style.display() == DisplayType::DisplayNone {
                return None;
            }

            Some(layout_object)
        }
        None => None,
    }
}

/// Converts DOM tree to render tree.
fn build_layout_tree<U: UiObject>(
    browser: Weak<RefCell<Browser<U>>>,
    node: &Option<Rc<RefCell<Node>>>,
    parent_obj: &Option<Rc<RefCell<LayoutObject<U>>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject<U>>>> {
    let layout_object = create_layout_object(browser.clone(), &node, parent_obj, cssom);

    if layout_object.is_none() {
        return None;
    }

    match node {
        Some(n) => {
            let original_first_child = n.borrow().first_child();
            let original_next_sibling = n.borrow().next_sibling();
            let mut first_child = build_layout_tree(
                browser.clone(),
                &original_first_child,
                &layout_object,
                cssom,
            );
            let mut next_sibling =
                build_layout_tree(browser.clone(), &original_next_sibling, &None, cssom);

            // if the original first child node is "display:none" and the original first child
            // node has a next sibiling node, treat the next sibling node as a new first child
            // node.
            if first_child.is_none() && original_first_child.is_some() {
                let mut original_dom_node = original_first_child
                    .expect("first child should exist")
                    .borrow()
                    .next_sibling();

                loop {
                    first_child = build_layout_tree(
                        browser.clone(),
                        &original_dom_node,
                        &layout_object,
                        cssom,
                    );

                    // check the next sibling node
                    if first_child.is_none() && original_dom_node.is_some() {
                        original_dom_node = original_dom_node
                            .expect("next sibling should exist")
                            .borrow()
                            .next_sibling();
                        continue;
                    }

                    break;
                }
            }

            // if the original next sibling node is "display:none" and the original next
            // sibling node has a next sibling node, treat the next sibling node as a new next
            // sibling node.
            if next_sibling.is_none() && n.borrow().next_sibling().is_some() {
                let mut original_dom_node = original_next_sibling
                    .expect("first child should exist")
                    .borrow()
                    .next_sibling();

                loop {
                    next_sibling =
                        build_layout_tree(browser.clone(), &original_dom_node, &None, cssom);

                    if next_sibling.is_none() && original_dom_node.is_some() {
                        original_dom_node = original_dom_node
                            .expect("next sibling should exist")
                            .borrow()
                            .next_sibling();
                        continue;
                    }

                    break;
                }
            }

            let obj = match layout_object {
                Some(ref obj) => obj,
                None => panic!("render object should exist here"),
            };
            obj.borrow_mut().first_child = first_child;
            obj.borrow_mut().next_sibling = next_sibling;
        }
        None => {}
    }

    return layout_object;
}

#[derive(Debug, Clone)]
pub struct LayoutView<U: UiObject> {
    browser: Weak<RefCell<Browser<U>>>,
    root: Option<Rc<RefCell<LayoutObject<U>>>>,
}

impl<U: UiObject> LayoutView<U> {
    pub fn new(
        browser: Weak<RefCell<Browser<U>>>,
        root: Rc<RefCell<Node>>,
        cssom: &StyleSheet,
    ) -> Self {
        let mut tree = Self {
            browser: browser.clone(),
            root: build_layout_tree(browser, &Some(root), &None, cssom),
        };

        tree.update_layout();

        tree
    }

    fn layout_node(
        &self,
        node: &Option<Rc<RefCell<LayoutObject<U>>>>,
        parent_style: &ComputedStyle,
        parent_position: &LayoutPosition,
    ) {
        match node {
            Some(n) => {
                n.borrow_mut().update_layout(parent_style, parent_position);

                let first_child = n.borrow().first_child();
                self.layout_node(&first_child, &n.borrow().style, &n.borrow().position);

                let next_sibling = n.borrow().next_sibling();
                self.layout_node(&next_sibling, &n.borrow().style, &n.borrow().position);
            }
            None => return,
        }
    }

    /// Calculate the layout position.
    fn update_layout(&mut self) {
        let fake_node = Rc::new(RefCell::new(Node::new(NodeKind::Document)));
        let fake_style = ComputedStyle::new(&fake_node);
        let fake_position = LayoutPosition::new(0.0, 0.0);
        self.layout_node(&self.root, &fake_style, &fake_position);
    }

    pub fn root(&self) -> Option<Rc<RefCell<LayoutObject<U>>>> {
        self.root.clone()
    }
}

use super::*;
use crate::layout::NodeId;
use crate::ui::View;
use crate::ui::widgets::button;
use crate::{Context, Node};

#[test]
fn test_generational_node_id_bijection() {
    let cases = [
        (0, 0),
        (0, 1),
        (1, 0),
        (42, 100),
        (12345, 67890),
        (u32::MAX - 1, 999999),
    ];

    for (generation, index) in cases {
        let node = Node(NodeId { index, generation });
        let ak_id = to_accesskit_id(node);
        let roundtrip = from_accesskit_id(ak_id);
        assert_eq!(
            node, roundtrip,
            "Failed bijection for generation={generation}, index={index}"
        );
    }
}

#[test]
fn test_accessible_info_builder() {
    let info = AccessibleInfo::new(Role::Button)
        .with_label("Submit")
        .with_value("Form Value")
        .with_description("Submits user details")
        .with_disabled(false)
        .with_toggled(true)
        .with_numeric_range(50.0, 0.0, 100.0, Some(5.0))
        .with_action(Action::Click)
        .with_action(Action::Focus);

    assert_eq!(info.role, Role::Button);
    assert_eq!(info.label.as_deref(), Some("Submit"));
    assert_eq!(info.value.as_deref(), Some("Form Value"));
    assert_eq!(info.description.as_deref(), Some("Submits user details"));
    assert!(!info.disabled);
    assert_eq!(info.toggled, Some(true));
    assert_eq!(info.numeric_value, Some(50.0));
    assert_eq!(info.min_numeric_value, Some(0.0));
    assert_eq!(info.max_numeric_value, Some(100.0));
    assert_eq!(info.step, Some(5.0));
    assert!(info.actions.contains(&Action::Click));
    assert!(info.actions.contains(&Action::Focus));
}

#[test]
fn test_context_accessibility_registration() {
    let mut ctx = Context::new();
    let node = ctx.create_node();

    assert!(ctx.get_accessible(node).is_none());
    assert!(!ctx.dirty_accessibility.contains(&node));

    let info = AccessibleInfo::new(Role::CheckBox).with_label("Agree to Terms");
    ctx.set_accessible(node, info.clone());

    assert_eq!(ctx.get_accessible(node), Some(&info));
    assert!(ctx.dirty_accessibility.contains(&node));

    let removed = ctx.remove_accessible(node);
    assert_eq!(removed, Some(info));
    assert!(ctx.get_accessible(node).is_none());
}

#[test]
fn test_build_accesskit_tree_update_full() {
    let mut ctx = Context::new();
    let root = ctx.create_node();
    ctx.root_attach(root);

    let btn_node = ctx.create_node();
    root.append(&mut ctx, btn_node);
    btn_node.set_accessible(
        &mut ctx,
        AccessibleInfo::new(Role::Button)
            .with_label("Play")
            .with_action(Action::Click),
    );

    let slider_node = ctx.create_node();
    root.append(&mut ctx, slider_node);
    slider_node.set_accessible(
        &mut ctx,
        AccessibleInfo::new(Role::Slider)
            .with_label("Volume")
            .with_numeric_range(75.0, 0.0, 100.0, Some(1.0))
            .with_action(Action::SetValue),
    );

    ctx.compute_layout(800.0, 600.0);
    ctx.request_focus(btn_node);

    let update = ctx.build_accesskit_tree_update(true);

    assert!(update.tree.is_some());
    assert_eq!(update.focus, to_accesskit_id(btn_node));
    assert_eq!(update.nodes.len(), 3);

    let root_ak = update
        .nodes
        .iter()
        .find(|(id, _)| *id == to_accesskit_id(root))
        .map(|(_, n)| n)
        .expect("Root node missing");
    assert_eq!(root_ak.role(), Role::Window);
    assert_eq!(
        root_ak.children(),
        &[to_accesskit_id(btn_node), to_accesskit_id(slider_node)]
    );

    let btn_ak = update
        .nodes
        .iter()
        .find(|(id, _)| *id == to_accesskit_id(btn_node))
        .map(|(_, n)| n)
        .expect("Button node missing");
    assert_eq!(btn_ak.role(), Role::Button);
    assert_eq!(btn_ak.label(), Some("Play"));
    assert!(btn_ak.supports_action(Action::Click));

    let slider_ak = update
        .nodes
        .iter()
        .find(|(id, _)| *id == to_accesskit_id(slider_node))
        .map(|(_, n)| n)
        .expect("Slider node missing");
    assert_eq!(slider_ak.role(), Role::Slider);
    assert_eq!(slider_ak.label(), Some("Volume"));
    assert_eq!(slider_ak.numeric_value(), Some(75.0));
}

#[test]
fn test_view_accessible_modifier() {
    let mut ctx = Context::new();
    let root = ctx.create_node();
    ctx.root_attach(root);

    let my_view: AccessibleView<(), _> = button("Custom")
        .on_click(())
        .accessible_label("Accessible Custom Button");
    let mut element = my_view.build(&mut ctx);
    let node = my_view.get_node(&element);

    let info = node.get_accessible(&ctx).expect("Accessible info missing");
    assert_eq!(info.label.as_deref(), Some("Accessible Custom Button"));

    my_view.teardown(&mut ctx, &mut element);
    assert!(node.get_accessible(&ctx).is_none());
}

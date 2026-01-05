use gtk4::prelude::*;
use gtk4::{Align, Box, Button, Image, Label, Orientation};

/// Create a tile styled like GNOME 47 quick settings
pub fn tile(icon_name: &str, primary: &str, secondary: &str) -> Button {
    let (btn, _, _) = tile_with_labels(icon_name, primary, secondary);
    btn
}

/// Create a tile and return it with label references for dynamic updates
pub fn tile_with_labels(icon_name: &str, primary: &str, secondary: &str) -> (Button, Label, Label) {
    let button = Button::builder()
        .halign(Align::Fill)
        .valign(Align::Fill)
        .css_classes(["qs-tile"])
        .build();

    let outer = Box::new(Orientation::Horizontal, 12);
    outer.set_margin_start(12);
    outer.set_margin_end(12);
    outer.set_margin_top(12);
    outer.set_margin_bottom(12);
    outer.set_valign(Align::Center);
    outer.set_halign(Align::Fill);

    let icon = Image::from_icon_name(icon_name);
    icon.set_pixel_size(20);

    let text_box = Box::new(Orientation::Vertical, 4);
    let primary_label = Label::builder()
        .label(primary)
        .xalign(0.0)
        .css_classes(vec!["qs-primary"])
        .build();
    let secondary_label = Label::builder()
        .label(secondary)
        .xalign(0.0)
        .css_classes(vec!["qs-secondary"])
        .build();

    text_box.append(&primary_label);
    text_box.append(&secondary_label);

    outer.append(&icon);
    outer.append(&text_box);
    button.set_child(Some(&outer));

    (button, primary_label, secondary_label)
}

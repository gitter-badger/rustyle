mod background_color;
mod clear;
mod color;
mod direction;
mod display;
mod empty_cells;
mod float;
mod font;
mod height;
mod letter_spacing;
mod list_style_position;
mod list_style_type;
mod min_height;
mod user_select;

use crate::core::property::Property;

pub fn register_all_properties() {
    background_color::Instance.register();
    user_select::Instance.register();
    color::Instance.register();
    list_style_type::Instance.register();
    list_style_position::Instance.register();
    clear::Instance.register();
    direction::Instance.register();
    display::Instance.register();
    empty_cells::Instance.register();
    float::Instance.register();
    min_height::Instance.register();
    letter_spacing::Instance.register();
    height::Instance.register();

    font::register_properties();
}

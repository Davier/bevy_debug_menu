pub mod check_box;
pub mod ecr_tree;
pub mod input_box;
pub mod radio_button;
pub mod tree_node;

pub use check_box::BuildCheckBox;
pub use input_box::BuildInputBox;
pub use radio_button::BuildRadioButtons;
pub use tree_node::BuildTreeNode;

use bevy::prelude::{stage, AppBuilder, IntoSystem, Plugin};

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(ecr_tree::ecr::update_system.system())
            .add_system_to_stage(
                /*stage::POST_UPDATE*/ stage::UPDATE,
                ecr_tree::ecr::update_entity_labels.system(),
            ) // listens to Mutated<Name>, Mutated<Label> and Added<EntityLabel>
            .add_system(ecr_tree::leaf::update_checkbox_system.system())
            .add_system(ecr_tree::leaf::update_inputbox_system.system())
            .add_event::<input_box::UnfocusedEvent>()
            .add_event::<input_box::FocusedEvent>()
            .add_system(input_box::interact_mouse_system.system())
            .add_system(input_box::interact_keyboard_system.system())
            .add_event::<tree_node::ExpandedEvent>()
            .add_system(tree_node::interact_button_system.system())
            .add_system(ecr_tree::node::update_node_system.system()) // after tree_node::interact_button_system
            .add_event::<radio_button::SelectionChangedEvent>()
            .add_system(radio_button::interact_system.system())
            .add_system(check_box::interact_system.system())
            .add_system_to_stage(
                stage::POST_UPDATE,
                check_box::update_mutated_system.system(),
            ) // listens to Mutated<Interaction>, Mutated<CheckBox>
            .add_event::<check_box::ToggledEvent>();
    }
}

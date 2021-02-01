use bevy::{
    core::AsBytes,
    ecs::Commands,
    input::mouse::MouseWheel,
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
    ui,
};

use crate::{diagnostic, entity, resource, scene, widgets::*};

pub struct DebugMenuPlugin;

impl Plugin for DebugMenuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Style>()
            // .register_type::<wgpu::AdapterInfo>()
            .add_startup_system(spawn_system.system())
            .add_system(update_system.system())
            .add_system(handle_inputs_system.system())
            .add_system(selection_changed_event_system.system())
            .add_system(diagnostic::update_system.system())
            .add_system(entity::update_system.system()) // after check_box::update_mutated_system
            .add_system(scene::interact_save_button.system());
        #[cfg(feature = "extra")]
        app.init_resource::<resource::TestResource>()
            .register_type::<resource::TestResource>()
            .add_system(resource::update_system.system()); // after check_box::update_mutated_system
        app.add_plugin(WidgetsPlugin);
    }
}

// Marker component to filter out all of the debug menu's entities from the entity list
#[derive(Debug, Clone, Copy)]
pub struct DebugIgnore;

#[derive(Debug)]
struct DebugMenu {
    menu_container: Entity,
    selected_panel: Panel,
    scrolling_position: f32,
    show_progress: f32,
    show: bool,
}

#[derive(Debug)]
enum Panel {
    Default(Entity),
    Diagnostic(Entity),
    Entity(Entity),
    Resource(Entity),
    Scene(Entity),
}
impl Panel {
    fn get_entity(&self) -> Entity {
        match self {
            Panel::Default(e) => *e,
            Panel::Diagnostic(e) => *e,
            Panel::Entity(e) => *e,
            Panel::Resource(e) => *e,
            Panel::Scene(e) => *e,
        }
    }
}

#[derive(Debug)]
pub struct Style {
    pub font: Handle<Font>,
    pub color_background: Handle<ColorMaterial>,
    pub color_title_text: Color,
    pub style_menu: radio_button::Style,
    pub style_diagnostic: diagnostic::Style,
    pub style_list: ecr_tree::Style,
    pub style_scene: scene::Style,
    #[cfg(feature = "extra")]
    pub z_index: ui::ZIndex,
}

impl FromResources for Style {
    fn from_resources(resources: &Resources) -> Self {
        let mut materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
        let mut assets_font = resources.get_mut::<Assets<Font>>().unwrap();
        let font = {
            // const FONT: &[u8] = include_bytes!("../assets/FiraSans-Bold.ttf");
            const FONT: &[u8] = include_bytes!("../assets/test.ttf");
            assets_font.add(Font::try_from_bytes(FONT.into()).unwrap())
        };
        let font_mono = {
            const FONT: &[u8] = include_bytes!("../assets/FiraMono-Bold.ttf");
            assets_font.add(Font::try_from_bytes(FONT.into()).unwrap())
        };
        let mut assets_texture = resources.get_mut::<Assets<Texture>>().unwrap();
        let icon_chevron_down = {
            const ICON_CHEVRON_DOWN: &[u8] = include_bytes!("../assets/chevron-down.png");
            materials.add(ColorMaterial::texture(
                assets_texture.add(load_texture(ICON_CHEVRON_DOWN)),
            ))
        };
        let icon_chevron_up = {
            const ICON_CHEVRON_UP: &[u8] = include_bytes!("../assets/chevron-up.png");
            materials.add(ColorMaterial::texture(
                assets_texture.add(load_texture(ICON_CHEVRON_UP)),
            ))
        };
        let icon_toggle_on = {
            const ICON_TOGGLE_ON: &[u8] = include_bytes!("../assets/toggle-on.png");
            materials.add(ColorMaterial::texture(
                assets_texture.add(load_texture(ICON_TOGGLE_ON)),
            ))
        };
        let icon_toggle_off = {
            const ICON_TOGGLE_OFF: &[u8] = include_bytes!("../assets/toggle-off.png");
            materials.add(ColorMaterial::texture(
                assets_texture.add(load_texture(ICON_TOGGLE_OFF)),
            ))
        };
        let icon_toggle_off_hovered = {
            const ICON_TOGGLE_OFF_HOVERED: &[u8] =
                include_bytes!("../assets/toggle-off-hovered.png");
            materials.add(ColorMaterial::texture(
                assets_texture.add(load_texture(ICON_TOGGLE_OFF_HOVERED)),
            ))
        };

        let style_tree_node = tree_node::Style {
            color_root_container: materials.add(Color::NONE.into()),
            color_button: materials.add(Color::BLACK.into()),
            color_button_hovered: Some(materials.add(Color::MIDNIGHT_BLUE.into())),
            color_button_clicked: Some(materials.add(Color::ALICE_BLUE.into())),
            color_button_expanded: Some(materials.add(Color::MIDNIGHT_BLUE.into())),
            color_children_container: materials.add(Color::rgba(0.2, 0.2, 0.2, 0.8).into()),
            node_style_button: ui::Style {
                flex_shrink: 0.,
                margin: Rect {
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(4.0),
                    bottom: Val::Px(0.0),
                },
                ..Default::default()
            },
            node_style_children_container: ui::Style {
                flex_shrink: 0.,
                flex_direction: FlexDirection::ColumnReverse,
                margin: Rect {
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                },
                padding: Rect {
                    left: Val::Px(8.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(4.0),
                },
                ..Default::default()
            },
        };

        let style_menu = radio_button::Style {
            style_container: ui::Style {
                padding: Rect::all(Val::Px(2.0)),
                flex_shrink: 0.,
                flex_wrap: FlexWrap::Wrap,
                #[cfg(feature = "extra")]
                z_index: ZIndex::Some(1), // FIXME: add a container for the whole header
                ..Default::default()
            },
            color_container: style_tree_node.color_children_container.clone(),
            style_button: ui::Style {
                flex_direction: FlexDirection::ColumnReverse,
                flex_grow: 1.0,
                margin: Rect::all(Val::Px(2.0)),
                ..Default::default()
            },
            color_button: style_tree_node.color_button.clone(),
            color_button_hovered: style_tree_node.color_button_hovered.clone(),
            color_button_clicked: style_tree_node.color_button_clicked.clone(),
            color_button_selected: style_tree_node.color_button_hovered.clone(),
        };

        let color_background = materials.add(Color::rgb(0.8, 0.8, 0.8).into());

        let style_diagnostic = diagnostic::Style {
            font: font_mono,
            color_background: color_background.clone(),
            color_box: style_tree_node.color_button.clone(),
            style_box: ui::Style {
                flex_shrink: 0.,
                align_items: AlignItems::Center,
                margin: Rect {
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(4.0),
                    bottom: Val::Px(0.0),
                },
                ..Default::default()
            },
            font_size: 18.,
        };

        let style_list = ecr_tree::Style {
            font: font.clone(),
            color_background: color_background.clone(),
            style_node: style_tree_node,
            color_node_text: Color::WHITE,
            style_input_box: input_box::Style {
                color_background: materials.add(Color::WHITE.into()),
                font: font.clone(),
            },
            icon_chevron_down,
            icon_chevron_up,
            style_check_box: check_box::Style {
                icon_toggle_on,
                icon_toggle_off,
                icon_toggle_off_hovered: Some(icon_toggle_off_hovered),
                icon_toggle_on_hovered: None,
            },
        };

        let style_scene = scene::Style {
            font: font.clone(),
            font_size: 18.0,
            color_background: color_background.clone(),
        };

        Style {
            color_background,
            font,
            color_title_text: Color::BLACK,
            style_menu,
            // style_entity_list,
            style_diagnostic,
            style_list,
            style_scene,
            // In front of default layers
            #[cfg(feature = "extra")]
            z_index: ZIndex::Some(10),
        }
    }
}

fn spawn_system(commands: &mut Commands, style: Res<Style>) {
    trace!("inserting main panel");
    let mut ui_style_root = ui::Style {
        // Define absolute position and size for the main container
        position_type: PositionType::Absolute,
        size: Size {
            width: Val::Percent(40.),
            height: Val::Percent(100.),
        },
        // Flow from top to bottom
        flex_direction: FlexDirection::ColumnReverse,
        // Align at the top of the screen
        align_self: AlignSelf::FlexEnd,
        // Content is at the top
        justify_content: JustifyContent::FlexStart,
        ..Default::default()
    };
    set_zindex(&mut ui_style_root, &style);
    #[cfg(feature = "extra")]
    fn set_zindex(ui_style_root: &mut ui::Style, style: &Style) {
        ui_style_root.z_index = style.z_index;
    }
    #[cfg(not(feature = "extra"))]
    fn set_zindex(_ui_style_root: &mut ui::Style, _style: &Style) {}

    let container = commands
        .spawn(NodeBundle {
            style: ui_style_root,
            material: style.color_background.clone(),
            ..Default::default()
        })
        .with(DebugIgnore)
        .with_children(|parent| {
            parent
                .spawn(TextBundle {
                    style: ui::Style {
                        align_self: AlignSelf::Center,
                        size: Size {
                            width: Val::Undefined,
                            height: Val::Px(32.0), // Same as font_size
                        },
                        flex_shrink: 0.,
                        ..Default::default()
                    },
                    text: Text::with_section(
                        "Debug Menu".to_string(),
                        TextStyle {
                            font: style.font.clone(),
                            font_size: 32.0,
                            color: style.color_title_text,
                        },
                        TextAlignment {
                            horizontal: HorizontalAlign::Center,
                            vertical: VerticalAlign::Center,
                        },
                    ),
                    ..Default::default()
                })
                .with(DebugIgnore);
        })
        .current_entity()
        .unwrap();
    let menus = ["Diagnostics", "Entities", "Resources", "Scenes"];
    let mut radio_button = None;
    commands.with_children(|parent| {
        radio_button = Some(parent.spawn_radio_buttons(
            menus.len(),
            None,
            style.style_menu.clone(),
            Some(ecr_tree::with_debug_ignore),
        ));
    });

    for (i, entity) in radio_button.as_ref().unwrap().buttons.iter().enumerate() {
        commands.set_current_entity(*entity);
        commands.with_children(|parent| {
            parent
                .spawn(TextBundle {
                    text: Text::with_section(
                        menus[i].to_string(),
                        TextStyle {
                            font: style.font.clone(),
                            font_size: 28.0,
                            ..Default::default()
                        },
                        Default::default(),
                    ),
                    style: ui::Style {
                        align_self: AlignSelf::Center,
                        size: Size {
                            width: Val::Undefined,
                            height: Val::Px(28.0), // Same as font_size
                        },
                        flex_shrink: 0.,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(DebugIgnore);
        });
    }

    let mut default_panel = None;
    commands.set_current_entity(container);
    commands.with_children(|parent| {
        parent.spawn(NodeBundle::default());
        default_panel = parent.current_entity();
    });

    commands.set_current_entity(container);
    commands.with(DebugMenu {
        menu_container: radio_button.as_ref().unwrap().widget,
        selected_panel: Panel::Default(default_panel.unwrap()),
        scrolling_position: Default::default(),
        show_progress: Default::default(),
        show: Default::default(),
    });
    // println!("{}", r#"{"reason":"compiler-message","package_id":"test_bevy 0.1.0 (path+file:///home/davierb/prog/ltu_prototype)","target":{"kind":["bin"],"crate_types":["bin"],"name":"test_bevy","src_path":"/home/davierb/prog/ltu_prototype/src/main.rs","edition":"2018","doc":true,"doctest":false,"test":true},"message":{"rendered":"warning: unused import: `bevy::prelude::*`\n --> src/test.rs:1:5\n  |\n1 | use bevy::prelude::*;\n  |     ^^^^^^^^^^^^^^^^\n  |\n  = note: `#[warn(unused_imports)]` on by default\n\n","children":[{"children":[],"code":null,"level":"note","message":"`#[warn(unused_imports)]` on by default","rendered":null,"spans":[]},{"children":[],"code":null,"level":"help","message":"remove the whole `use` item","rendered":null,"spans":[{"byte_end":21,"byte_start":0,"column_end":22,"column_start":1,"expansion":null,"file_name":"src/test.rs","is_primary":true,"label":null,"line_end":1,"line_start":1,"suggested_replacement":"","suggestion_applicability":"MachineApplicable","text":[{"highlight_end":22,"highlight_start":1,"text":"use bevy::prelude::*;"}]}]}],"code":{"code":"unused_imports","explanation":null},"level":"warning","message":"unused import: `bevy::prelude::*`","spans":[{"byte_end":20,"byte_start":4,"column_end":21,"column_start":5,"expansion":null,"file_name":"src/test.rs","is_primary":true,"label":null,"line_end":1,"line_start":1,"suggested_replacement":null,"suggestion_applicability":null,"text":[{"highlight_end":21,"highlight_start":5,"text":"use bevy::prelude::*;"}]}]}}"#);
}

fn update_system(
    time: Res<Time>,
    windows: Res<Windows>,
    mut query_debug_menu: Query<(Entity, &mut DebugMenu), With<DebugIgnore>>,
    mut query_style: Query<(&mut ui::Style, &Node), With<DebugIgnore>>,
) {
    if let Some((entity, mut debug_menu)) = query_debug_menu.iter_mut().next() {
        // Horizontal transition
        if let Ok((mut debug_menu_style, _)) = query_style.get_mut(entity) {
            if debug_menu.show_progress >= 0.0 {
                let transition_time = 0.2;
                debug_menu.show_progress -= time.delta_seconds() / transition_time;
                let panel_width = match debug_menu_style.size.width {
                    Val::Percent(x) => x,
                    _ => todo!(),
                };
                let (origin, target) = if debug_menu.show {
                    (-panel_width, 0.0)
                } else {
                    (0.0, -panel_width)
                };
                use interpolation::*;
                let new_position_left = Val::Percent(
                    origin
                        + (1.0 - debug_menu.show_progress).quadratic_in_out() * (target - origin),
                );
                if new_position_left != debug_menu_style.position.left {
                    debug_menu_style.position.left = new_position_left;
                }
            }
        }

        if let Ok((mut panel_style, panel_node)) =
            query_style.get_mut(debug_menu.selected_panel.get_entity())
        {
            let window_height = windows.get_primary().unwrap().height();
            let header_height = window_height * 0.15;
            let panel_height = panel_node.size.y;
            let max_scroll = window_height - panel_height - header_height; // FIXME: use bottom pos of panel - botton pos of window

            // Vertical scrolling
            if debug_menu.scrolling_position < max_scroll {
                debug_menu.scrolling_position = max_scroll;
            }
            if debug_menu.scrolling_position > 0. {
                debug_menu.scrolling_position = 0.;
            }
            let new_position_top = Val::Px(debug_menu.scrolling_position);
            if new_position_top != panel_style.position.top {
                panel_style.position.top = new_position_top;
            }
        }
    }
}

fn handle_inputs_system(
    mut mousewheel_events: EventReader<MouseWheel>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut DebugMenu>,
) {
    if let Some(mut debug_menu) = query.iter_mut().next() {
        for ev in mousewheel_events.iter() {
            // TODO: check if mouse is over the menu
            debug_menu.scrolling_position += ev.y * 40.0;
        }
        if keyboard_input.just_pressed(KeyCode::F10) {
            debug_menu.show = !debug_menu.show;
            debug_menu.show_progress = 1.0;
        }
    }
}

fn selection_changed_event_system(
    commands: &mut Commands,
    mut radio_button_events: EventReader<radio_button::SelectionChangedEvent>,
    mut query: Query<(Entity, &mut DebugMenu)>,
    style: Res<Style>,
) {
    if let Some((debug_menu_entity, mut debug_menu)) = query.iter_mut().next() {
        for event in radio_button_events.iter() {
            if event.widget == debug_menu.menu_container {
                let previous_panel = debug_menu.selected_panel.get_entity();
                commands.despawn_recursive(previous_panel);
                commands.set_current_entity(debug_menu_entity);
                match event.new_selection {
                    None => {
                        let mut default_panel = None;
                        commands.with_children(|parent| {
                            parent.spawn(NodeBundle::default());
                            default_panel = parent.current_entity();
                        });
                        debug_menu.selected_panel = Panel::Default(default_panel.unwrap());
                    }
                    Some(0) => {
                        let diagnostic_list_container =
                            diagnostic::spawn(commands, &style.style_diagnostic);
                        debug_menu.selected_panel = Panel::Diagnostic(diagnostic_list_container);
                    }
                    Some(1) => {
                        let entity_list_container = entity::spawn(commands, &style.style_list);
                        debug_menu.selected_panel = Panel::Entity(entity_list_container);
                    }
                    Some(2) => {
                        let resource_list_container = resource::spawn(commands, &style.style_list);
                        debug_menu.selected_panel = Panel::Resource(resource_list_container);
                    }
                    Some(3) => {
                        let scene_container = scene::spawn(commands, &style.style_scene);
                        debug_menu.selected_panel = Panel::Scene(scene_container);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

// Extracted from ImageTextureLoader::load()
fn load_texture(bytes: &[u8]) -> Texture {
    let dyn_img = image::load_from_memory(bytes).unwrap();
    let width;
    let height;

    let data: Vec<u8>;
    let format: TextureFormat;

    match dyn_img {
        image::DynamicImage::ImageLuma8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::R8Unorm;

            data = i.into_raw();
        }
        image::DynamicImage::ImageLumaA8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rg8Unorm;

            data = i.into_raw();
        }
        image::DynamicImage::ImageRgb8(i) => {
            let i = image::DynamicImage::ImageRgb8(i).into_rgba8();
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba8UnormSrgb;

            data = i.into_raw();
        }
        image::DynamicImage::ImageRgba8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba8UnormSrgb;

            data = i.into_raw();
        }
        image::DynamicImage::ImageBgr8(i) => {
            let i = image::DynamicImage::ImageBgr8(i).into_bgra8();

            width = i.width();
            height = i.height();
            format = TextureFormat::Bgra8UnormSrgb;

            data = i.into_raw();
        }
        image::DynamicImage::ImageBgra8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Bgra8UnormSrgb;

            data = i.into_raw();
        }
        image::DynamicImage::ImageLuma16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::R16Uint;

            let raw_data = i.into_raw();

            data = raw_data.as_slice().as_bytes().to_owned();
        }
        image::DynamicImage::ImageLumaA16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rg16Uint;

            let raw_data = i.into_raw();

            data = raw_data.as_slice().as_bytes().to_owned();
        }

        image::DynamicImage::ImageRgb16(image) => {
            width = image.width();
            height = image.height();
            format = TextureFormat::Rgba16Uint;

            let mut local_data =
                Vec::with_capacity(width as usize * height as usize * format.pixel_size());

            for pixel in image.into_raw().chunks_exact(3) {
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];
                let a = u16::max_value();

                local_data.extend_from_slice(&r.to_ne_bytes());
                local_data.extend_from_slice(&g.to_ne_bytes());
                local_data.extend_from_slice(&b.to_ne_bytes());
                local_data.extend_from_slice(&a.to_ne_bytes());
            }

            data = local_data;
        }
        image::DynamicImage::ImageRgba16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba16Uint;

            let raw_data = i.into_raw();

            data = raw_data.as_slice().as_bytes().to_owned();
        }
    }

    Texture::new(
        Extent3d::new(width, height, 1),
        TextureDimension::D2,
        data,
        format,
    )
}

pub fn setup_ui_camera(commands: &mut Commands) {
    commands.spawn(UiCameraBundle::default());
}

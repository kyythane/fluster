use fluster_core::actions::{
    Action, ActionList, EntityDefinition, EntityUpdateDefinition, PartDefinition,
    PartUpdateDefinition,
};
use fluster_core::rendering::{AugmentedShape, Coloring, MorphPoint, Point, Shape};
use fluster_core::runner;
use fluster_core::tween::Easing;
use fluster_core::types::ScaleRotationTranslation;
use fluster_graphics::FlusterRenderer;
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::f32::consts::PI;
use uuid::Uuid;

fn build_action_list() -> ActionList {
    let shape_id = Uuid::new_v4();
    let shape2_id = Uuid::new_v4();
    let shape3_id = Uuid::new_v4();
    let shape4_id = Uuid::new_v4();
    let shape5_id = Uuid::new_v4();
    let root_id = Uuid::new_v4();
    let entity_id = Uuid::new_v4();
    let entity2_id = Uuid::new_v4();
    let actions = vec![
        Action::SetBackground {
            color: ColorU::new(254, 200, 216, 255),
        },
        Action::CreateRoot(root_id),
        Action::EndInitialization,
        Action::DefineShape {
            id: shape_id,
            shape: Shape::Fill {
                points: vec![
                    Point::Line(Vector2F::new(-15.0, -15.0)),
                    Point::Line(Vector2F::new(15.0, -15.0)),
                    Point::Line(Vector2F::new(15.0, 15.0)),
                    Point::Line(Vector2F::new(-15.0, 15.0)),
                ],
                color: ColorU::new(149, 125, 173, 255),
            },
        },
        Action::DefineShape {
            id: shape2_id,
            shape: Shape::Path {
                points: vec![
                    Point::Line(Vector2F::new(-15.0, -15.0)),
                    Point::Line(Vector2F::new(15.0, -15.0)),
                    Point::Line(Vector2F::new(15.0, 15.0)),
                    Point::Line(Vector2F::new(-15.0, 15.0)),
                ],
                stroke_style: StrokeStyle {
                    line_width: 3.0,
                    line_cap: LineCap::Square,
                    line_join: LineJoin::Bevel,
                },
                is_closed: true,
                color: ColorU::black(),
            },
        },
        Action::DefineShape {
            id: shape3_id,
            shape: Shape::Group {
                shapes: vec![
                    AugmentedShape {
                        shape: Shape::Fill {
                            points: vec![
                                Point::Line(Vector2F::new(-15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, 15.0)),
                                Point::Line(Vector2F::new(-15.0, 15.0)),
                            ],
                            color: ColorU::new(149, 125, 173, 255),
                        },
                        transform: Transform2F::from_scale_rotation_translation(
                            Vector2F::splat(1.0),
                            0.0,
                            Vector2F::new(-100.0, -100.0),
                        ),
                    },
                    AugmentedShape {
                        shape: Shape::Fill {
                            points: vec![
                                Point::Line(Vector2F::new(-15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, 15.0)),
                                Point::Line(Vector2F::new(-15.0, 15.0)),
                            ],
                            color: ColorU::new(149, 125, 173, 255),
                        },
                        transform: Transform2F::from_scale_rotation_translation(
                            Vector2F::splat(1.0),
                            0.0,
                            Vector2F::new(100.0, -100.0),
                        ),
                    },
                    AugmentedShape {
                        shape: Shape::Fill {
                            points: vec![
                                Point::Line(Vector2F::new(-15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, 15.0)),
                                Point::Line(Vector2F::new(-15.0, 15.0)),
                            ],
                            color: ColorU::new(149, 125, 173, 255),
                        },
                        transform: Transform2F::from_scale_rotation_translation(
                            Vector2F::splat(1.0),
                            0.0,
                            Vector2F::new(100.0, 100.0),
                        ),
                    },
                    AugmentedShape {
                        shape: Shape::Fill {
                            points: vec![
                                Point::Line(Vector2F::new(-15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, -15.0)),
                                Point::Line(Vector2F::new(15.0, 15.0)),
                                Point::Line(Vector2F::new(-15.0, 15.0)),
                            ],
                            color: ColorU::new(149, 125, 173, 255),
                        },
                        transform: Transform2F::from_scale_rotation_translation(
                            Vector2F::splat(1.0),
                            0.0,
                            Vector2F::new(-100.0, 100.0),
                        ),
                    },
                ],
            },
        },
        Action::DefineShape {
            id: shape4_id,
            shape: Shape::Path {
                points: vec![
                    Point::Move(Vector2F::new(300.0, 100.0)),
                    Point::Line(Vector2F::new(258.0, 142.0)),
                    Point::Bezier {
                        control_1: Vector2F::new(322.0, 160.0),
                        control_2: Vector2F::new(326.0, 150.0),
                        to: Vector2F::new(330.0, 142.0),
                    },
                    Point::Quadratic {
                        control: Vector2F::new(240.0, 100.0),
                        to: Vector2F::new(330.0, 62.0),
                    },
                    Point::Move(Vector2F::new(330.0, 130.0)),
                    Point::Arc {
                        control: Vector2F::new(300.0, 100.0),
                        to: Vector2F::new(360.0, 92.0),
                        radius: 21.0,
                    },
                ],
                stroke_style: StrokeStyle {
                    line_width: 3.0,
                    line_cap: LineCap::Square,
                    line_join: LineJoin::Bevel,
                },
                is_closed: false,
                color: ColorU::black(),
            },
        },
        Action::DefineShape {
            id: shape5_id,
            shape: Shape::MorphPath {
                points: vec![
                    MorphPoint::Line(Vector2F::new(-15.0, -15.0), Vector2F::new(-18.0, -12.0)),
                    MorphPoint::Line(Vector2F::new(15.0, -15.0), Vector2F::new(0.0, -22.0)),
                    MorphPoint::Line(Vector2F::new(15.0, 15.0), Vector2F::new(30.0, 15.0)),
                    MorphPoint::Line(Vector2F::new(-15.0, 15.0), Vector2F::new(-11.0, 33.0)),
                ],
                stroke_style: StrokeStyle {
                    line_width: 3.0,
                    line_cap: LineCap::Square,
                    line_join: LineJoin::Bevel,
                },
                is_closed: true,
                color: ColorU::white(),
            },
        },
        Action::AddEntity(EntityDefinition {
            id: entity_id,
            name: String::from("first"),
            transform: Transform2F::from_scale_rotation_translation(
                Vector2F::splat(0.5),
                0.0,
                Vector2F::new(400.0, 400.0),
            ),
            depth: 2,
            parts: vec![
                PartDefinition::Vector {
                    item_id: shape2_id,
                    transform: Transform2F::from_scale_rotation_translation(
                        Vector2F::splat(2.0),
                        0.0,
                        Vector2F::splat(0.0),
                    ),
                },
                PartDefinition::Vector {
                    item_id: shape_id,
                    transform: Transform2F::from_scale_rotation_translation(
                        Vector2F::splat(2.0),
                        0.0,
                        Vector2F::splat(0.0),
                    ),
                },
                PartDefinition::Vector {
                    item_id: shape5_id,
                    transform: Transform2F::from_scale_rotation_translation(
                        Vector2F::splat(2.0),
                        0.0,
                        Vector2F::new(300.0, 300.0),
                    ),
                },
                PartDefinition::Vector {
                    item_id: shape4_id,
                    transform: Transform2F::from_scale_rotation_translation(
                        Vector2F::splat(2.0),
                        0.0,
                        Vector2F::new(0.0, 0.0),
                    ),
                },
            ],
            parent: None,
            morph_index: 0.0,
        }),
        Action::AddEntity(EntityDefinition {
            id: entity2_id,
            name: String::from("second"),
            transform: Transform2F::default(),
            depth: 3,
            parts: vec![PartDefinition::Vector {
                item_id: shape3_id,
                transform: Transform2F::default(),
            }],
            parent: Some(entity_id),
            morph_index: 0.0,
        }),
        Action::PresentFrame(0, 1),
        Action::UpdateEntity(EntityUpdateDefinition {
            duration_frames: 480,
            easing: Some(Easing::BounceOut),
            id: entity2_id,
            part_updates: vec![],
            transform: Some(ScaleRotationTranslation {
                scale: Vector2F::splat(1.0),
                theta: PI,
                translation: Vector2F::new(200.0, 0.0),
            }),
            morph_index: None,
        }),
        Action::UpdateEntity(EntityUpdateDefinition {
            duration_frames: 360,
            easing: Some(Easing::QuinticIn),
            id: entity_id,
            part_updates: vec![],
            transform: None,
            morph_index: Some(1.0),
        }),
        Action::PresentFrame(1, 239),
        Action::UpdateEntity(EntityUpdateDefinition {
            duration_frames: 300,
            easing: None,
            id: entity2_id,
            part_updates: vec![PartUpdateDefinition::Vector {
                color: Some(Coloring::Colorings(vec![
                    Coloring::Color(ColorU::new(210, 145, 188, 255)),
                    Coloring::Color(ColorU::new(224, 187, 228, 255)),
                    Coloring::Color(ColorU::new(210, 145, 188, 255)),
                    Coloring::Color(ColorU::new(255, 223, 211, 255)),
                ])),
                easing: Easing::CubicInOut,
                item_id: shape3_id,
                transform: None,
            }],
            transform: None,
            morph_index: None,
        }),
        Action::PresentFrame(240, 600),
        Action::Quit,
    ];
    ActionList::new(Box::new(|| None), Some(&actions))
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(GLProfile::Core);
    gl_attributes.set_context_version(3, 3);
    let window_size = Vector2I::new(800, 600);
    let window = video
        .window(
            "Fluster demo",
            window_size.x() as u32,
            window_size.y() as u32,
        )
        .opengl()
        .build()
        .unwrap();

    let gl_context = window.gl_create_context().unwrap();
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    window.gl_make_current(&gl_context).unwrap();

    let renderer = Renderer::new(
        GLDevice::new(GLVersion::GL3, 0),
        &FilesystemResourceLoader::locate(),
        DestFramebuffer::full_window(window_size),
        RendererOptions {
            background_color: Some(ColorF::white()),
        },
    );

    let font_context = CanvasFontContext::from_system_source();

    let mut fluster_renderer = FlusterRenderer::new(
        font_context,
        renderer,
        Box::new(move || window.gl_swap_window()),
    );

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut end_of_frame_callback = move |state: runner::State| {
        let mut state = state;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    state.set_running(false);
                }
                _ => {}
            }
        }
        state
    };

    let mut action_list = build_action_list();

    match runner::play(
        &mut fluster_renderer,
        &mut action_list,
        &mut end_of_frame_callback,
        1.0 / 60.0,
        window_size.to_f32(),
    ) {
        Err(message) => println!("{}", message),
        _ => {}
    }
}

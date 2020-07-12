use fluster_core::actions::{
    Action, ActionList, ContainerCreationDefintition, ContainerCreationProperty,
    ContainerUpdateDefintition, ContainerUpdateProperty,
};
use fluster_core::runner;
use fluster_core::tween::Easing;
use fluster_core::types::{
    basic::ScaleRotationTranslation,
    coloring::{ColorSpace, Coloring},
    shapes::{AugmentedShape, Edge, MorphEdge, Shape},
};
use fluster_graphics::FlusterRendererImpl;
use palette::{LinSrgba, Srgb, Srgba};
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::ColorF;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::embedded::EmbeddedResourceLoader;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::{f32::consts::PI, time::Duration};
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
            color: Srgb::<f32>::from_format(Srgb::<u8>::new(254, 200, 216)).into_linear(),
        },
        Action::CreateRoot(root_id),
        Action::EndInitialization,
        Action::DefineShape {
            id: shape_id,
            shape: Shape::Fill {
                edges: Edge::new_rect(
                    Vector2F::splat(30.0),
                    Transform2F::from_translation(Vector2F::splat(-15.0)),
                ),
                color: Srgba::<f32>::from_format(Srgba::<u8>::new(149, 125, 173, 255))
                    .into_linear(),
            },
        },
        Action::DefineShape {
            id: shape2_id,
            shape: Shape::Path {
                edges: Edge::new_rect(
                    Vector2F::splat(30.0),
                    Transform2F::from_translation(Vector2F::splat(-15.0)),
                ),
                stroke_style: StrokeStyle {
                    line_width: 3.0,
                    line_cap: LineCap::Square,
                    line_join: LineJoin::Bevel,
                },
                color: LinSrgba::new(0.0, 0.0, 0.0, 1.0),
            },
        },
        Action::DefineShape {
            id: shape3_id,
            shape: Shape::Group {
                shapes: vec![
                    AugmentedShape {
                        shape: Shape::Fill {
                            edges: Edge::new_rect(
                                Vector2F::splat(30.0),
                                Transform2F::from_translation(Vector2F::splat(-15.0)),
                            ),
                            color: Srgba::<f32>::from_format(Srgba::<u8>::new(149, 125, 173, 255))
                                .into_linear(),
                        },
                        transform: Transform2F::from_scale_rotation_translation(
                            Vector2F::splat(1.0),
                            0.0,
                            Vector2F::new(-100.0, -100.0),
                        ),
                    },
                    AugmentedShape {
                        shape: Shape::Fill {
                            edges: Edge::new_polygon(
                                5,
                                30.0,
                                Transform2F::from_translation(Vector2F::splat(-15.0)),
                            ),
                            color: Srgba::<f32>::from_format(Srgba::<u8>::new(149, 125, 173, 255))
                                .into_linear(),
                        },
                        transform: Transform2F::from_scale_rotation_translation(
                            Vector2F::splat(1.0),
                            0.0,
                            Vector2F::new(100.0, -100.0),
                        ),
                    },
                    AugmentedShape {
                        shape: Shape::Fill {
                            edges: Edge::new_superellipse(
                                Vector2F::splat(30.0),
                                4.0,
                                Transform2F::from_translation(Vector2F::splat(-15.0)),
                            ),
                            color: Srgba::<f32>::from_format(Srgba::<u8>::new(149, 125, 173, 255))
                                .into_linear(),
                        },
                        transform: Transform2F::from_scale_rotation_translation(
                            Vector2F::splat(1.0),
                            0.0,
                            Vector2F::new(100.0, 100.0),
                        ),
                    },
                    AugmentedShape {
                        shape: Shape::Fill {
                            edges: Edge::new_ellipse(
                                Vector2F::splat(15.0),
                                Transform2F::from_translation(Vector2F::splat(-15.0)),
                            ),
                            color: Srgba::<f32>::from_format(Srgba::<u8>::new(149, 125, 173, 255))
                                .into_linear(),
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
                edges: vec![
                    Edge::Move(Vector2F::new(300.0, 100.0)),
                    Edge::Line(Vector2F::new(258.0, 142.0)),
                    Edge::Bezier {
                        control_1: Vector2F::new(322.0, 160.0),
                        control_2: Vector2F::new(326.0, 150.0),
                        to: Vector2F::new(330.0, 142.0),
                    },
                    Edge::Quadratic {
                        control: Vector2F::new(240.0, 100.0),
                        to: Vector2F::new(330.0, 62.0),
                    },
                    Edge::Move(Vector2F::new(330.0, 130.0)),
                    Edge::ArcTo {
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
                color: LinSrgba::new(0.0, 0.0, 0.0, 1.0),
            },
        },
        Action::DefineShape {
            id: shape5_id,
            shape: Shape::MorphPath {
                edges: vec![
                    MorphEdge::Line(Vector2F::new(-15.0, -15.0), Vector2F::new(-18.0, -12.0)),
                    MorphEdge::Line(Vector2F::new(15.0, -15.0), Vector2F::new(0.0, -22.0)),
                    MorphEdge::Line(Vector2F::new(15.0, 15.0), Vector2F::new(30.0, 15.0)),
                    MorphEdge::Line(Vector2F::new(-15.0, 15.0), Vector2F::new(-11.0, 33.0)),
                    MorphEdge::Close,
                ],
                stroke_style: StrokeStyle {
                    line_width: 3.0,
                    line_cap: LineCap::Square,
                    line_join: LineJoin::Bevel,
                },
                color: LinSrgba::new(1.0, 1.0, 1.0, 1.0),
            },
        },
        Action::CreateContainer(ContainerCreationDefintition::new(
            root_id,
            entity_id,
            vec![
                ContainerCreationProperty::Transform(ScaleRotationTranslation::new(
                    Vector2F::splat(0.5),
                    0.0,
                    Vector2F::new(400.0, 400.0),
                )),
                ContainerCreationProperty::Order(2),
            ],
        )),
        Action::CreateContainer(ContainerCreationDefintition::new(
            entity_id,
            shape_id,
            vec![
                ContainerCreationProperty::Transform(ScaleRotationTranslation::new(
                    Vector2F::splat(2.0),
                    0.0,
                    Vector2F::splat(0.0),
                )),
                ContainerCreationProperty::Display(shape_id),
            ],
        )),
        Action::CreateContainer(ContainerCreationDefintition::new(
            entity_id,
            shape2_id,
            vec![
                ContainerCreationProperty::Transform(ScaleRotationTranslation::new(
                    Vector2F::splat(2.0),
                    0.0,
                    Vector2F::splat(0.0),
                )),
                ContainerCreationProperty::Display(shape2_id),
            ],
        )),
        Action::CreateContainer(ContainerCreationDefintition::new(
            root_id,
            shape4_id,
            vec![
                ContainerCreationProperty::Transform(ScaleRotationTranslation::new(
                    Vector2F::splat(2.0),
                    0.0,
                    Vector2F::new(50.0, 100.0),
                )),
                ContainerCreationProperty::Display(shape4_id),
            ],
        )),
        Action::CreateContainer(ContainerCreationDefintition::new(
            entity_id,
            shape5_id,
            vec![
                ContainerCreationProperty::Transform(ScaleRotationTranslation::new(
                    Vector2F::splat(2.0),
                    0.0,
                    Vector2F::new(100.0, 100.0),
                )),
                ContainerCreationProperty::Display(shape5_id),
                ContainerCreationProperty::MorphIndex(0.0),
            ],
        )),
        Action::CreateContainer(ContainerCreationDefintition::new(
            entity_id,
            entity2_id,
            vec![
                ContainerCreationProperty::Transform(ScaleRotationTranslation::default()),
                ContainerCreationProperty::Order(1),
                ContainerCreationProperty::Display(shape3_id),
            ],
        )),
        Action::PresentFrame(0, 1),
        Action::UpdateContainer(ContainerUpdateDefintition::new(
            entity2_id,
            vec![ContainerUpdateProperty::Transform(
                ScaleRotationTranslation::new(Vector2F::splat(1.0), PI, Vector2F::new(200.0, 0.0)),
                Easing::BounceOut,
                480,
            )],
        )),
        Action::UpdateContainer(ContainerUpdateDefintition::new(
            shape5_id,
            vec![ContainerUpdateProperty::MorphIndex(
                1.0,
                Easing::ElasticInOut,
                360,
            )],
        )),
        Action::PresentFrame(1, 240),
        Action::UpdateContainer(ContainerUpdateDefintition::new(
            entity2_id,
            vec![ContainerUpdateProperty::Coloring(
                Coloring::Colorings(vec![
                    Coloring::Color(
                        Srgba::<f32>::from_format(Srgba::<u8>::new(210, 145, 188, 255))
                            .into_linear(),
                    ),
                    Coloring::Color(
                        Srgba::<f32>::from_format(Srgba::<u8>::new(224, 187, 228, 255))
                            .into_linear(),
                    ),
                    Coloring::Color(
                        Srgba::<f32>::from_format(Srgba::<u8>::new(210, 145, 188, 255))
                            .into_linear(),
                    ),
                    Coloring::Color(
                        Srgba::<f32>::from_format(Srgba::<u8>::new(255, 223, 211, 255))
                            .into_linear(),
                    ),
                ]),
                ColorSpace::Linear,
                Easing::ElasticInOut,
                360,
            )],
        )),
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
    let device = GLDevice::new(GLVersion::GL3, 0);
    let render_mode = RendererMode::default_for_device(&device);
    let renderer = Renderer::new(
        device,
        &EmbeddedResourceLoader,
        render_mode,
        RendererOptions {
            background_color: Some(ColorF::white()),
            show_debug_ui: false,
            dest: DestFramebuffer::full_window(window_size),
        },
    );

    let font_context = CanvasFontContext::from_system_source();

    let mut fluster_renderer = FlusterRendererImpl::new(
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
        Duration::from_secs_f64(1.0 / 60.0),
        window_size.to_f32(),
    ) {
        Err(message) => println!("{}", message),
        _ => {}
    }
}

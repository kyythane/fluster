#![deny(clippy::all)]
use crate::rendering::StageRenderer;
use crate::simulation::FrameState;
use iced::{executor, image, Application, Command, Element, Image, Length, Size, Text};
use iced_native::{layout, Hasher, Layout, MouseCursor, Point, Rectangle, Widget};
use iced_wgpu::{Defaults, Primitive, Renderer};
use pathfinder_geometry::vector::Vector2I;
use std::hash::Hash;

pub struct Stage<'a> {
    width: u16,
    height: u16,
    frame: image::Handle,
    frame_state: FrameState<'a>,
}

impl<'a> Stage<'a> {
    pub fn new(width: u16, height: u16, frame: image::Handle, frame_state: FrameState<'a>) -> Self {
        Self {
            width,
            height,
            frame,
            frame_state,
        }
    }
}

impl<'a, Message> Widget<Message, Renderer> for Stage<'a> {
    fn width(&self) -> Length {
        Length::Units(self.width)
    }

    fn height(&self) -> Length {
        Length::Units(self.height)
    }

    fn layout(&self, _renderer: &Renderer, _limits: &layout::Limits) -> layout::Node {
        layout::Node::new(Size::new(f32::from(self.width), f32::from(self.height)))
    }

    fn hash_layout(&self, state: &mut Hasher) {
        self.frame.hash(state);
        self.width.hash(state);
        self.height.hash(state);
    }

    fn draw(
        &self,
        _renderer: &mut Renderer,
        _defaults: &Defaults,
        _layout: Layout<'_>,
        cursor_position: Point,
    ) -> (Primitive, MouseCursor) {
        (
            Primitive::Image {
                handle: self.frame.clone(),
                bounds: Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: f32::from(self.width),
                    height: f32::from(self.height),
                },
            },
            self.frame_state.compute_mouse_state(cursor_position),
        )
    }
}

pub struct App;

#[derive(Debug, Clone)]
pub enum AppMessage {}

impl Application for App {
    type Executor = executor::Default;
    type Message = AppMessage;
    type Flags = ();

    fn new(_flags: ()) -> (App, Command<Self::Message>) {
        (App, Command::none())
    }

    fn title(&self) -> String {
        String::from("An Editor for Fluster Files")
    }

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Text::new("Hello, world!").into()
    }
}

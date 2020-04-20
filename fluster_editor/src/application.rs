#![deny(clippy::all)]
use crate::rendering::StageRenderer;
use crate::simulation::StageState;
use iced::{
    executor, image::Handle as ImageHandle, Align, Application, Column, Command, Container,
    Element, Image, Length, Size, Text,
};
use iced_native::{layout, Hasher, Layout, MouseCursor, Point, Rectangle, Widget};
use iced_wgpu::{Defaults, Primitive, Renderer};
use pathfinder_geometry::vector::Vector2I;
use std::convert::TryInto;
use std::hash::Hash;

pub struct Stage<'a> {
    width: u16,
    height: u16,
    frame: ImageHandle,
    frame_state: &'a StageState,
}

impl<'a> Stage<'a> {
    pub fn new(width: u16, height: u16, frame: ImageHandle, frame_state: &'a StageState) -> Self {
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
            //self.frame_state.compute_mouse_state(cursor_position),
            MouseCursor::Grab,
        )
    }
}

impl<'a, Message> Into<Element<'a, Message>> for Stage<'a> {
    fn into(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

pub struct App {
    stage_renderer: StageRenderer,
    frame_state: StageState,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    FrameUpdate(StageState),
}

pub struct AppFlags {
    stage_size: Vector2I,
}

impl AppFlags {
    pub fn new(stage_size: Vector2I) -> Self {
        Self { stage_size }
    }
}

impl Default for AppFlags {
    fn default() -> Self {
        Self {
            stage_size: Vector2I::new(800, 600),
        }
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = AppMessage;
    type Flags = AppFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let stage_renderer = StageRenderer::new(flags.stage_size).unwrap();
        (
            Self {
                stage_renderer,
                frame_state: StageState::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("An Editor for Fluster Files")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Self::Message::FrameUpdate(frame_state) => {
                self.frame_state = frame_state;
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let stage = Stage::new(
            self.stage_renderer.width().try_into().unwrap(),
            self.stage_renderer.height().try_into().unwrap(),
            ImageHandle::from_pixels(
                self.stage_renderer.width().try_into().unwrap(),
                self.stage_renderer.height().try_into().unwrap(),
                self.stage_renderer.draw_frame(&self.frame_state).unwrap(),
            ),
            &self.frame_state,
        );
        let content = Column::new()
            .padding(20)
            .spacing(20)
            .align_items(Align::Center)
            .push(stage);
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

#![deny(clippy::all)]
use crate::rendering::StageRenderer;
use crate::simulation::{StageState, TimelineState};
use crate::tools::EditState;
use iced::{
    executor, image::Handle as ImageHandle, Align, Application, Column, Command, Container,
    Element, Image, Length, Size, Text,
};
use iced_native::{
    layout, Clipboard, Event, Hasher, Layout, MouseCursor, Point, Rectangle, Widget,
};
use iced_wgpu::{Defaults, Primitive, Renderer};
use pathfinder_geometry::vector::Vector2I;
use std::convert::TryInto;
use std::hash::Hash;

pub struct Stage<'a> {
    width: u16,
    height: u16,
    frame: ImageHandle,
    edit_state: &'a EditState,
}

impl<'a> Stage<'a> {
    pub fn new(width: u16, height: u16, frame: ImageHandle, edit_state: &'a EditState) -> Self {
        Self {
            width,
            height,
            frame,
            edit_state,
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
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> (Primitive, MouseCursor) {
        let cursor = if layout.bounds().contains(cursor_position) {
            self.edit_state.mouse_cursor()
        } else {
            MouseCursor::OutOfBounds
        };
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
            cursor,
        )
    }

    /*
    TODO: implement mouse picking/tool use :)

    /// Processes a runtime [`Event`].
    ///
    /// It receives:
    ///   * an [`Event`] describing user interaction
    ///   * the computed [`Layout`] of the [`Widget`]
    ///   * the current cursor position
    ///   * a mutable `Message` list, allowing the [`Widget`] to produce
    ///   new messages based on user interaction.
    ///   * the `Renderer`
    ///   * a [`Clipboard`], if available
    ///
    /// By default, it does nothing.
    ///
    /// [`Event`]: ../enum.Event.html
    /// [`Widget`]: trait.Widget.html
    /// [`Layout`]: ../layout/struct.Layout.html
    /// [`Clipboard`]: ../trait.Clipboard.html
    */
    fn on_event(
        &mut self,
        event: Event,
        _layout: Layout<'_>,
        cursor_position: Point,
        _messages: &mut Vec<Message>,
        _renderer: &Renderer,
        _clipboard: Option<&dyn Clipboard>,
    ) {
        match event {
            Event::Mouse(mouse_event) => {
                let new_edit_state = self.edit_state.on_mouse_event(mouse_event, cursor_position);
                //TODO: message
            }
            Event::Keyboard(keyboard_event) => {
                //TODO: oh fuck hot keys
            }
            _ => (),
        }
    }
}

impl<'a, Message> Into<Element<'a, Message>> for Stage<'a> {
    fn into(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

pub struct App {
    stage_renderer: StageRenderer,
    stage_state: StageState,
    edit_state: EditState,
    timeline_state: TimelineState,
}

#[derive(Debug, Clone)]
pub enum AppMessage {}

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
                stage_state: StageState::default(),
                edit_state: EditState::default(),
                timeline_state: TimelineState::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("An Editor for Fluster Files")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let stage = Stage::new(
            self.stage_renderer.width().try_into().unwrap(),
            self.stage_renderer.height().try_into().unwrap(),
            ImageHandle::from_pixels(
                self.stage_renderer.width().try_into().unwrap(),
                self.stage_renderer.height().try_into().unwrap(),
                self.stage_renderer
                    .draw_frame(&self.stage_state, &self.timeline_state)
                    .unwrap(),
            ),
            &self.edit_state,
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

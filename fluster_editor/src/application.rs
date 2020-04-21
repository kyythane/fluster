#![deny(clippy::all)]
use crate::rendering::StageRenderer;
use crate::simulation::{StageState, TimelineState};
use crate::tools::{EditMessage, EditState};
use iced::{
    executor, image::Handle as ImageHandle, Align, Application, Column, Command, Container,
    Element, Image, Length, Size, Text,
};
use iced_native::{
    layout, Clipboard, Event, Hasher, Layout, MouseCursor, Point, Rectangle, Widget,
};
use iced_wgpu::{Defaults, Primitive, Renderer};
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::convert::TryInto;
use std::hash::Hash;

pub struct Stage<'a, Message> {
    width: u16,
    height: u16,
    frame: ImageHandle,
    edit_state: &'a EditState,
    stage_state: &'a StageState,
    on_edit: Box<dyn Fn(EditMessage) -> Message>,
}

impl<'a, Message> Stage<'a, Message> {
    pub fn new(
        frame: ImageHandle,
        stage_state: &'a StageState,
        edit_state: &'a EditState,
        on_edit: Box<dyn Fn(EditMessage) -> Message>,
    ) -> Self {
        Self {
            width: stage_state.width().try_into().unwrap(),
            height: stage_state.height().try_into().unwrap(),
            frame,
            stage_state,
            edit_state,
            on_edit,
        }
    }
}

impl<'a, Message> Widget<Message, Renderer> for Stage<'a, Message> {
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
                bounds: layout.bounds(),
            },
            cursor,
        )
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        messages: &mut Vec<Message>,
        _renderer: &Renderer,
        _clipboard: Option<&dyn Clipboard>,
    ) {
        let in_bounds = layout.bounds().contains(cursor_position);
        let stage_position = Vector2F::new(
            (cursor_position.x - layout.bounds().x) * self.stage_state.scale(),
            (cursor_position.y - layout.bounds().y) * self.stage_state.scale(),
        );
        match event {
            Event::Mouse(mouse_event) => {
                println!("{:?} {:?}", mouse_event, stage_position);
                //TODO: mouse picking for existing shapes/entities
                //TODO: how are shapes/entities tracked in edit
                if let Some(edit_message) =
                    self.edit_state
                        .on_mouse_event(mouse_event, stage_position, in_bounds)
                {
                    messages.push((self.on_edit)(edit_message))
                }
            }
            Event::Keyboard(keyboard_event) => {
                //match keyboard_event {}
                //TODO: oh fuck hot keys
                // TODO: modifiers for clicks!!!!!! :(
                // delete, copy, paste, cut, uh...
            }
            _ => (),
        }
    }
}

impl<'a, Message> Into<Element<'a, Message>> for Stage<'a, Message>
where
    Message: 'static,
{
    fn into(self) -> Element<'a, Message> {
        Element::new(self)
    }
}

pub struct App {
    stage_state: StageState,
    stage_renderer: StageRenderer,
    edit_state: EditState,
    timeline_state: TimelineState,
    frame_handle: ImageHandle,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    EditMessage(EditMessage),
}

pub struct AppFlags {
    stage_size: Vector2I,
    background_color: ColorU,
}

impl AppFlags {
    pub fn new(stage_size: Vector2I, background_color: ColorU) -> Self {
        Self {
            stage_size,
            background_color,
        }
    }
}

impl Default for AppFlags {
    fn default() -> Self {
        Self::new(Vector2I::new(800, 600), ColorU::white())
    }
}

impl App {
    fn convert_edit_message(edit_message: EditMessage) -> AppMessage {
        AppMessage::EditMessage(edit_message)
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = AppMessage;
    type Flags = AppFlags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let stage_state = StageState::new(flags.stage_size, flags.background_color);
        let mut stage_renderer = StageRenderer::new(flags.stage_size).unwrap();
        let timeline_state = TimelineState::default();
        let frame_handle = stage_renderer
            .draw_frame(stage_state.compute_render_data(&timeline_state))
            .unwrap();
        (
            Self {
                stage_state,
                stage_renderer,
                edit_state: EditState::default(),
                timeline_state,
                frame_handle,
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
            self.frame_handle.clone(),
            &self.stage_state,
            &self.edit_state,
            Box::new(Self::convert_edit_message),
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

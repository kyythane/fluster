use crate::messages::AppMessage;
use crate::rendering::StageRenderer;
use crate::simulation::{StageState, TimelineState};
use crate::tools::{EditDisplayState, EditState, Tool};

use iced::{
    button::State as ButtonState, executor, image::Handle as ImageHandle, mouse, Align,
    Application, Button, Column, Command, Container, Element, Image, Length, Row, Size,
};
use iced_graphics::{Backend, Defaults, Primitive, Renderer};
use iced_native::{layout, Clipboard, Event, Hasher, Layout, Point, Widget};
use palette::LinSrgb;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::{convert::TryInto, hash::Hash};
pub struct Stage<'a, 'b, 'c> {
    width: u16,
    height: u16,
    frame: ImageHandle,
    edit_state: &'a EditState,
    stage_state: &'a StageState<'b, 'c>,
}

impl<'a, 'b, 'c> Stage<'a, 'b, 'c> {
    pub fn new(
        frame: ImageHandle,
        stage_state: &'a StageState<'b, 'c>,
        edit_state: &'a EditState,
    ) -> Self {
        Self {
            width: stage_state.width().try_into().unwrap(),
            height: stage_state.height().try_into().unwrap(),
            frame,
            stage_state,
            edit_state,
        }
    }
}

impl<'a, 'b, 'c, B> Widget<AppMessage, Renderer<B>> for Stage<'a, 'b, 'c>
where
    B: Backend,
{
    fn width(&self) -> Length {
        Length::Units(self.width)
    }

    fn height(&self) -> Length {
        Length::Units(self.height)
    }

    fn layout(&self, _renderer: &Renderer<B>, _limits: &layout::Limits) -> layout::Node {
        layout::Node::new(Size::new(f32::from(self.width), f32::from(self.height)))
    }

    fn hash_layout(&self, state: &mut Hasher) {
        self.frame.hash(state);
        self.width.hash(state);
        self.height.hash(state);
    }

    fn draw(
        &self,
        _renderer: &mut Renderer<B>,
        _defaults: &Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> (Primitive, mouse::Interaction) {
        let cursor = if layout.bounds().contains(cursor_position) {
            self.edit_state.mouse_cursor()
        } else {
            mouse::Interaction::Idle
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
        messages: &mut Vec<AppMessage>,
        _renderer: &Renderer<B>,
        _clipboard: Option<&dyn Clipboard>,
    ) {
        let in_bounds = layout.bounds().contains(cursor_position);
        let stage_position = Vector2F::new(
            (cursor_position.x - layout.bounds().x) * self.stage_state.scale(),
            //TODO: Potential BUG: Is the inversion of the Y-axis here at all correct or is it making up for another issue elsewhere
            (self.height as f32 - (cursor_position.y - layout.bounds().y))
                * self.stage_state.scale(),
        );
        match event {
            Event::Mouse(mouse_event) => {
                let selection_shape = self.edit_state.selection_shape(stage_position);
                let selection = self.stage_state.query_selection(&selection_shape);
                messages.push(AppMessage::EditHandleMessage(selection.clone()));
                if let Some(edit_message) = self.edit_state.on_mouse_event(
                    mouse_event,
                    selection,
                    stage_position,
                    in_bounds,
                ) {
                    messages.push(AppMessage::EditMessage(edit_message))
                }
            }
            Event::Keyboard(keyboard_event) => {
                //match keyboard_event {}
                // TODO: oh fuck hot keys
                // TODO: modifiers for clicks!!!!!! :(
                // delete, copy, paste, cut, uh...
            }
            _ => (),
        }
    }
}

impl<'a, 'b, 'c> Into<Element<'a, AppMessage>> for Stage<'a, 'b, 'c> {
    fn into(self) -> Element<'a, AppMessage> {
        Element::new(self)
    }
}

#[derive(Default)]
pub struct ToolPaneState {
    pointer_state: ButtonState,
    path_state: ButtonState,
    rect_state: ButtonState,
    polygon_state: ButtonState,
    ellipse_state: ButtonState,
    fill_state: ButtonState,
    eyedropper_state: ButtonState,
}

pub struct AppFlags {
    stage_size: Vector2I,
    background_color: LinSrgb,
}

impl AppFlags {
    pub fn new(stage_size: Vector2I, background_color: LinSrgb) -> Self {
        Self {
            stage_size,
            background_color,
        }
    }
}

impl Default for AppFlags {
    fn default() -> Self {
        Self::new(Vector2I::new(800, 600), LinSrgb::new(1.0, 1.0, 1.0))
    }
}

pub struct App<'a, 'b> {
    stage_state: StageState<'a, 'b>,
    stage_renderer: StageRenderer,
    edit_state: EditState,
    edit_display_state: EditDisplayState,
    timeline_state: TimelineState,
    frame_handle: ImageHandle,
    tool_pane_state: ToolPaneState,
}

impl<'a, 'b> App<'a, 'b> {
    fn refresh_stage(&mut self) {
        let frame_handle = self.stage_renderer.draw_frame(
            self.stage_state.background_color(),
            self.stage_state.engine(),
        );
        self.frame_handle = frame_handle.unwrap();
    }

    fn tool_pane(tool_pane_state: &mut ToolPaneState) -> Column<AppMessage> {
        fn button_factory(button_state: &mut ButtonState, tool: Tool) -> Button<AppMessage> {
            Button::new(button_state, Image::new(tool.image_handle()))
                .on_press(AppMessage::EditMessage(tool.change_message()))
                .width(Length::Fill)
        }

        Column::new()
            .padding(20)
            .spacing(3)
            .push(
                Row::new()
                    .spacing(3)
                    .align_items(Align::Center)
                    .push(button_factory(
                        &mut tool_pane_state.pointer_state,
                        Tool::Pointer,
                    ))
                    .push(button_factory(&mut tool_pane_state.path_state, Tool::Path)),
            )
            .push(
                Row::new()
                    .spacing(3)
                    .align_items(Align::Center)
                    .push(button_factory(&mut tool_pane_state.rect_state, Tool::Rect))
                    .push(button_factory(
                        &mut tool_pane_state.polygon_state,
                        Tool::Polygon,
                    )),
            )
            .push(
                Row::new()
                    .spacing(3)
                    .align_items(Align::Center)
                    .push(button_factory(
                        &mut tool_pane_state.ellipse_state,
                        Tool::Ellipse,
                    ))
                    .push(button_factory(&mut tool_pane_state.fill_state, Tool::Fill)),
            )
            .push(
                Row::new()
                    .spacing(3)
                    .align_items(Align::Center)
                    .push(button_factory(
                        &mut tool_pane_state.eyedropper_state,
                        Tool::Eyedropper,
                    )),
            )
    }
}

impl<'a, 'b> Application for App<'a, 'b> {
    type Executor = executor::Default;
    type Message = AppMessage;
    type Flags = AppFlags;

    // TODO: saving/loading/new
    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let stage_state = StageState::new(flags.stage_size, flags.background_color);
        let mut stage_renderer = StageRenderer::new(flags.stage_size).unwrap();
        let timeline_state = TimelineState::new(stage_state.root());
        let frame_handle = stage_renderer
            .draw_frame(stage_state.background_color(), stage_state.engine())
            .unwrap();
        (
            Self {
                stage_state,
                stage_renderer,
                edit_state: EditState::default(),
                edit_display_state: EditDisplayState::default(),
                timeline_state,
                frame_handle,
                tool_pane_state: ToolPaneState::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("An Editor for Fluster Files")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        let refresh_stage = match message {
            Self::Message::EditMessage(edit_message) => {
                self.edit_state.update(&edit_message);
                self.stage_state.apply_edit(&edit_message)
            }
            Self::Message::EditHandleMessage(handles) => self.stage_state.draw_handles(handles),
            Self::Message::StageUpdateMessage => true,
        };
        if refresh_stage {
            self.refresh_stage();
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let stage = Stage::new(
            self.frame_handle.clone(),
            &self.stage_state,
            &self.edit_state,
        );
        let tools = Self::tool_pane(&mut self.tool_pane_state);
        let options_pane = self.edit_display_state.options_pane(&self.edit_state);
        let content = Row::new()
            .padding(20)
            .spacing(20)
            .align_items(Align::Center)
            .push(stage)
            .push(Column::new().push(tools).push(options_pane));
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

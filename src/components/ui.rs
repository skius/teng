use crate::components::Component;
use crate::rendering::pixel::Pixel;
use crate::rendering::renderer::Renderer;
use crate::{BreakingAction, SharedState, UpdateInfo};
use crossterm::event::{Event, MouseButton, MouseEventKind};
use std::collections::HashMap;

// TODO: it's also problematic that the SharedState contains mouse positions which are in the global frame and not the local one

pub trait UiElement<S = ()> {
    fn is_hover_drag(&self, x: usize, y: usize) -> bool {
        false
    }

    fn is_resizing_drag(&self, x: usize, y: usize) -> bool {
        false
    }

    fn get_size(&self) -> (usize, usize) {
        (0, 0)
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<S>) {}

    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        None
    }

    fn update(&mut self, shared_state: &mut SharedState<S>) {}

    fn render(&self, renderer: &mut dyn Renderer, depth_base: i32);
}

struct Window<S> {
    index: usize,
    // may go out of bounds
    anchor_x: i64,
    anchor_y: i64,
    element: Box<dyn UiElement<S>>,
}

impl<S> Window<S> {
    fn render(&self, renderer: &mut dyn Renderer, depth_base: i32) {
        // Make any render calls offset by the anchor and capped to the size
        let (width, height) = self.element.get_size();

        struct OffsetRenderer<'a> {
            renderer: &'a mut dyn Renderer,
            anchor_x: i64,
            anchor_y: i64,
            width: usize,
            height: usize,
        }

        impl<'a> Renderer for OffsetRenderer<'a> {
            fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
                if x >= self.width || y >= self.height {
                    return;
                }
                let xi = x as i64;
                let yi = y as i64;
                self.renderer.render_pixel(
                    (xi + self.anchor_x) as usize,
                    (yi + self.anchor_y) as usize,
                    pixel,
                    depth,
                );
            }
        }

        let mut offset_renderer = OffsetRenderer {
            renderer,
            anchor_x: self.anchor_x,
            anchor_y: self.anchor_y,
            width,
            height,
        };

        self.element.render(&mut offset_renderer, depth_base);
    }

    fn is_hover_drag(&self, x: usize, y: usize) -> bool {
        let (width, height) = self.element.get_size();
        let max_x = self.anchor_x + width as i64;
        let max_y = self.anchor_y + height as i64;
        if x as i64 >= max_x || y as i64 >= max_y {
            return false;
        }
        self.element.is_hover_drag(
            (x as i64 - self.anchor_x) as usize,
            (y as i64 - self.anchor_y) as usize,
        )
    }

    fn is_resizing_drag(&self, x: usize, y: usize) -> bool {
        let (width, height) = self.element.get_size();
        let max_x = self.anchor_x + width as i64;
        let max_y = self.anchor_y + height as i64;
        if x as i64 >= max_x || y as i64 >= max_y {
            return false;
        }
        self.element.is_resizing_drag(
            (x as i64 - self.anchor_x) as usize,
            (y as i64 - self.anchor_y) as usize,
        )
    }

    fn is_hover(&self, x: usize, y: usize) -> bool {
        let (width, height) = self.element.get_size();
        let max_x = self.anchor_x + width as i64;
        let max_y = self.anchor_y + height as i64;
        let xi = x as i64;
        let yi = y as i64;
        xi >= self.anchor_x && xi < max_x && yi >= self.anchor_y && yi < max_y
    }

    fn move_anchor_by(&mut self, dx: i32, dy: i32) {
        self.anchor_x += dx as i64;
        self.anchor_y += dy as i64;
    }

    fn resize_by(&mut self, dx: i32, dy: i32, shared_state: &mut SharedState<S>) {
        let (width, height) = self.element.get_size();
        let new_width = width as i64 + dx as i64;
        let new_height = height as i64 + dy as i64;
        if new_width < 1 || new_height < 1 {
            return;
        }
        self.element
            .on_resize(new_width as usize, new_height as usize, shared_state);
    }
}

pub struct UiProxy<S> {
    new_elements: Vec<(usize, usize, Box<dyn UiElement<S>>)>,
}

impl<S> UiProxy<S> {
    pub fn new() -> Self {
        Self {
            new_elements: Vec::new(),
        }
    }

    pub fn add_window(&mut self, anchor_x: usize, anchor_y: usize, element: Box<dyn UiElement<S>>) {
        self.new_elements.push((anchor_x, anchor_y, element));
    }
}

struct Dragging {
    // key of the window being dragged
    index: usize,
    // the last position of the mouse, future positions move the window by the difference
    last_x: usize,
    last_y: usize,
}

struct Ui<S> {
    highest_index: usize,
    elements: HashMap<usize, Window<S>>,
    // appearing later in the vector means appearing on top
    render_order: Vec<usize>,
    // only changes on press, not on hold, so that we can drag&drop
    focused: Option<usize>,
    move_dragging: Option<Dragging>,
    resize_dragging: Option<Dragging>,
}

impl<S> Ui<S> {
    fn new() -> Self {
        Self {
            highest_index: 0,
            elements: HashMap::new(),
            render_order: Vec::new(),
            focused: None,
            move_dragging: None,
            resize_dragging: None,
        }
    }

    fn add_window(&mut self, anchor_x: usize, anchor_y: usize, element: Box<dyn UiElement<S>>) {
        self.highest_index += 1;
        let index = self.highest_index;
        self.elements.insert(
            index,
            Window {
                index,
                anchor_x: anchor_x as i64,
                anchor_y: anchor_y as i64,
                element,
            },
        );
        self.render_order.push(index);
    }

    fn get_mut_focused(&mut self) -> Option<&mut Window<S>> {
        self.focused.and_then(|index| self.elements.get_mut(&index))
    }

    fn render_order_move_to_front(&mut self, index: usize) {
        self.render_order.retain(|i| *i != index);
        self.render_order.push(index);
    }

    // TODO: 'rescale' mouse events to use the window's coordinate system
    fn on_event_focused(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        self.get_mut_focused()
            .and_then(|window| window.element.on_event(event, shared_state))
    }

    fn on_event_all(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        for window in self.elements.values_mut() {
            if let Some(action) = window.element.on_event(event.clone(), shared_state) {
                return Some(action);
            }
        }

        None
    }

    // TODO: 'rescale' size to use the window's coordinate system. Do we actually want to run on_resize if the terminal resizes?
    fn on_resize_all(&mut self, width: usize, height: usize, shared_state: &mut SharedState<S>) {
        for window in self.elements.values_mut() {
            window.element.on_resize(width, height, shared_state);
        }
    }

    fn update_all(&mut self, shared_state: &mut SharedState<S>) {
        for window in self.elements.values_mut() {
            window.element.update(shared_state);
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<S>, depth_base: i32) {
        for (order, index) in self.render_order.iter().enumerate() {
            if let Some(window) = self.elements.get(index) {
                // each window gets 10 depth levels.
                // TODO: finally move to f64 depths so we can give a window more depth levels/use more than 10 windows for our assigned 100 depth levels..
                window.render(renderer, depth_base + order as i32 * 10);
            }
        }
    }
}

pub struct UiComponent<S = ()> {
    ui: Ui<S>,
}

impl UiComponent {
    pub fn new() -> Self {
        Self { ui: Ui::new() }
    }
}

impl<S: 'static> Component<S> for UiComponent<S> {
    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        match event {
            ref e @ Event::Mouse(me) => {
                let x = me.column as usize;
                let y = me.row as usize;
                if me.kind == MouseEventKind::Down(MouseButton::Left) {
                    let mut focused = None;
                    // need to traverse in render order
                    for window_idx in self.ui.render_order.iter().rev() {
                        let window = self.ui.elements.get(window_idx).unwrap();
                        if window.is_hover(x, y) {
                            focused = Some(window.index);
                            if window.is_hover_drag(x, y) {
                                self.ui.move_dragging = Some(Dragging {
                                    index: window.index,
                                    last_x: x,
                                    last_y: y,
                                });
                            } else if window.is_resizing_drag(x, y) {
                                self.ui.resize_dragging = Some(Dragging {
                                    index: window.index,
                                    last_x: x,
                                    last_y: y,
                                });
                            }
                            break;
                        }
                    }
                    if let Some(focused_index) = focused {
                        // change render order
                        self.ui.render_order_move_to_front(focused_index);
                    };
                    self.ui.focused = focused;
                }
                if me.kind == MouseEventKind::Up(MouseButton::Left) {
                    self.ui.move_dragging = None;
                    self.ui.resize_dragging = None;
                }
                // move dragged window
                if let Some(dragging) = &mut self.ui.move_dragging {
                    if let Some(window) = self.ui.elements.get_mut(&dragging.index) {
                        let dx = x as i32 - dragging.last_x as i32;
                        let dy = y as i32 - dragging.last_y as i32;
                        window.move_anchor_by(dx, dy);
                        dragging.last_x = x;
                        dragging.last_y = y;
                    }
                }
                // resize dragged window
                if let Some(dragging) = &mut self.ui.resize_dragging {
                    if let Some(window) = self.ui.elements.get_mut(&dragging.index) {
                        let dx = x as i32 - dragging.last_x as i32;
                        let dy = y as i32 - dragging.last_y as i32;
                        window.resize_by(dx, dy, shared_state);
                        dragging.last_x = x;
                        dragging.last_y = y;
                    }
                }
                if let Some(action) = self.ui.on_event_focused(e.clone(), shared_state) {
                    return Some(action);
                }
            }
            e @ Event::Key(_) => {
                if let Some(action) = self.ui.on_event_focused(e, shared_state) {
                    return Some(action);
                }
                // TODO: give all elements a chance to handle key events. some configuration? or separate trait method?
                // if let Some(action) = self.ui.on_event_all(e, shared_state) {
                //     return Some(action);
                // }
            }
            _ => {}
        }

        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        for (x, y, element) in shared_state.ui.new_elements.drain(..) {
            self.ui.add_window(x, y, element);
        }

        self.ui.update_all(shared_state);
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<S>, depth_base: i32) {
        self.ui.render(renderer, shared_state, depth_base);
    }
}

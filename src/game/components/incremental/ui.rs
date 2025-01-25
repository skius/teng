use crate::game::components::incremental::{GamePhase, GameState, PlayerGhost};
use crate::game::{
    BreakingAction, Component, Pixel, Render, Renderer, SetupInfo, SharedState, UpdateInfo,
};
use anymap::any::Any;
use crossterm::event::Event;

#[derive(Clone, Copy)]
enum OffsetX {
    /// This variant defines the button text to be left-aligned and the leftmost x coordinate of the button.
    Left(usize),
    /// This variant defines the button text to be right-aligned and the rightmost x coordinate of the button.
    Right(usize),
}

#[derive(Clone, Copy)]
enum OffsetY {
    /// This variant defines the button text to be top-aligned and the topmost y coordinate of the button.
    Top(usize),
    /// This variant defines the button text to be bottom-aligned and the bottommost y coordinate of the button.
    Bottom(usize),
}

trait UiButton: Any {
    fn help_text(&self) -> &'static str;

    fn bbox(&self) -> (usize, usize, usize, usize) {
        panic!("Need to implement mouse_hover is bbox is not provided")
    }

    fn update_screen_dimensions(&mut self, screen_height: usize, screen_width: usize);

    fn mouse_hover(&self, mouse_x: usize, mouse_y: usize) -> bool {
        let (x, y, width, height) = self.bbox();
        mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
    }

    fn on_click(&mut self, shared_state: &mut SharedState);

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32);
}

macro_rules! new_button {
   (
        $name:ident,
        cost_growth: $cost_growth:literal,
        help_text: $help_text:literal,
        allow_in_moving: $allow_in_moving:literal,
        on_click: |$self:ident, $game_state:ident| $on_click:block,
        render: |$self2:ident, $game_state2:ident| $render:block,
        $( $field:ident: $field_type:ty = $field_default:expr ),*
    ) => {
        new_button!(
            $name,
            cost_growth: $cost_growth,
            cost_start: 1,
            help_text: $help_text,
            allow_in_moving: $allow_in_moving,
            on_click: |$self, $game_state| $on_click,
            render: |$self2, $game_state2| $render,
            $( $field: $field_type = $field_default ),*
        );
    };
    (
        $name:ident,
        cost_growth: $cost_growth:literal,
        cost_start: $cost_start:literal,
        help_text: $help_text:literal,
        allow_in_moving: $allow_in_moving:literal,
        on_click: |$self:ident, $game_state:ident| $on_click:block,
        render: |$self2:ident, $game_state2:ident| $render:block,
        $( $field:ident: $field_type:ty = $field_default:expr ),*
    ) => {
        {
        struct $name {
            offset_x: OffsetX,
            offset_y: OffsetY,
            width: usize,
            height: usize,
            screen_width: usize,
            screen_height: usize,
            button_text: String,
            cost: usize,
            $( $field: $field_type ),*
        }

        impl $name {
            fn new(offset_x: OffsetX, offset_y: OffsetY, screen_height: usize, screen_width: usize) -> Self {
                let text = "Buy".to_string();
                Self {
                    offset_x,
                    offset_y,
                    width: text.len(),
                    height: 1,
                    screen_width,
                    screen_height,
                    button_text: text,
                    cost: $cost_start,
                    $( $field: $field_default ),*
                }
            }

            #[allow(unused)]
            fn change_button_text(&mut self, new_text: &str) {
                self.width = new_text.len();
                self.button_text = new_text.to_string();
            }

            fn screen_pos(&self) -> (usize, usize) {
                let (x, y, _, _) = self.bbox();
                (x, y)
            }
        }

        impl UiButton for $name {
            fn help_text(&self) -> &'static str {
                $help_text
            }

            fn update_screen_dimensions(&mut self, screen_height: usize, screen_width: usize) {
                self.screen_height = screen_height;
                self.screen_width = screen_width;
            }

            fn bbox(&self) -> (usize, usize, usize, usize) {
                let x = match self.offset_x {
                    OffsetX::Left(x) => x,
                    OffsetX::Right(x) => self.screen_width - self.width - x,
                };
                let y = match self.offset_y {
                    OffsetY::Top(y) => y,
                    OffsetY::Bottom(y) => self.screen_height - self.height - y,
                };
                (x, y, self.width, self.height)
            }

            fn on_click(&mut $self, shared_state: &mut SharedState) {
                let $game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
                if $game_state.phase != GamePhase::Building && !$allow_in_moving {
                    return;
                }
                if $game_state.max_blocks >= $self.cost {
                    // TODO: add shared shopmanager
                    $game_state.max_blocks -= $self.cost;
                    $game_state.blocks -= $self.cost;
                    $on_click
                    $self.cost = (($self.cost as f64) * $cost_growth).ceil() as usize;
                }
            }

            fn render(&$self2, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
                let $game_state2 = shared_state.extensions.get::<GameState>().unwrap();
                let is_hover = $self2.mouse_hover(shared_state.mouse_info.last_mouse_pos.0, shared_state.mouse_info.last_mouse_pos.1);
                let lmb_down = shared_state.mouse_info.left_mouse_down;

                let fg_color = [0, 0, 0];
                let enough_blocks = $game_state2.max_blocks >= $self2.cost;
                let deactivated_color = [100, 100, 100];
                let mut bg_color = if $allow_in_moving {
                    match (is_hover, lmb_down) {
                        (true, true) => [200, 200, 255],
                        (true, false) => [255, 255, 255],
                        (false, _) => [200, 200, 200],
                    }
                } else {
                    match (is_hover, lmb_down, $game_state2.phase) {
                        (_, _, phase) if phase != GamePhase::Building => deactivated_color,
                        (true, true, _) => [200, 200, 255],
                        (true, false, _) => [255, 255, 255],
                        (false, _, _) => [200, 200, 200],
                    }
                };
                if !enough_blocks {
                    bg_color = deactivated_color;
                }
                let (x, y) = $self2.screen_pos();
                $self2.button_text.with_color(fg_color).with_bg_color(bg_color).render(
                    &mut renderer,
                    x,
                    y,
                    depth_base,
                );
                let left_text = $render;
                // render to the left
                let len = left_text.len();
                left_text.render(&mut renderer, x - len as usize, y, depth_base);
            }
        }
        |x, y, screen_height, screen_width| Box::new($name::new(x, y, screen_height, screen_width))
        }
    };
}

macro_rules! add_buttons {
    ($buttons:expr, $x:expr, $y:expr, $screen_height:expr, $screen_width:expr, $($button:expr),*$(,)?) => {
        {
                $(
    {
                    $buttons.push(($button)($x, OffsetY::Bottom($y), $screen_height, $screen_width));
                    $y -= 1;
        }
                )*
        }
    };
}

pub struct UiBarComponent {
    buttons: Vec<Box<dyn UiButton>>,
    hover_button: Option<usize>,
}

impl UiBarComponent {
    pub const HEIGHT: usize = 11;
    const BUILDING_PHASE_COLOR: [u8; 3] = [0, 200, 0];
    const MOVING_PHASE_COLOR: [u8; 3] = [200, 0, 0];

    pub fn new() -> Self {
        Self {
            buttons: vec![],
            hover_button: None,
        }
    }
}

impl Component for UiBarComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        let mut y_offset = Self::HEIGHT - 2;
        let x_offset = 1;
        let x_offset = OffsetX::Right(x_offset);
        let screen_height = setup_info.height;
        let screen_width = setup_info.width;
        add_buttons!(
            self.buttons,
            x_offset,
            y_offset,
            screen_height,
            screen_width,
            new_button!(
                PlayerJumpHeightButton,
                cost_growth: 3.0,
                cost_start: 15,
                help_text: "Help: Increase the jump height of the player.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    game_state.upgrades.player_jump_boost_factor += 0.1;
                },
                render: |self, game_state| {
                    format!(
                        "Jump Height ({:.1}) for {} ",
                        game_state.upgrades.player_jump_boost_factor, self.cost
                    )
                },
            ),
            new_button!(
                FallSpeedButton,
                cost_growth: 1.2,
                cost_start: 20,
                help_text: "Help: Increase the fall speed of the player.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    game_state.upgrades.fall_speed_factor += 0.1;
                },
                render: |self, game_state| {
                    format!(
                        "Fall Speed ({:.1}) for {} ",
                        game_state.upgrades.fall_speed_factor, self.cost
                    )
                },
            ),
            new_button!(
                GhostBuyButton,
                cost_growth: 1.4,
                cost_start: 80,
                help_text: "Help: Ghosts give the same amount of blocks on death as the player and 1 block\nif they are alive at the end of the round.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    let new_offset = if let Some(player_ghost) = game_state.player_ghosts.last() {
                        player_ghost.offset_secs + game_state.curr_ghost_delay
                    } else {
                        game_state.curr_ghost_delay
                    };
                    game_state.player_ghosts.push(PlayerGhost::new(new_offset));
                },
                render: |self, game_state| {
                    format!(
                        "Player Ghosts ({}) for {} ",
                        game_state.player_ghosts.len(),
                        self.cost
                    )
                },
            ),
            new_button!(
                GhostCutenessButton,
                cost_growth: 1.1,
                cost_start: 100,
                help_text: "Help: Ghosts give more blocks if they're alive at the end of a round.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    game_state.upgrades.ghost_cuteness += 1;
                },
                render: |self, game_state| {
                    format!(
                        "Ghost Cuteness ({}) for {} ",
                        game_state.upgrades.ghost_cuteness,
                        self.cost
                    )
                },
            ),
            new_button!(
                VelocityExponentButton,
                cost_growth: 2.0,
                cost_start: 120,
                help_text: "Help: Received blocks are additionally multiplied by\nthe death velocity^exponent.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    game_state.upgrades.velocity_exponent += 0.05;
                },
                render: |self, game_state| {
                    format!(
                        "Velocity Exponent ({:.2}) for {} ",
                        game_state.upgrades.velocity_exponent, self.cost
                    )
                },
            ),
            new_button!(
                AutoPlayButton,
                cost_growth: 1.0,
                cost_start: 1000,
                help_text: "Help: Automatically start rounds and make the player jump.",
                allow_in_moving: true,
                on_click: |self, game_state| {
                    if let Some(auto_play) = game_state.upgrades.auto_play {
                        game_state.upgrades.auto_play = Some(!auto_play);
                    } else {
                        game_state.upgrades.auto_play = Some(false);
                        self.change_button_text("Toggle");
                        self.cost = 0;
                    }

                },
                render: |self, game_state| {
                    if self.cost > 0 {
                        format!(
                            "Auto Play for {} ",
                            self.cost
                        )
                    } else {
                        format!(
                            "Auto Play ({}) ",
                            if game_state.upgrades.auto_play.unwrap() { "On" } else { "Off" }
                        )
                    }
                },
            ),
            new_button!(
                BlockHeightButton,
                cost_growth: 1.8,
                cost_start: 400,
                help_text: "Help: Increase the height of blocks by 1.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    game_state.upgrades.block_height += 1;
                },
                render: |self, game_state| {
                    format!(
                        "Block Height ({}) for {} ",
                        game_state.upgrades.block_height, self.cost
                    )
                },
            ),
            new_button!(
                GhostDelayButton,
                cost_growth: 1.8,
                cost_start: 800,
                help_text: "Help: Decrease the delay between player and ghost movement.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    game_state.curr_ghost_delay /= 1.2;
                    let mut curr_offset = game_state.curr_ghost_delay;
                    for ghost in &mut game_state.player_ghosts {
                        ghost.offset_secs = curr_offset;
                        curr_offset += game_state.curr_ghost_delay;
                    }
                },
                render: |self, game_state| {
                    format!(
                        "Ghost Delay ({:.3}) for {} ",
                        game_state.curr_ghost_delay, self.cost
                    )
                },
            ),
            new_button!(
                PlayerWeightButton,
                cost_growth: 2.3,
                cost_start: 5_000,
                help_text: "Help: Increase the weight of the player.",
                allow_in_moving: false,
                on_click: |self, game_state| {
                    game_state.upgrades.player_weight += 1;
                },
                render: |self, game_state| {
                    format!(
                        "Player Weight ({}) for {} ",
                        game_state.upgrades.player_weight, self.cost
                    )
                },
            ),
        );
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        if let Event::Resize(width, height) = event {
            self.buttons.iter_mut().for_each(|button| {
                button.update_screen_dimensions(height as usize, width as usize);
            });
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let last_mouse_info = shared_state.mouse_info;
        // Check if we're hovering a button
        let (x, y) = last_mouse_info.last_mouse_pos;
        let mut hovering = false;
        for (i, button) in self.buttons.iter().enumerate() {
            if button.mouse_hover(x, y) {
                self.hover_button = Some(i);
                hovering = true;
                break;
            }
        }
        if !hovering {
            self.hover_button = None;
        }
        if shared_state.mouse_pressed.left {
            // we pressed a button, if we're hovering
            if let Some(hover_button) = self.hover_button {
                self.buttons[hover_button].on_click(shared_state);
            }
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let blocks = game_state.blocks;
        let max_blocks = game_state.max_blocks;
        let received_blocks = game_state.received_blocks;
        let max_received_blocks = game_state.max_blocks_per_round;
        let phase = &game_state.phase;
        let (phase_str, phase_color) = match phase {
            GamePhase::MoveToBuilding => ("Building", Self::BUILDING_PHASE_COLOR),
            GamePhase::Building => ("Building", Self::BUILDING_PHASE_COLOR),
            GamePhase::BuildingToMoving => ("Moving", Self::MOVING_PHASE_COLOR),
            GamePhase::Moving => ("Moving", Self::MOVING_PHASE_COLOR),
        };

        // Draw outline of UI
        let top_y = shared_state.display_info.height() - Self::HEIGHT;
        let width = shared_state.display_info.width();

        let background_depth = depth_base;
        let content_depth = background_depth + 1;
        let button_depth = content_depth + 1;

        // draw entire background
        for y in top_y..(top_y + Self::HEIGHT) {
            " ".repeat(width)
                .render(&mut renderer, 0, y, background_depth);
        }

        // draw top corners
        renderer.render_pixel(0, top_y, Pixel::new('┌'), content_depth);
        renderer.render_pixel(width - 1, top_y, Pixel::new('┐'), content_depth);
        // draw top line
        "─"
            .repeat(width - 2)
            .chars()
            .enumerate()
            .for_each(|(i, c)| {
                renderer.render_pixel(i + 1, top_y, Pixel::new(c), content_depth);
            });
        let bottom_y = top_y + Self::HEIGHT - 1;
        renderer.render_pixel(0, bottom_y, Pixel::new('└'), content_depth);
        renderer.render_pixel(width - 1, bottom_y, Pixel::new('┘'), content_depth);
        // draw bottom line
        "─"
            .repeat(width - 2)
            .chars()
            .enumerate()
            .for_each(|(i, c)| {
                renderer.render_pixel(i + 1, bottom_y, Pixel::new(c), content_depth);
            });
        // Draw connecting lines
        for y in (top_y + 1)..bottom_y {
            renderer.render_pixel(0, y, Pixel::new('│'), content_depth);
            renderer.render_pixel(width - 1, y, Pixel::new('│'), content_depth);
        }

        let mut x = 1;
        let mut y = top_y + 1;
        let mut s = "Phase: ";
        s.render(&mut renderer, x, y, content_depth);
        x += s.len();
        s = phase_str;
        s.with_color(phase_color)
            .render(&mut renderer, x, y, content_depth);
        x = 1;
        y += 1;
        // Render game runtime
        let runtime = game_state.start_of_game.elapsed().as_secs_f64();
        let runtime_str = format!("Time: {:.1}s", runtime);
        runtime_str.render(&mut renderer, x, y, content_depth);
        y += 1;
        x = 1;
        // render block numbers constant sized
        let max_blocks_str = format!("{}", max_blocks);
        let width = max_blocks_str.len();
        let block_s = if received_blocks > 0 {
            format!(
                "Blocks: {:width$}/{} + {received_blocks}",
                blocks, max_blocks
            )
        } else {
            // let recv_s = if game_state.phase == GamePhase::Building {
            //     format!(" (received: {}, per second: {:.2})", game_state.last_received_blocks, bps)
            // } else {
            //     "".to_string()
            // };
            // format!("Blocks: {:width$}/{} {recv_s}", blocks, max_blocks)
            format!("Blocks: {:width$}/{}", blocks, max_blocks)
        };
        block_s.render(&mut renderer, x, y, content_depth);
        y += 1;
        x = 1;
        // TODO: factor in building time to bps?
        // TODO: keep track of max bps overall?
        let bps = game_state.last_received_blocks as f64 / game_state.last_round_time;
        format!(
            "Last round: {} at {:.2}/s",
            game_state.last_received_blocks, bps
        )
        .render(&mut renderer, x, y, content_depth);
        y += 1;
        x = 1;
        let received_blocks_str = format!("High Score: {}", max_received_blocks);
        received_blocks_str.render(&mut renderer, x, y, content_depth);
        y += 1;
        x = 1;
        let controls_str = match (phase, self.hover_button) {
            (_, Some(hover_button)) => self.buttons[hover_button].help_text(),
            (GamePhase::Building | GamePhase::MoveToBuilding, _) => {
                "Controls: LMB to place blocks, Space to start round\n\
            Goal: Build a map for the character to die from falling from increasing heights"
            }
            (GamePhase::Moving | GamePhase::BuildingToMoving, _) => {
                "Controls: A/D to move, Space to jump\n\
            Goal: Die from falling from increasing heights to earn more blocks"
            }
        };
        controls_str.render(&mut renderer, x, y, content_depth);

        // render buttons
        for button in &self.buttons {
            button.render(&mut renderer, shared_state, button_depth);
        }
    }
}

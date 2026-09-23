use glam::Vec2;
use image::Delay;
use crate::demos::platformer_demo::components::state::PlayerState;
use crate::demos::platformer_demo::player_platform::PlayerPlatform;
use crate::engine::component::{SlotID, Tickable};
use crate::engine::component::timer::Timer;
use crate::engine::component::transform::Transform;
use crate::engine::component::tween::{ease, TimeMode, Tween};
use crate::engine::input::Input;
use crate::engine::world::World;

const SLOT_ACTIONS: [&str; 4] = ["ability_1", "ability_2", "ability_3", "ability_4"];

#[derive(Copy, Clone, PartialEq, Debug, Eq)]
pub enum AbilityId {
    Dash,
    Sprint,
    Dodge,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum InputPhase {
    Pressed(f32),
    Down(f32),
    Released(f32),
}

#[derive(Copy, Clone, PartialEq, Debug, Eq)]
pub enum PlayerAction {
    Jump,
    //etc.
}

/// Represents the operations issued to the player as a reaction to something
#[derive(Copy, Clone, Debug)]
pub enum AbilityReaction {
    Impulse(Vec2),
    Translation(Vec2),
    Rotation(f32),
    OverrideVelocity(Vec2),

    SetAnimation(&'static str),
    ToggleCollider(SlotID, bool),
    //etc.
}

/// Effects that get applied to a player action in response to it happening during an ability
#[derive(Clone, Copy, Debug)]
pub enum ModifierEffect {
    ScaleImpulse(f32),
    AddImpulse(Vec2),
    OverrideVelocity(Vec2),
    //etc.
}
/// The response of the ability to a player action happening while it is active.
pub enum AbilityResponse {
    /// The player action is ignored. Both the action and the ability proceed unaffected.
    PassThrough,
    /// The player action is suppressed entirely
    Suppress,
    /// The ability gets deactivated but pushes modifiers to the player action
    Modify(Vec<ModifierEffect>),
    /// The ability does not deactivate and reacts to the player action
    Reactions(Vec<AbilityReaction>),
}

pub enum TriggerResult {
    /// the ability that was triggered fully resolved on the first tick
    Instant(Vec<AbilityReaction>),
    /// the ability that was triggered lasts multiple ticks
    Sustained(Vec<AbilityReaction>),
    /// the ability that was triggered did not trigger due to input state or player state
    Rejected,
}

pub enum ProgressResult {
    Continue(Vec<AbilityReaction>),
    Finished(Vec<AbilityReaction>),
}

pub trait Ability {
    fn id (&self) -> AbilityId;
    fn is_active(&self) -> bool;

    /// This is called when the correct input action registered an event, regardless of phase.
    fn trigger(
        &mut self,
        phase: InputPhase,
        interrupting: Option<AbilityId>,
        player_state: &mut PlayerState,
        world: &mut World,
    ) -> TriggerResult;
    fn progress(
        &mut self,
        phase: Option<InputPhase>,
        player_state: &mut PlayerState,
        world: &mut World,
        delta_time: f32
    ) -> ProgressResult;

    /// Queried every tick this ability IS the active one, before any other slot is considered for triggering.
    /// Can be implemented to vary tick-to-tick.
    fn is_interruptible(&self) -> bool;
    /// Called on the ability that WAS active, only when a different ability claims activation this
    /// tick. This encodes the type of cleanup that should happen when this ability is interrupted
    /// by another.
    fn on_interrupted(&mut self) -> Vec<AbilityReaction>;
    /// Checks if this Ability interacts with the following player action.
    fn interacts_with(&self, action: PlayerAction) -> bool;
    /// If a player action that this ability can interact with happens, react to it.
    fn react_to(
        &mut self,
        action: PlayerAction,
        player_state: &mut PlayerState,
        world: &mut World
    ) -> AbilityResponse;
}

pub struct Abilities {
    slots: [Option<Box<dyn Ability>>; 4],
    active: Option<usize>,
    pub pending: Vec<AbilityReaction>,
}

impl Abilities {
    pub fn new() -> Abilities {
        Self {
            slots: [None, None, None, None],
            active: None,
            pending: Vec::new(),
        }
    }

    pub fn equip(&mut self, slot: usize, ability: Box<dyn Ability>) {
        if self.active == Some(slot) {
            self.active = None;
        }

        self.slots[slot] = Some(ability);
    }

    pub fn tick(&mut self, input: &Input, player_state: &mut PlayerState, world: &mut World, delta_time: f32) {
        if let Some(idx) = self.active {
            let ability = self.slots[idx].as_mut()
                .expect("Active ability index desynced from ability slots");

            let phase = Self::get_action_phase(SLOT_ACTIONS[idx], input);

            match ability.progress(phase, player_state, world, delta_time) {
                ProgressResult::Continue(reactions) => self.pending.extend(reactions),
                ProgressResult::Finished(reactions) => {
                    self.pending.extend(reactions);
                    self.active = None;
                }
            }
        }

        let interrupt_target = match self.active {
            None => None,
            Some(idx) => {
                let ability = self.slots[idx].as_ref().unwrap();
                if !ability.is_interruptible() {
                    return;
                }
                Some(ability.id())
            }
        };

        for (i, slot) in self.slots.iter_mut().enumerate() {
            if Some(i) == self.active {
                continue;
            }
            let Some(ability) = slot else { continue; };
            let action_name = SLOT_ACTIONS[i];

            let phase = Self::get_action_phase(action_name, input);

            let Some(phase) = phase else { continue; };

            match ability.trigger(phase, interrupt_target, player_state, world) {
                TriggerResult::Rejected => {},
                TriggerResult::Instant(reactions) => {
                    if let Some(prev_idx) = self.active {
                        let prev = self.slots[prev_idx].as_mut().unwrap();
                        self.pending.extend(prev.on_interrupted());
                        self.active = None;
                    }
                    self.pending.extend(reactions);
                    break;
                },
                TriggerResult::Sustained(reactions) => {
                    if let Some(prev_idx) = self.active {
                        let prev = self.slots[prev_idx].as_mut().unwrap();
                        self.pending.extend(prev.on_interrupted());
                    }
                    self.pending.extend(reactions);
                    self.active = Some(i);
                    break;
                }
            }
        }
    }

    pub fn resolve_action(
        &mut self,
        action: PlayerAction,
        player_state: &mut PlayerState,
        world: &mut World,
    ) -> AbilityResponse {
        let Some(idx) = self.active else { return AbilityResponse::PassThrough; };
        let ability = self.slots[idx].as_mut().unwrap();
        if !ability.interacts_with(action) {
            return AbilityResponse::PassThrough
        }

        let response = ability.react_to(action, player_state, world);

        if matches!(response, AbilityResponse::Modify(_)) {
            self.active = None;
        }

        response
    }

    pub fn take_pending(&mut self) -> Vec<AbilityReaction> {
        std::mem::take(&mut self.pending)
    }

    fn get_action_phase(action_name: &str, input: &Input) -> Option<InputPhase> {
        let pressed = input.is_action_pressed(action_name);
        let down = input.is_action_down(action_name);
        let released = input.is_action_released(action_name);

        if pressed.active {
            Some(InputPhase::Pressed(pressed.intensity))
        } else if down.active {
            Some(InputPhase::Down(pressed.intensity))
        } else if released.active {
            Some(InputPhase::Released(pressed.intensity))
        } else {
            None
        }
    }
}

pub struct DashAbility {
    tween: Tween<Vec2>,
    direction: Vec2,
}

impl DashAbility {
    const DASH_DURATION: f32 = 0.4;
    const DASH_DISTANCE: f32 = 3.0;

    pub fn new() -> DashAbility {
        let tween = Tween::new(
            Vec2::ZERO,
            Vec2::ZERO,
            DashAbility::DASH_DURATION,
            ease::linear,
            TimeMode::None,
        );

        Self {
            tween,
            direction: Vec2::ZERO,
        }
    }
}

impl Ability for DashAbility {
    fn id(&self) -> AbilityId {
        AbilityId::Dash
    }

    fn is_active(&self) -> bool {
        !self.tween.is_finished()
    }

    fn trigger(&mut self, phase: InputPhase, interrupting: Option<AbilityId>, player_state: &mut PlayerState, world: &mut World) -> TriggerResult {
        if let InputPhase::Pressed(_) = phase {
            if player_state.direction == Vec2::ZERO {
                return TriggerResult::Rejected;
            }

            let transform: &Transform = world.get_component(player_state.id).unwrap();
            let start_pos = *transform.position();
            let direction = player_state.direction.normalize();
            let end_pos = (direction * Self::DASH_DISTANCE) + start_pos;

            self.tween.retarget(start_pos, end_pos);
            self.direction = direction;

            TriggerResult::Sustained(vec![])
        } else {
            TriggerResult::Rejected
        }
    }

    fn progress(&mut self, phase: Option<InputPhase>, player_state: &mut PlayerState, world: &mut World, delta_time: f32) -> ProgressResult {
        self.tween.advance(delta_time);

        let displacement = self.tween.get_motion();
        let mut reactions = vec![
            AbilityReaction::Translation(displacement),
        ];

        if self.tween.is_finished() {
            reactions.push(AbilityReaction::OverrideVelocity(Vec2::ZERO));
            reactions.push(AbilityReaction::Impulse(self.direction * 3.0));
            ProgressResult::Finished(reactions)
        } else {
            ProgressResult::Continue(reactions)
        }
    }

    fn is_interruptible(&self) -> bool {
        true
    }

    fn on_interrupted(&mut self) -> Vec<AbilityReaction> {
        self.tween.pause();
        Vec::new()
    }

    fn interacts_with(&self, action: PlayerAction) -> bool {
        action == PlayerAction::Jump
    }

    fn react_to(&mut self, action: PlayerAction, player_state: &mut PlayerState, world: &mut World) -> AbilityResponse {
        AbilityResponse::Suppress
    }
}

pub struct DodgeAbility {
    timer: Timer,
    toggled: bool,
}

impl DodgeAbility {
    const IFRAME_DURATION: f32 = 1.0;

    pub fn new() -> DodgeAbility {
        DodgeAbility {
            timer: Timer::new(Self::IFRAME_DURATION),
            toggled: false,
        }
    }
}

impl Ability for DodgeAbility {
    fn id(&self) -> AbilityId {
        AbilityId::Dodge
    }

    fn is_active(&self) -> bool {
        self.timer.is_running()
    }

    fn trigger(&mut self, phase: InputPhase, interrupting: Option<AbilityId>, player_state: &mut PlayerState, world: &mut World) -> TriggerResult {
        if let InputPhase::Pressed(_) = phase {
            self.timer.reset();
            self.timer.start();

            self.toggled = false;

            let mut reactions = vec![
                AbilityReaction::ToggleCollider(PlayerPlatform::C_H_BOX, false),
            ];

            if let Some(AbilityId::Dash) = interrupting {
                reactions.push(AbilityReaction::Impulse(player_state.direction.normalize() * 2.0));
            }

            TriggerResult::Sustained(reactions)
        } else {
            TriggerResult::Rejected
        }
    }

    fn progress(&mut self, phase: Option<InputPhase>, player_state: &mut PlayerState, world: &mut World, delta_time: f32) -> ProgressResult {
        self.timer.tick(delta_time);

        let mut reactions = Vec::new();

        if self.timer.finished() {
            reactions.push(AbilityReaction::ToggleCollider(PlayerPlatform::C_H_BOX, true));

            ProgressResult::Finished(reactions)
        } else {
            ProgressResult::Continue(reactions)
        }
    }

    fn is_interruptible(&self) -> bool {
        false
    }

    fn on_interrupted(&mut self) -> Vec<AbilityReaction> {
        vec![]
    }

    fn interacts_with(&self, action: PlayerAction) -> bool {
        false
    }

    fn react_to(&mut self, action: PlayerAction, player_state: &mut PlayerState, world: &mut World) -> AbilityResponse {
        AbilityResponse::PassThrough
    }
}
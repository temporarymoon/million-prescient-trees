use super::echo_ai::{AgentInput, EchoAgent};
use super::textures::AppTextures;
use crate::cfr::decision_index::DecisionIndex;
use crate::cfr::phase::{PerPhase, PhaseTag};
use crate::cfr::reveal_index::RevealIndex;
use crate::game::battlefield::Battlefield;
use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::{Edict, EdictSet};
use crate::game::known_state_summary::KnownStateEssentials;
use crate::game::status_effect::{StatusEffect, StatusEffectSet};
use crate::game::types::{Player, Score};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::Pair;
use egui::{Grid, Ui, Vec2, Widget};
use egui_extras::RetainedImage;
use std::format;
use std::sync::mpsc::{Receiver, Sender};
use tracing::Level;

// {{{ Agent type
/// The type of payloads sent from the human agent to the gui.
#[derive(Debug, Clone, Copy)]
enum RequestPayload {
    StateAdvanced(AgentInput),
    Reveal(RevealIndex, Score),
    GameFinished,
}

pub struct HumanAgent {
    sender: Sender<RequestPayload>,
    receiver: Receiver<DecisionIndex>,
}
// }}}
// {{{ UI types
#[derive(Debug, Clone, Copy, Default)]
struct PartialMainPhaseChoice {
    creatures: CreatureSet,
    edict: Option<Edict>,
}

#[derive(Debug, Clone, Copy)]
pub enum UITab {
    CardPreview,
    Field,
    Effects,
    History,
    DebugInfo,
}

/// Holds all the state of the gui!
///
/// The reason this is different from `UIState` if because
/// egui_dock likes to have a mut ref to both the tab tree and
/// the main state at the same time afaik.
pub struct GUIApp {
    tab_tree: egui_dock::Tree<UITab>,
    state: UIState,
}

#[derive(Debug, Clone, Copy)]
enum HoveredCard {
    Creature(Creature),
    Edict(Edict),
    Battlefield(Battlefield),
    StatusEffect(StatusEffect),
}

// Holds stuff required for communication with the ui thread.
pub struct UIBus {
    sender: Sender<DecisionIndex>,
    receiver: Receiver<RequestPayload>,
}

/// A value summarizing all the choices a player made during an entire turn.
///
/// Some of the data might not yet be available, so we use options everywhere.
#[derive(Debug, Clone, Copy, Default)]
struct PlayerHistoryEntry {
    creature: Option<Creature>,
    edict: Option<Edict>,
    sabotage: Option<Creature>,
}

/// A value summarizing data about a given turn, be it in the past or the future.
#[derive(Debug, Clone, Copy, Default)]
struct HistoryEntry {
    score: Option<Score>,
    choices: Pair<PlayerHistoryEntry>,
}

/// State used to render the contents of the individual ui tabs.
struct UIState {
    // Received from the agent
    input: AgentInput,
    game_finished: bool,

    // Internal state
    history: [HistoryEntry; 4],
    partial_main_choice: Option<PartialMainPhaseChoice>,
    communication: UIBus,
    decision_sent: bool,

    // Ui state
    textures: AppTextures,
    hovered_card: Option<HoveredCard>,
}
// }}}
// {{{ Agent implementation
impl RequestPayload {
    pub fn get_input(&self) -> Option<AgentInput> {
        match self {
            Self::StateAdvanced(input) => Some(*input),
            _ => None,
        }
    }
}

impl UIBus {
    fn new(sender: Sender<DecisionIndex>, receiver: Receiver<RequestPayload>) -> Self {
        Self { sender, receiver }
    }
}

impl HumanAgent {
    pub fn create() -> (Self, UIBus) {
        let decisions = std::sync::mpsc::channel();
        let input = std::sync::mpsc::channel();

        let ui_bus = UIBus::new(decisions.0, input.1);
        let res = Self {
            sender: input.0,
            receiver: decisions.1,
        };

        (res, ui_bus)
    }
}

impl EchoAgent for HumanAgent {
    fn choose(&mut self, agent_input: AgentInput) -> DecisionIndex {
        let _guard = tracing::span!(Level::DEBUG, "human agent choose method");
        tracing::trace!("Sending input");

        self.sender
            .send(RequestPayload::StateAdvanced(agent_input))
            .unwrap();

        tracing::trace!("Input sent");
        let decision = self.receiver.recv().unwrap();
        tracing::trace!("Received decision");

        decision
    }

    fn game_finished(&mut self) {
        let _guard = tracing::span!(Level::DEBUG, "human agent game finished method");
        tracing::trace!("Game finished");

        self.sender.send(RequestPayload::GameFinished).unwrap();
    }

    fn reveal_info(&mut self, reveal_index: RevealIndex, updated_score: Score) {
        let _guard = tracing::span!(Level::DEBUG, "human agent reveal info method");
        tracing::trace!("Received revealed info, with score={updated_score:?}.");

        self.sender
            .send(RequestPayload::Reveal(reveal_index, updated_score))
            .unwrap();
    }
}
// }}}
// {{{ UI implementation
impl UIState {
    const CARD_SIZE: [f32; 2] = [50.0, 50.0];

    // {{{ Data helpers
    fn my_creatures(&self) -> Option<CreatureSet> {
        self.input
            .hidden
            .get_sabotage()
            .or_else(|| self.partial_main_choice.map(|p| p.creatures))
    }

    fn played_edicts(&self) -> Pair<Option<Edict>> {
        let choices = match self.input.phase {
            PerPhase::Main(_) => None,
            PerPhase::Sabotage(p) => Some(p.edict_choices),
            PerPhase::Seer(p) => Some(p.edict_choices),
        };

        choices.map_or_else(
            || {
                let mut result = [None; 2];

                if let Some(partial) = self.partial_main_choice {
                    result[0] = partial.edict;
                };

                result
            },
            |x| x.map(Some),
        )
    }

    fn sabotage_choices(&self) -> Pair<Option<Creature>> {
        match self.input.phase {
            PerPhase::Seer(seer) => seer.sabotage_choices,
            _ => [None; 2],
        }
    }
    // }}}
    // {{{ Drawing helpers
    #[inline(always)]
    fn draw_gray_image(ui: &mut Ui, image: &RetainedImage) -> egui::Response {
        let texture = image.texture_id(ui.ctx());
        egui::Image::new(texture, image.size_vec2())
            .tint(egui::Color32::DARK_GRAY)
            .ui(ui)
    }

    /// Renders a retained image as an image inside a button.
    #[inline(always)]
    fn draw_clickable_image_size(
        ui: &mut Ui,
        image: &RetainedImage,
        size: impl Into<Vec2>,
    ) -> egui::Response {
        let texture = image.texture_id(ui.ctx());
        egui::ImageButton::new(texture, size).ui(ui)
    }

    #[inline(always)]
    fn draw_clickable_image(ui: &mut Ui, image: &RetainedImage) -> egui::Response {
        Self::draw_clickable_image_size(ui, image, image.size_vec2())
    }

    #[inline(always)]
    fn draw_battlefield(&mut self, ui: &mut Ui, battlefield: Battlefield, disabled: bool) {
        let retained_image = &self.textures.battlefields[battlefield as usize];
        let res = if disabled {
            Self::draw_gray_image(ui, retained_image)
        } else {
            retained_image.show(ui)
        };

        if res.hovered() {
            self.hovered_card = Some(HoveredCard::Battlefield(battlefield));
        }
    }

    #[inline(always)]
    fn draw_status_effect(&mut self, ui: &mut Ui, status_effect: StatusEffect) {
        let res = ui.label(format!("{status_effect:?}"));
        if res.hovered() {
            self.hovered_card = Some(HoveredCard::StatusEffect(status_effect));
        }
    }

    #[inline(always)]
    fn draw_status_effect_set(&mut self, ui: &mut Ui, status_effects: StatusEffectSet) {
        for status_effect in status_effects {
            Self::draw_status_effect(self, ui, status_effect);
        }
    }

    #[inline(always)]
    fn draw_edict(&mut self, ui: &mut Ui, edict: Edict, clickable: bool) -> egui::Response {
        let tex = &self.textures.edicts[edict as usize];
        let res = if clickable {
            Self::draw_clickable_image(ui, tex)
        } else {
            tex.show(ui)
        };

        if res.hovered() {
            self.hovered_card = Some(HoveredCard::Edict(edict));
        }

        res
    }

    #[inline(always)]
    fn draw_opt_edict(&mut self, ui: &mut Ui, edict: Option<Edict>) {
        if let Some(edict) = edict {
            Self::draw_edict(self, ui, edict, false);
        } else {
            self.textures.card_back.show(ui);
        };
    }

    #[inline(always)]
    fn draw_creature(
        &mut self,
        ui: &mut Ui,
        creature: Creature,
        clickable: bool,
    ) -> egui::Response {
        let tex = &self.textures.creatures[creature as usize];
        let res = if clickable {
            Self::draw_clickable_image(ui, tex)
        } else {
            tex.show(ui)
        };

        if res.hovered() {
            self.hovered_card = Some(HoveredCard::Creature(creature));
        }

        res
    }

    #[inline(always)]
    fn draw_opt_creature(&mut self, ui: &mut Ui, creature: Option<Creature>) {
        if let Some(creature) = creature {
            self.draw_creature(ui, creature, false);
        } else {
            self.textures.card_back.show(ui);
        }
    }

    #[inline(always)]
    fn draw_edict_set(&mut self, ui: &mut Ui, edicts: EdictSet) {
        for edict in edicts {
            Self::draw_edict(self, ui, edict, false);
        }
    }
    // }}}
    // {{{ Agent communication
    /// Communicates a choice, and marks the decision as sent.
    #[inline(always)]
    fn send(&mut self, index: DecisionIndex) {
        self.communication.sender.send(index).unwrap();
        self.decision_sent = true;
    }

    /// Attempts to send the main phase choice the user has made.
    ///
    /// Communicates the choice the playe rmade for the sabotage phase.
    ///
    /// Acts as a noop if the phase isn't correct, or if the user
    /// hasn't finished choosing the input just yet.
    fn try_communicate_main(&mut self) {
        if self.decision_sent || self.input.phase.tag() != PhaseTag::Main {
            return;
        }

        match self.partial_main_choice {
            Some(PartialMainPhaseChoice {
                creatures,
                edict: Some(edict),
            }) if creatures.len() == self.input.state.creature_choice_size(self.input.player) => {
                tracing::event!(Level::INFO, "Sending main phase decision to agent");

                let index = DecisionIndex::encode_main_phase_index(
                    &self.input.state,
                    self.input.player,
                    self.input.hidden.get_main(),
                    creatures,
                    edict,
                )
                .unwrap();

                self.send(index);
            }
            _ => {}
        }
    }

    /// Communicates the choice the playe rmade for the seer phase.
    ///
    /// Acts as a noop if the phase isn't correct, if the user
    /// hasn't finished choosing the input just yet, or if the choice
    /// has already been sent.
    fn communicate_sabotage(&mut self, guess: Creature) {
        if self.decision_sent || self.input.phase.tag() != PhaseTag::Sabotage {
            return;
        }

        tracing::event!(Level::INFO, "Sending sabotage phase decision to agent");

        let index = DecisionIndex::encode_sabotage_index(
            &self.input.state,
            self.input.hidden.get_main(),
            Some(guess),
        );

        self.send(index);
    }

    /// Communicates the choice the playe rmade for the seer phase.
    ///
    /// Acts as a noop if the phase isn't correct, if the user
    /// hasn't finished choosing the input just yet, or if the choice
    /// has already been sent.
    fn communicate_seer(&mut self, final_choice: Creature) {
        if self.decision_sent || self.input.phase.tag() != PhaseTag::Seer {
            return;
        }

        tracing::event!(Level::INFO, "Sending seer phase decision to agent");

        let all_choices = self.input.hidden.get_sabotage().unwrap();
        let index = DecisionIndex::encode_seer_index(all_choices, final_choice).unwrap();

        self.send(index);
    }

    /// Attempts to read data coming from the bus, and updates the internal state accordingly.
    fn try_accept_input(&mut self) {
        match self.communication.receiver.try_recv() {
            // {{{ State advanced
            Ok(RequestPayload::StateAdvanced(input)) => {
                tracing::event!(Level::INFO, "Received unfinished input from agent");

                self.input = input;
                self.partial_main_choice = if input.phase.tag() == PhaseTag::Main {
                    Some(PartialMainPhaseChoice::default())
                } else {
                    None
                };

                // {{{ Take single choice decisions
                // If we have a single valid decision we can take, we take it right away.
                if self
                    .input
                    .player
                    .select(self.input.phase.decision_counts(&self.input.state))
                    == 1
                {
                    tracing::event!(Level::INFO, "Sending single choice decision to agent");
                    // Send the only possible decision right away!
                    self.send(DecisionIndex::default());
                } else {
                    self.decision_sent = false;
                }
                // }}}
            }
            // }}}
            // {{{ Reveal
            Ok(RequestPayload::Reveal(reveal_index, updated_score)) => {
                let _guard = tracing::span!(Level::TRACE, "Updating history");
                tracing::event!(Level::TRACE, "Updating history");

                let entry = &mut self.history[self.input.state.battlefields.current];

                match self
                    .input
                    .phase
                    .advance_phase(&self.input.state, reveal_index)
                    .unwrap()
                {
                    PerPhase::Sabotage(sabotage) => {
                        for player in Player::PLAYERS {
                            let player_entry = player.select_mut(&mut entry.choices);
                            player_entry.edict = Some(player.select(sabotage.edict_choices));
                        }
                    }
                    PerPhase::Seer(seer) => {
                        for player in Player::PLAYERS {
                            let player_entry = player.select_mut(&mut entry.choices);
                            player_entry.sabotage = player.select(seer.sabotage_choices);

                            if player != self.input.state.last_creature_revealer() {
                                player_entry.creature = Some(seer.revealed_creature);
                            }
                        }
                    }
                    PerPhase::Main(_) => {
                        let first_revealer = !self.input.state.last_creature_revealer();
                        let revealed_creature =
                            first_revealer.select(entry.choices).creature.unwrap();

                        let decoded = reveal_index
                            .decode_seer_phase_reveal(self.input.state.graveyard, revealed_creature)
                            .unwrap();

                        let player_entry = self
                            .input
                            .state
                            .last_creature_revealer()
                            .select_mut(&mut entry.choices);

                        player_entry.creature = Some(decoded);
                        entry.score = Some(updated_score);
                    }
                };

                tracing::event!(Level::TRACE, "Succesfully updated history");
            }
            // }}}
            // {{{ Game finished
            Ok(RequestPayload::GameFinished) => {
                self.game_finished = true;
            }
            // }}}
            _ => {}
        }
    }
    // }}}
}

impl egui_dock::TabViewer for UIState {
    // {{{ Main drawing procedure
    type Tab = UITab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("{tab:?}").into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let span = tracing::span!(Level::INFO, "Rendering ui");
        let _guard = span.enter();

        let [me, you] = self.input.player.order_as(self.input.state.player_states);
        match tab {
            // {{{ Field state
            UITab::Field => {
                if self.game_finished {
                    let result = self
                        .history
                        .last()
                        .unwrap()
                        .score
                        .unwrap()
                        .from_perspective(self.input.player)
                        .to_battle_result();

                    ui.heading(format!("Game ended! Game result: {:?}", result));
                    return;
                }

                ui.vertical(|ui| {
                    // {{{ Prepare data
                    let opponent_creature_possibilities =
                        !(self.input.hidden.get_main() | self.input.state.graveyard);

                    let [my_edict, your_edict] = self.played_edicts();
                    let [my_sabotage, your_sabotage] = self.sabotage_choices();

                    let is_main = self.input.phase.tag() == PhaseTag::Main;

                    let show_my_sabotage =
                        !is_main && self.input.phase.sabotage_status(self.input.player);
                    let show_your_sabotage =
                        !is_main && self.input.phase.sabotage_status(!self.input.player);

                    // The following three vars are true when the player is expected to make a
                    // choice for the respective phase.
                    let can_make_main_choice = is_main && !self.decision_sent;
                    let can_make_sabotage_choice = self.input.phase.tag() == PhaseTag::Sabotage
                        && !self.decision_sent
                        && show_my_sabotage;
                    let can_make_seer_choice = self.input.phase.tag() == PhaseTag::Seer
                        && !self.decision_sent
                        && self.input.state.seer_status(self.input.player);
                    // }}}
                    // {{{ Opponent's board
                    ui.heading("Opponent's board");

                    // {{{ Edicts
                    ui.horizontal(|ui| {
                        self.draw_edict_set(ui, you.edicts);
                    });
                    // }}}
                    // {{{ Creatures
                    ui.horizontal(|ui| {
                        for creature in opponent_creature_possibilities {
                            let res = self.draw_creature(ui, creature, can_make_sabotage_choice);

                            if can_make_sabotage_choice && res.clicked() {
                                self.communicate_sabotage(creature);
                            }
                        }
                    });
                    // }}}
                    // {{{ Choices
                    Grid::new("Opponent's choices").show(ui, |ui| {
                        // {{{ Labels
                        ui.label("Edict");

                        if show_your_sabotage {
                            ui.label("Sabotage");
                        }

                        ui.label("Creature");

                        ui.end_row();
                        // }}}

                        self.draw_opt_edict(ui, your_edict);

                        if show_your_sabotage {
                            self.draw_opt_creature(ui, your_sabotage);
                        }

                        let choices = self.history[self.input.state.battlefields.current].choices;
                        self.draw_opt_creature(ui, (!self.input.player).select(choices).creature);

                        ui.end_row();
                    });
                    // }}}
                    // }}}

                    ui.separator();
                    ui.label(format!(
                        "Your score - opponent's score = {:?}",
                        self.input.state.score.from_perspective(self.input.player)
                    ));
                    ui.separator();

                    // {{{ Player's board
                    ui.heading("Your board");

                    // {{{ Choices
                    Grid::new("Player's choices").show(ui, |ui| {
                        // {{{ Labels
                        ui.label("Edict");

                        if show_my_sabotage {
                            ui.label("Sabotage");
                        }

                        ui.label("Creatures");
                        ui.end_row();
                        // }}}

                        self.draw_opt_edict(ui, my_edict);

                        if show_my_sabotage {
                            self.draw_opt_creature(ui, my_sabotage);
                        }

                        // {{{ Creature choices
                        let creature_choices = self.my_creatures().unwrap_or_default();
                        for creature in creature_choices {
                            let res = self.draw_creature(ui, creature, can_make_seer_choice);

                            if can_make_seer_choice && res.clicked() {
                                self.communicate_seer(creature)
                            }
                        }

                        let max_creature_choice_count =
                            self.input.state.creature_choice_size(self.input.player);

                        for _ in creature_choices.len()..max_creature_choice_count {
                            self.textures.card_back.show(ui);
                        }
                        // }}}

                        ui.end_row();
                    });
                    // }}}
                    // {{{ Edicts
                    ui.horizontal(|ui| {
                        for edict in me.edicts {
                            let res = self.draw_edict(ui, edict, can_make_main_choice);

                            if can_make_main_choice && res.clicked() {
                                if let Some(choice) = &mut self.partial_main_choice {
                                    choice.edict = Some(edict);
                                }
                            }
                        }
                    });
                    // }}}
                    // {{{ Creatures
                    ui.horizontal(|ui| {
                        for creature in self.input.hidden.get_main() {
                            let res = self.draw_creature(ui, creature, can_make_main_choice);

                            if can_make_main_choice && res.clicked() {
                                if let Some(choice) = &mut self.partial_main_choice {
                                    if choice.creatures.has(creature) {
                                        choice.creatures.remove(creature);
                                    } else if choice.creatures.len()
                                        == self.input.state.creature_choice_size(self.input.player)
                                    {
                                        choice.creatures.remove(choice.creatures.index(0).unwrap());
                                        choice.creatures.insert(creature);
                                    } else {
                                        choice.creatures.insert(creature);
                                    }
                                }
                            }
                        }
                    });
                    // }}}
                    // }}}
                    // {{{ Communicate
                    if self.input.phase.tag() == PhaseTag::Main {
                        self.try_communicate_main();
                    }
                    // }}}
                });
            }
            // }}}
            // {{{ Effects
            UITab::Effects => {
                ui.vertical(|ui| {
                    ui.heading("Your effects");
                    self.draw_status_effect_set(ui, me.effects);
                });

                ui.vertical(|ui| {
                    ui.heading("Opponent's effects");
                    self.draw_status_effect_set(ui, you.effects);
                });
            }
            // }}}
            // {{{ History
            UITab::History => {
                ui.vertical(|ui| {
                    // {{{ Battlefields
                    Grid::new("battlefield & history grid").show(ui, |ui| {
                        ui.label("Battlefields");
                        ui.label("Your creature");
                        ui.label("Your edict");
                        ui.label("Your sabotage");
                        ui.label("Opponent's sabotage");
                        ui.label("Opponent's edict");
                        ui.label("Opponent's creature");
                        ui.end_row();

                        for index in 0..4 {
                            let in_the_past =
                                self.game_finished || index < self.input.state.battlefields.current;

                            self.draw_battlefield(
                                ui,
                                self.input.state.battlefields.all[index],
                                false,
                            );

                            let entry = self.history[index];

                            if in_the_past {
                                let [me, you] = self.input.player.order_as(entry.choices);
                                self.draw_opt_creature(ui, me.creature);
                                self.draw_opt_edict(ui, me.edict);
                                self.draw_opt_creature(ui, me.sabotage);
                                self.draw_opt_creature(ui, you.sabotage);
                                self.draw_opt_edict(ui, you.edict);
                                self.draw_opt_creature(ui, you.creature);
                            } else {
                                for _ in 0..6 {
                                    Self::draw_gray_image(ui, &self.textures.card_back);
                                }
                            }

                            ui.end_row();
                        }
                    });
                    // }}}
                    // {{{ Score plot
                    ui.group(|ui| {
                        ui.heading("Your score - Opponent's score");

                        let plot = egui::plot::Plot::new("score plot")
                            .allow_scroll(false)
                            .allow_drag(false);

                        plot.show(ui, |plot_ui| {
                            let scores = self.history.iter().filter_map(|e| e.score);
                            let min_score = scores.clone().min().unwrap_or(Score(-5));
                            let max_score = scores.clone().max().unwrap_or(Score(5));

                            let padding_x = 0.3;
                            let padding_y = 3.0;

                            plot_ui.set_plot_bounds(egui::plot::PlotBounds::from_min_max(
                                [-padding_x, min_score.0 as f64 - padding_y],
                                [4.0 + padding_x, max_score.0 as f64 + padding_y],
                            ));

                            let points: Vec<_> = self
                                .history
                                .iter()
                                .enumerate()
                                .filter_map(|(i, entry)| entry.score.map(|score| (i, score)))
                                .map(|(i, score)| {
                                    let mut bar = egui::plot::Bar::new(
                                        i as f64 + 0.5,
                                        score.from_perspective(self.input.player).0 as f64,
                                    );

                                    bar.bar_width = 1.0;

                                    bar
                                })
                                .collect();

                            plot_ui.bar_chart(egui::plot::BarChart::new(points));
                        })
                    });
                    // }}}
                });
            }
            // }}}
            // {{{ Card preview
            UITab::CardPreview => {
                if let Some(hovered) = self.hovered_card {
                    // {{{ Card name
                    let name = match hovered {
                        HoveredCard::Creature(inner) => format!("{inner:?}"),
                        HoveredCard::Edict(inner) => format!("{inner:?}"),
                        HoveredCard::Battlefield(inner) => format!("{inner:?}"),
                        HoveredCard::StatusEffect(inner) => format!("{inner:?}"),
                    };

                    ui.heading(name);
                    // }}}
                    // {{{ Image
                    let max_width = ui.available_width();
                    match hovered {
                        HoveredCard::Creature(creature) => {
                            self.textures.creatures[creature as usize]
                                .show_size(ui, Vec2::new(max_width, max_width));
                        }
                        HoveredCard::Edict(edict) => {
                            self.textures.edicts[edict as usize]
                                .show_size(ui, Vec2::new(max_width, max_width));
                        }
                        HoveredCard::Battlefield(battlefield) => {
                            self.textures.battlefields[battlefield as usize]
                                .show_size(ui, Vec2::new(max_width, max_width));
                        }
                        HoveredCard::StatusEffect(status_effect) => {
                            self.draw_status_effect(ui, status_effect)
                        }
                    }
                    // }}}
                    // {{{ Description
                    let description = match hovered {
                        HoveredCard::Creature(inner) => Creature::DESCRIPTIONS[inner as usize],
                        HoveredCard::Edict(inner) => Edict::DESCRIPTIONS[inner as usize],
                        _ => "unwritten",
                    };

                    ui.label(description);
                    // }}}
                    ui.separator();
                    // {{{ Extra info
                    match hovered {
                        // {{{ Creature
                        HoveredCard::Creature(creature) => {
                            ui.label(format!(
                                "Strength = {} + bonuses from:",
                                creature.strength()
                            ));

                            let bonus_image_size = ui.available_width() / 4.0;

                            ui.horizontal(|ui| {
                                for battlefield in Battlefield::BATTLEFIELDS {
                                    if battlefield.bonus(creature) {
                                        let is_current =
                                            self.input.state.battlefields.current() == battlefield;
                                        let size_multiplier = if is_current { 1.5 } else { 1.0 };

                                        let res = Self::draw_clickable_image_size(
                                            ui,
                                            &self.textures.battlefields[battlefield as usize],
                                            [bonus_image_size * size_multiplier; 2],
                                        );

                                        if res.clicked() {
                                            self.hovered_card =
                                                Some(HoveredCard::Battlefield(battlefield));
                                        }
                                    };
                                }
                            });
                        }
                        // }}}
                        // {{{ Battlefield
                        HoveredCard::Battlefield(battlefield) => {
                            let mut creatures = CreatureSet::empty();

                            for creature in Creature::CREATURES {
                                if battlefield.bonus(creature) {
                                    creatures.insert(creature);
                                }
                            }

                            if creatures != CreatureSet::empty() {
                                let bonus_image_size = ui.available_width() / 5.0;

                                ui.label("Battlefield bonuses");
                                ui.horizontal(|ui| {
                                    for creature in creatures {
                                        let res = Self::draw_clickable_image_size(
                                            ui,
                                            &self.textures.creatures[creature as usize],
                                            [bonus_image_size; 2],
                                        );

                                        if res.clicked() {
                                            self.hovered_card =
                                                Some(HoveredCard::Creature(creature));
                                        }
                                    }
                                });
                                ui.separator();
                            }

                            ui.label(format!("Reward: {}", battlefield.reward()));
                        }
                        // }}}
                        _ => {}
                    };
                    // }}}
                }
            } // }}}
            // {{{ Debug info
            UITab::DebugInfo => {
                ui.heading("Debug info");
                Grid::new("debug info").show(ui, |ui| {
                    ui.label("Decision sent");
                    ui.label(format!("{}", self.decision_sent));
                    ui.end_row();
                    ui.label("Phase");
                    ui.label(format!("{:?}", self.input.phase.tag()));
                    ui.end_row();
                    ui.label("Hovered");
                    ui.label(format!("{:?}", self.hovered_card));
                });
            } // }}}
        }
    }
    // }}}
}
// }}}
// {{{ GUIApp stuff
impl GUIApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>, communication: UIBus) -> Self {
        let ui_state = UIState {
            input: communication.receiver.recv().unwrap().get_input().unwrap(),
            history: [HistoryEntry::default(); 4],
            partial_main_choice: Some(PartialMainPhaseChoice::default()),
            decision_sent: false,
            textures: AppTextures::new(),
            hovered_card: None,
            game_finished: false,
            communication,
        };

        // {{{ Tabs
        let mut tab_tree = egui_dock::Tree::new(vec![UITab::Field, UITab::Effects, UITab::History]);
        tab_tree.split_left(
            egui_dock::tree::node_index::NodeIndex::root(),
            0.33,
            vec![UITab::CardPreview, UITab::DebugInfo],
        );
        // }}}

        Self {
            tab_tree,
            state: ui_state,
        }
    }

    /// Main rendering function
    fn ui(&mut self, ui: &mut Ui) {
        self.state.try_accept_input();

        egui_dock::DockArea::new(&mut self.tab_tree)
            .style(egui_dock::Style::from_egui(ui.style().as_ref()))
            .show_inside(ui, &mut self.state);
    }
}

// {{{ Eframe implementation
impl eframe::App for GUIApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::new([false; 2]).show(ui, |ui| self.ui(ui));
        });
    }
}
// }}}
// }}}

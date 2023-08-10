use super::echo_ai::{AgentInput, EchoAgent};
use super::textures::AppTextures;
use crate::cfr::decision_index::DecisionIndex;
use crate::cfr::phase::{PerPhase, PhaseTag};
use crate::game::battlefield::Battlefield;
use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::{Edict, EdictSet};
use crate::game::known_state_summary::KnownStateEssentials;
use crate::game::status_effect::{StatusEffect, StatusEffectSet};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::Pair;
use egui::{Grid, Ui, Vec2, Widget};
use egui_extras::RetainedImage;
use std::format;
use std::sync::mpsc::{Receiver, Sender};
use tracing::Level;

// {{{ General types
type Decision = Option<DecisionIndex>;
// }}}
// {{{ Agent type
pub struct HumanAgent {
    sender: Sender<AgentInput>,
    receiver: Receiver<Decision>,
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
    sender: Sender<Decision>,
    receiver: Receiver<AgentInput>,
}

/// State used to render the contents of the individual ui tabs.
struct UIState {
    // Received from the agent
    input: AgentInput,

    // Internal state
    history: [Option<Pair<(Creature, Edict, Option<Creature>)>>; 4],
    partial_main_choice: Option<PartialMainPhaseChoice>,
    communication: UIBus,
    decision_sent: bool,

    // Ui state
    textures: AppTextures,
    hovered_card: Option<HoveredCard>,
}
// }}}
// {{{ Agent implementation
impl UIBus {
    pub fn new(sender: Sender<Decision>, receiver: Receiver<AgentInput>) -> Self {
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
    fn choose(&mut self, agent_input: AgentInput) -> Option<DecisionIndex> {
        let _guard = tracing::span!(Level::DEBUG, "human agent choose method");
        tracing::trace!("Sending input");
        self.sender.send(agent_input).unwrap();
        tracing::trace!("Input sent");
        let decision = self.receiver.recv().unwrap();
        tracing::trace!("Received decision");

        decision
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

    /// Similar to `draw_creature_set`, but will show additional
    /// card backs to get the set to a given size.
    #[inline(always)]
    fn draw_creature_set_of_length(&mut self, ui: &mut Ui, creatures: CreatureSet, len: usize) {
        let remaining = len - creatures.len();

        for creature in creatures {
            self.draw_creature(ui, creature, false);
        }

        for _ in 0..remaining {
            self.draw_opt_creature(ui, None);
        }
    }

    #[inline(always)]
    fn draw_creature_set(&mut self, ui: &mut Ui, creatures: CreatureSet) {
        for creature in creatures {
            self.draw_creature(ui, creature, false);
        }
    }
    // }}}
    // {{{ Communication
    fn try_communicate_main(&mut self) {
        if self.decision_sent {
            return;
        }

        match self.input.phase.tag() {
            PhaseTag::Main => match self.partial_main_choice {
                Some(PartialMainPhaseChoice {
                    creatures,
                    edict: Some(edict),
                }) if creatures.len()
                    == self.input.state.creature_choice_size(self.input.player) =>
                {
                    tracing::event!(Level::INFO, "Sending decision to agent");

                    let index = DecisionIndex::encode_main_phase_index(
                        &self.input.state,
                        self.input.player,
                        self.input.hidden.get_main(),
                        creatures,
                        edict,
                    );

                    self.communication.sender.send(index).unwrap();
                    self.decision_sent = true;
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Attempts to read data coming from the bus, and updates the internal state accordingly.
    fn try_accept_input(&mut self) {
        if let Ok(input) = self.communication.receiver.try_recv() {
            tracing::event!(Level::INFO, "Received input from agent");

            self.input = input;

            self.decision_sent = false;
            self.partial_main_choice = if self.input.phase.tag() == PhaseTag::Main {
                Some(PartialMainPhaseChoice::default())
            } else {
                None
            };
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
                ui.vertical(|ui| {
                    // {{{ Prepare data
                    let opponent_creature_possibilities =
                        !(self.input.hidden.get_main() | self.input.state.graveyard);

                    let [my_edict, your_edict] = self.played_edicts();
                    let [my_sabotage, your_sabotage] = self.sabotage_choices();
                    let is_main = self.input.phase.tag() == PhaseTag::Main;
                    let show_sabotage = !is_main;
                    // }}}
                    // {{{ Opponent's board
                    ui.heading("Opponent's board");
                    ui.horizontal(|ui| {
                        self.draw_edict_set(ui, you.edicts);
                    });

                    ui.horizontal(|ui| {
                        self.draw_creature_set(ui, opponent_creature_possibilities);
                    });

                    Grid::new("Opponent's choices").show(ui, |ui| {
                        ui.label("Edict");

                        if show_sabotage {
                            ui.label("Sabotage");
                        }

                        ui.end_row();
                        self.draw_opt_edict(ui, your_edict);

                        if show_sabotage {
                            self.draw_opt_creature(ui, your_sabotage);
                        }

                        ui.end_row();
                    });
                    // }}}
                    ui.separator();
                    // {{{ Player's board
                    ui.heading("Your board");

                    Grid::new("Player's choices").show(ui, |ui| {
                        ui.label("Edict");
                        ui.label("Creatures");

                        if show_sabotage {
                            ui.label("Sabotage");
                        }

                        ui.end_row();
                        self.draw_opt_edict(ui, my_edict);
                        self.draw_creature_set_of_length(
                            ui,
                            self.my_creatures().unwrap_or_default(),
                            self.input.state.creature_choice_size(self.input.player),
                        );

                        if show_sabotage {
                            self.draw_opt_creature(ui, my_sabotage);
                        }

                        ui.end_row();
                    });

                    let can_make_main_choice = is_main && !self.decision_sent;
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
                    // {{{ Communicate
                    if is_main {
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
                            let in_the_past = index < self.input.state.battlefields.current;

                            self.draw_battlefield(
                                ui,
                                self.input.state.battlefields.all[index],
                                false,
                            );

                            if let Some([me, you]) = self.history[index] {
                                self.draw_creature(ui, me.0, false);
                                self.draw_edict(ui, me.1, false);
                                self.draw_opt_creature(ui, me.2);
                                self.draw_opt_creature(ui, you.2);
                                self.draw_edict(ui, you.1, false);
                                self.draw_creature(ui, you.0, false);
                            } else {
                                for _ in 0..6 {
                                    if in_the_past {
                                        self.textures.card_back.show(ui);
                                    } else {
                                        Self::draw_gray_image(ui, &self.textures.card_back);
                                    }
                                }
                            }

                            ui.end_row();
                        }
                    });
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
                                        let res = Self::draw_clickable_image_size(
                                            ui,
                                            &self.textures.battlefields[battlefield as usize],
                                            [bonus_image_size; 2],
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
            input: communication.receiver.recv().unwrap(),
            history: [None; 4],
            partial_main_choice: Some(PartialMainPhaseChoice::default()),
            decision_sent: false,
            textures: AppTextures::new(),
            hovered_card: None,
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

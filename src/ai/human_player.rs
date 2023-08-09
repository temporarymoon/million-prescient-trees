use super::echo_ai::{AgentInput, EchoAgent};
use super::textures::AppTextures;
use crate::cfr::decision_index::DecisionIndex;
use crate::cfr::hidden_index::HiddenState;
use crate::cfr::phase::{MainPhase, PerPhase, SomePhase};
use crate::game::battlefield::Battlefield;
use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::{Edict, EdictSet};
use crate::game::known_state::KnownState;
use crate::game::known_state_summary::KnownStateEssentials;
use crate::game::status_effect::{StatusEffect, StatusEffectSet};
use crate::game::types::Player;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::Pair;
use egui::{Grid, Ui, Vec2, Widget};
use egui_extras::RetainedImage;
use std::format;
use std::sync::mpsc::{Receiver, Sender};

// {{{ General types
type Decision = Option<DecisionIndex>;
// }}}
// {{{ Agent type
struct HumanAgent {
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

/// State used to render the contents of the individual ui tabs.
struct UIState {
    // Received from the agent
    state: KnownState,
    hidden: HiddenState,
    phase: SomePhase,

    // Internal state
    history: [Option<Pair<(Creature, Edict, Option<Creature>)>>; 4],
    partial_main_choice: Option<PartialMainPhaseChoice>,

    // Ui state
    textures: AppTextures,
    hovered_card: Option<HoveredCard>,
    // Communication:
    // sender: Sender<Decision>,
    // receiver: Receiver<AgentInput>,
}
// }}}
// {{{ Agent implementation
impl EchoAgent for HumanAgent {
    fn choose(&self, agent_input: AgentInput) -> Option<DecisionIndex> {
        self.sender.send(agent_input).unwrap();
        self.receiver.recv().unwrap()
    }
}
// }}}
// {{{ UI implementation
impl UIState {
    const CARD_SIZE: [f32; 2] = [50.0, 50.0];

    // {{{ Data helpers
    fn my_creatures(&self) -> Option<CreatureSet> {
        self.hidden
            .choice
            .or_else(|| self.partial_main_choice.map(|p| p.creatures))
    }

    fn played_edicts(&self) -> Pair<Option<Edict>> {
        let choices = match self.phase {
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
        match self.phase {
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
    fn draw_edict(&mut self, ui: &mut Ui, edict: Edict) {
        let res = self.textures.edicts[edict as usize].show(ui);
        if res.hovered() {
            self.hovered_card = Some(HoveredCard::Edict(edict));
        }
    }

    #[inline(always)]
    fn draw_opt_edict(&mut self, ui: &mut Ui, edict: Option<Edict>) {
        if let Some(edict) = edict {
            Self::draw_edict(self, ui, edict);
        } else {
            self.textures.card_back.show(ui);
        };
    }

    #[inline(always)]
    fn draw_creature(&mut self, ui: &mut Ui, creature: Creature) {
        let res = self.textures.creatures[creature as usize].show(ui);

        if res.hovered() {
            self.hovered_card = Some(HoveredCard::Creature(creature));
        }
    }

    #[inline(always)]
    fn draw_opt_creature(&mut self, ui: &mut Ui, creature: Option<Creature>) {
        if let Some(creature) = creature {
            self.draw_creature(ui, creature)
        } else {
            self.textures.card_back.show(ui);
        }
    }

    #[inline(always)]
    fn draw_edict_set(&mut self, ui: &mut Ui, edicts: EdictSet) {
        for edict in edicts {
            Self::draw_edict(self, ui, edict);
        }
    }

    /// Similar to `draw_creature_set`, but will show additional
    /// card backs to get the set to a given size.
    #[inline(always)]
    fn draw_creature_set_of_length(&mut self, ui: &mut Ui, creatures: CreatureSet, len: usize) {
        let remaining = len - creatures.len();

        for creature in creatures {
            self.draw_creature(ui, creature);
        }

        for _ in 0..remaining {
            self.draw_opt_creature(ui, None);
        }
    }

    #[inline(always)]
    fn draw_creature_set(&mut self, ui: &mut Ui, creatures: CreatureSet) {
        for creature in creatures {
            self.draw_creature(ui, creature);
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
        let me = self.state.player_states[0];
        let you = self.state.player_states[1];
        match tab {
            // {{{ Field state
            UITab::Field => {
                ui.vertical(|ui| {
                    // {{{ Prepare data
                    let opponent_creature_possibilities =
                        !(self.hidden.hand | self.state.graveyard);

                    let [my_edict, your_edict] = self.played_edicts();
                    let [my_sabotage, your_sabotage] = self.sabotage_choices();
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
                        ui.label("Sabotage");
                        ui.end_row();
                        self.draw_opt_edict(ui, your_edict);
                        self.draw_opt_creature(ui, your_sabotage);
                        ui.end_row();
                    });
                    // }}}
                    ui.separator();
                    // {{{ Player's board
                    ui.heading("Your board");

                    Grid::new("Player's choices").show(ui, |ui| {
                        ui.label("Edict");
                        ui.label("Sabotage");
                        ui.label("Creatures");
                        ui.end_row();
                        self.draw_opt_edict(ui, my_edict);
                        self.draw_opt_creature(ui, my_sabotage);
                        self.draw_creature_set_of_length(
                            ui,
                            self.my_creatures().unwrap_or_default(),
                            self.state.creature_choice_size(Player::Me),
                        );
                        ui.end_row();
                    });

                    ui.horizontal(|ui| {
                        self.draw_edict_set(ui, me.edicts);
                    });

                    ui.horizontal(|ui| {
                        self.draw_creature_set(ui, self.hidden.hand);
                    });

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
                            let in_the_past = index < self.state.battlefields.current;

                            self.draw_battlefield(ui, self.state.battlefields.all[index], false);

                            if let Some([me, you]) = self.history[index] {
                                self.draw_creature(ui, me.0);
                                self.draw_edict(ui, me.1);
                                self.draw_opt_creature(ui, me.2);
                                self.draw_opt_creature(ui, you.2);
                                self.draw_edict(ui, you.1);
                                self.draw_creature(ui, you.0);
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
                                        let tex = &self.textures.battlefields[battlefield as usize];

                                        let res = egui::ImageButton::new(
                                            tex.texture_id(ui.ctx()),
                                            Vec2::new(bonus_image_size, bonus_image_size),
                                        )
                                        .ui(ui);

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
                                        let tex = &self.textures.creatures[creature as usize];

                                        let res = egui::ImageButton::new(
                                            tex.texture_id(ui.ctx()),
                                            Vec2::new(bonus_image_size, bonus_image_size),
                                        )
                                        .ui(ui);

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
        }
    }
    // }}}
}
// }}}
// {{{ GUIApp stuff
impl GUIApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let battlefields = [
            Battlefield::Night,
            Battlefield::Urban,
            Battlefield::Mountain,
            Battlefield::LastStrand,
        ];

        let mut state = KnownState::new_starting(battlefields);
        let mut hand = CreatureSet::default();
        state.battlefields.current = 2;

        state.player_states[0].edicts.remove(Edict::RileThePublic);
        state.player_states[0].edicts.remove(Edict::DivertAttention);

        for creature in Creature::CREATURES.into_iter().take(4) {
            state.graveyard.insert(creature);
        }

        for creature in (!state.graveyard).into_iter().take(state.hand_size()) {
            hand.insert(creature)
        }

        let mut history = [None; 4];

        history[1] = Some([
            (Creature::Seer, Edict::Ambush, None),
            (Creature::Monarch, Edict::Sabotage, Some(Creature::Witch)),
        ]);

        let ui_state = UIState {
            state,
            hidden: HiddenState::new(hand, None),
            phase: PerPhase::Main(MainPhase::new()),
            history,
            partial_main_choice: Some(PartialMainPhaseChoice::default()),
            textures: AppTextures::new(),
            hovered_card: None,
        };

        let mut tab_tree = egui_dock::Tree::new(vec![UITab::Field, UITab::Effects, UITab::History]);
        tab_tree.split_left(
            egui_dock::tree::node_index::NodeIndex::root(),
            0.33,
            vec![UITab::CardPreview],
        );

        Self {
            tab_tree,
            state: ui_state,
        }
    }

    /// Main rendering function
    fn ui(&mut self, ui: &mut Ui) {
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

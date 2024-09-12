#![feature(let_chains)]
use serde::{Deserialize, Serialize};

use bindings::{game::auto_rogue::types::MicroAction, *};

use client_utils::{
    behaviors::*,
    crdt::{CrdtContainer, ExpiringFWWRegister, SizedFWWExpiringSet, GrowOnlySet},
    find_action, distance,
    framework::{ExplorableMap, State},
};

#[derive(Default, Serialize, Deserialize)]
struct Memory {
    broadcast: Broadcast,
    home_level: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, CrdtContainer)]
struct Broadcast {
    #[crdt]
    map: ExplorableMap,
    #[crdt]
    dungeoneer: ExpiringFWWRegister<i64>,
    #[crdt]
    dedicated_scorers: SizedFWWExpiringSet<i64>,
    #[crdt]
    flag_guards: SizedFWWExpiringSet<i64>,
    #[crdt]
    stair_guards: SizedFWWExpiringSet<i64>,
    #[crdt]
    pop_boosts: GrowOnlySet<i64>,
}

impl Default for Broadcast {
    fn default() -> Self {
        Self {
            map: ExplorableMap::default(),
            dungeoneer: ExpiringFWWRegister::default(),
            flag_guards: SizedFWWExpiringSet::new(3),
            dedicated_scorers: SizedFWWExpiringSet::new(3),
            stair_guards: SizedFWWExpiringSet::new(2),
            pop_boosts: GrowOnlySet::default(),
        }
    }
}

impl State<Broadcast, ExplorableMap> for Memory {
    fn map(&mut self) -> Option<&mut ExplorableMap> {
        Some(&mut self.broadcast.map)
    }

    fn broadcast(&mut self) -> Option<&mut Broadcast> {
        Some(&mut self.broadcast)
    }

    fn run(&mut self) -> Command {
        let current_level = get_game_state().level_id;
        if self.home_level.is_none() {
            self.home_level = Some(current_level);
        }

        let (current_loc, actor) = actor();
        let now = get_game_state().turn;
        let mut is_dungeoneer = false;
        let mut is_flag_guard = self.broadcast.flag_guards.contains(&actor.id);
        let mut is_dedicated_scorer = self.broadcast.dedicated_scorers.contains(&actor.id);
        let mut is_stair_guard = self.broadcast.stair_guards.contains(&actor.id);

        if !is_flag_guard && !is_stair_guard {
            self.broadcast.dungeoneer.set(actor.id, now, now + 500);
            is_dungeoneer = self.broadcast.dungeoneer.get() == Some(&actor.id);
        }

        self.broadcast.dedicated_scorers.1 = (self.broadcast.pop_boosts.len()/4).max(3);
        if !is_dungeoneer {
            self.broadcast.dedicated_scorers.insert(actor.id, now, now + 500);
            is_dedicated_scorer = self.broadcast.dedicated_scorers.contains(&actor.id);
        }

        let inventory = inventory();
        let inventory_full =
            get_character_stats().current.inventory_size as usize <= inventory.len();
        let mut coin_count = 0;
        let mut fruit_count = 0;
        let mut wielded_weapon = None;
        for i in &inventory {
            if i.name == "Coin"{
                coin_count += 1;
            } else if i.name == "Fruit" {
                fruit_count += 1;
            } else if i.name == "Gem" {
                coin_count += 1;
            } else if find_action!(MicroAction::Attack { .. }, i).is_some() {
                if get_equipment_state().right_hand.is_none() {
                    if let Some(command) = equip(i.id, EquipmentSlot::RightHand) {
                        return command
                    }
                }
                if get_equipment_state().right_hand == Some(i.id) {
                    wielded_weapon = Some(i.id);
                }
            }
        }

        if wielded_weapon.is_some() && !is_dungeoneer && !is_dedicated_scorer {
            self.broadcast.flag_guards.insert(actor.id, now, now + 500);
            is_flag_guard = self.broadcast.flag_guards.contains(&actor.id);
            if !is_flag_guard {
                self.broadcast.stair_guards.insert(actor.id, now, now + 500);
                is_stair_guard = self.broadcast.stair_guards.contains(&actor.id);
            }
        }


        if self.home_level == Some(current_level) {
            if is_dungeoneer {
                let baggage: Vec<_> = inventory.iter().map(|i| i.id).filter(|i| Some(*i) != wielded_weapon).collect();
                if !baggage.is_empty() {
                    if let Some((id, _, _)) = find_action!(MicroAction::Drop) {
                        return Command::UseAction((id as u32, Some(ActionTarget::Items(baggage))));
                    }
                }
                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Exit"]) {
                    return command;
                }
            } else if is_stair_guard {
                if let Some(command) = attack_nearest(&[actor.faction]) {
                    return command;
                }
                if let Some(loc) = self.broadcast.map.nearest(&["Exit"]) {
                    if distance(loc, current_loc) > 1.5 {
                        if let Some(command) = self.broadcast.map.move_towards(loc) {
                            return command;
                        }
                    }
                }
            } else if is_flag_guard {
                if let Some(command) = attack_nearest(&[actor.faction]) {
                    return command;
                }

                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Flag"]) {
                    return command;
                }
            } else {
                if !is_dedicated_scorer {
                    if let Some(command) = attack_nearest(&[actor.faction]) {
                        return command;
                    }
                }
                if !inventory_full {
                    if wielded_weapon.is_none() && !is_dedicated_scorer {
                        if let Some(command) = self.broadcast.map.move_towards_nearest(&["Bow", "Sword"]) {
                            return command;
                        }
                    }
                    if let Some(item) = item_at(current_loc) {
                        if !item.is_furniture
                            && (["Gem", "Coin", "Fruit"].contains(&item.name.as_str())
                            || (wielded_weapon.is_none() && ["Bow", "Sword"].contains(&item.name.as_str())))
                            && let Some((id, _action, _micro_action)) = find_action!(MicroAction::PickUp)
                        {
                            return Command::UseAction((id as u32, None));
                        }
                    }
                }
                if let Some(command) = convert() {
                    return command;
                }

                if !inventory_full {
                    if let Some(command) = self.broadcast.map.move_towards_nearest(&["Gem", "Coin", "Fruit"]) {
                        return command;
                    }
                } else if coin_count > 0
                    && let Some(command) = self.broadcast.map.move_towards_nearest(&["Flag"])
                {
                    return command;
                } else if fruit_count > 0
                    && let Some(command) = self.broadcast.map.move_towards_nearest(&["Shrine"])
                {
                    return command;
                }
            }
        } else if !is_dungeoneer {
            if let Some(command) = self.broadcast.map.move_towards_nearest(&["Exit"]) {
                return command;
            }
        } else {
            if !inventory_full {
                if let Some(item) = item_at(current_loc) {
                    if !item.is_furniture
                        && let Some((id, _action, _micro_action)) = find_action!(MicroAction::PickUp)
                    {
                        if item.name == "Special" {
                            self.broadcast.pop_boosts.insert(item.id);
                        }
                        return Command::UseAction((id as u32, None));
                    }
                }
                if let Some(command) = attack_nearest(&[actor.faction, 0]) {
                    return command;
                }
                if self.broadcast.pop_boosts.len() < 3 {
                    if let Some(command) = self.broadcast.map.move_towards_nearest(&["Special", "Exit"]) {
                        return command;
                    }
                } else {
                    if let Some(command) = self.broadcast.map.move_towards_nearest(&["Special", "Bow", "Sword", "Exit"]) {
                        return command;
                    }
                }
            } else {
                if let Some(command) = attack_nearest(&[actor.faction, 0]) {
                    return command;
                }
                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Special", "Exit"]) {
                    return command;
                }
            }
        }

        if let Some(command) = self.broadcast.map.explore() {
            command
        } else if let Some(command) = wander() {
            command
        } else {
            Command::Nothing
        }
    }
}

type Component = client_utils::framework::Component<Memory, Broadcast, ExplorableMap>;
bindings::export!(Component with_types_in bindings);

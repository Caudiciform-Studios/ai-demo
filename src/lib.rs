#![feature(let_chains)]
use serde::{Deserialize, Serialize};

use bindings::{game::auto_rogue::types::MicroAction, *};

use client_utils::{
    behaviors::*,
    crdt::{CrdtContainer, ExpiringFWWRegister, SizedFWWExpiringSet},
    find_action,
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
    flag_guards: SizedFWWExpiringSet<i64>,
}

impl Default for Broadcast {
    fn default() -> Self {
        Self {
            map: ExplorableMap::default(),
            dungeoneer: ExpiringFWWRegister::default(),
            flag_guards: SizedFWWExpiringSet::new(3),
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
        let mut is_guard = self.broadcast.flag_guards.contains(&actor.id);

        if !is_guard {
            if let Some(dungeoneer) = self.broadcast.dungeoneer.get() {
                if *dungeoneer == actor.id {
                    is_dungeoneer = true;
                    self.broadcast.dungeoneer.update_expiry(now + 500);
                }
            } else {
                self.broadcast.dungeoneer.set(actor.id, now, now + 500);
            }
        }

        let inventory = inventory();
        let inventory_full =
            get_character_stats().current.inventory_size as usize <= inventory.len();
        let mut coin_count = 0;
        let mut fruit_count = 0;
        let mut has_weapon = false;
        let mut extra_weapons = vec![];
        for i in &inventory {
            if i.name == "Coin" {
                coin_count += 1;
            } else if i.name == "Fruit" {
                fruit_count += 1;
            } else if find_action!(MicroAction::Attack { .. }, i).is_some() {
                if get_equipment_state().right_hand.is_none() {
                    if let Some(command) = equip(i.id, EquipmentSlot::RightHand) {
                        return command
                    }
                }
                if get_equipment_state().right_hand != Some(i.id) {
                    extra_weapons.push(i.id);
                } else {
                    has_weapon = true;
                }
            }
        }

        if has_weapon && !is_dungeoneer {
            self.broadcast.flag_guards.insert(actor.id, now, now + 500);
            is_guard = self.broadcast.flag_guards.contains(&actor.id);
        }


        if self.home_level == Some(current_level) {
            if is_dungeoneer {
                if !extra_weapons.is_empty() {
                    if let Some((id, _, _)) = find_action!(MicroAction::Drop) {
                        return Command::UseAction((id as u32, Some(ActionTarget::Items(extra_weapons))));
                    }
                }
                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Exit"]) {
                    return command;
                }
            } else if is_guard {
                if let Some(command) = attack_nearest() {
                    return command;
                }

                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Flag"]) {
                    return command;
                }
            } else {
                if let Some(command) = attack_nearest() {
                    return command;
                }
                if !inventory_full {
                    if !has_weapon {
                        if let Some(command) = self.broadcast.map.move_towards_nearest(&["Bow", "Sword"]) {
                            return command;
                        }
                    }
                    if let Some(item) = item_at(current_loc) {
                        if !item.is_furniture
                            && (["Coin", "Fruit"].contains(&item.name.as_str())
                            || (!has_weapon && ["Bow", "Sword"].contains(&item.name.as_str())))
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
                    if let Some(command) = self.broadcast.map.move_towards_nearest(&["Coin", "Fruit"]) {
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
                        return Command::UseAction((id as u32, None));
                    }
                }
            }
            if self.broadcast.map.nearest(&["Coin"]).is_some() {
                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Exit"]) {
                    return command;
                }
            } else {
                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Special"]) {
                    return command;
                }

                if let Some(command) = self.broadcast.map.move_towards_nearest(&["Bow", "Sword"]) {
                    return command;
                }
            }

            if let Some(command) = self.broadcast.map.move_towards_nearest(&["Exit"]) {
                return command;
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

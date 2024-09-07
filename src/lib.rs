#![feature(let_chains)]
use serde::{Deserialize, Serialize};

use bindings::{game::auto_rogue::types::MicroAction, *};

use client_utils::{
    behaviors::convert,
    crdt::CrdtContainer,
    find_action,
    framework::{ExplorableMap, State},
};

#[derive(Default, Serialize, Deserialize)]
struct Memory {
    broadcast: Broadcast,
}

#[derive(Default, Debug, Serialize, Deserialize, CrdtContainer)]
struct Broadcast {
    map: ExplorableMap,
}

impl State<Broadcast, ExplorableMap> for Memory {
    fn map(&mut self) -> Option<&mut ExplorableMap> {
        Some(&mut self.broadcast.map)
    }

    fn broadcast(&mut self) -> Option<&mut Broadcast> {
        Some(&mut self.broadcast)
    }

    fn run(&mut self) -> Command {
        let inventory = inventory();
        let inventory_full =
            get_character_stats().current.inventory_size as usize <= inventory.len();
        let mut coin_count = 0;
        let mut fruit_count = 0;
        for i in &inventory {
            if i.name == "Coin" {
                coin_count += 1;
            } else if i.name == "Fruit" {
                fruit_count += 1;
            }
        }
        let (current_loc, _) = actor();

        if let Some(command) = convert() {
            return command;
        }

        if !inventory_full {
            if let Some(item) = item_at(current_loc) {
                if !item.is_furniture
                    && let Some((id, _action, _micro_action)) = find_action!(MicroAction::PickUp)
                {
                    return Command::UseAction((id as u32, None));
                }
            }

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

        if let Some(command) = self.broadcast.map.explore() {
            command
        } else {
            Command::Nothing
        }
    }
}

type Component = client_utils::framework::Component<Memory, Broadcast, ExplorableMap>;
bindings::export!(Component with_types_in bindings);

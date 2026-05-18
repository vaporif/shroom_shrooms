use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct EntitySprites {
    pub fragment: Handle<Image>,
    pub plant_root: Handle<Image>,
    pub fauna: Handle<Image>,
    pub mushroom: Handle<Image>,
    pub neutral_fungus: Handle<Image>,
    pub hive: Handle<Image>,
    pub loaded: bool,
}

pub fn load_entity_sprites(mut sprites: ResMut<EntitySprites>, asset_server: Res<AssetServer>) {
    if sprites.loaded {
        return;
    }
    sprites.fragment = asset_server.load("sprites/fragment.png");
    sprites.plant_root = asset_server.load("sprites/plant_root.png");
    sprites.fauna = asset_server.load("sprites/fauna.png");
    sprites.mushroom = asset_server.load("sprites/mushroom.png");
    sprites.neutral_fungus = asset_server.load("sprites/neutral_fungus.png");
    sprites.hive = asset_server.load("sprites/neutral_fungus.png");
    sprites.loaded = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_sprites_resource_has_all_handles() {
        let sprites = EntitySprites::default();
        assert_eq!(sprites.fragment, Handle::default());
        assert_eq!(sprites.plant_root, Handle::default());
        assert_eq!(sprites.fauna, Handle::default());
        assert_eq!(sprites.mushroom, Handle::default());
        assert_eq!(sprites.neutral_fungus, Handle::default());
        assert_eq!(sprites.hive, Handle::default());
    }
}

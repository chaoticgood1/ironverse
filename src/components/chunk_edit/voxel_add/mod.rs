use bevy::{prelude::*, input::ButtonState, utils::HashMap};
use rapier3d::prelude::{Point, ColliderBuilder, InteractionGroups, Group, Isometry};
use voxels::{data::voxel_octree::VoxelMode, utils::key_to_world_coord_f32};
use crate::{components::{chunk::{Chunks, Mesh}, player::Player}, input::{hotbar::HotbarResource, MouseInput}, data::GameResource, physics::Physics};
use super::{ChunkEdit, ChunkEditParams, EditState};

pub mod normal;
pub mod snap;

pub struct CustomPlugin;
impl Plugin for CustomPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugin(normal::CustomPlugin)
      .add_plugin(snap::CustomPlugin)
      .add_system(on_edit.run_if(add_state));
  }
}

fn add_state(state: Res<State<EditState>>) -> bool {
  state.0 == EditState::AddNormal ||
  state.0 == EditState::AddSnap
}

fn on_edit(
  hotbar_res: Res<HotbarResource>,

  mut game_res: ResMut<GameResource>,
  mut mouse_inputs: EventReader<MouseInput>,
  mut physics: ResMut<Physics>,
  mut edits: Query<(&ChunkEdit, &ChunkEditParams, &mut Chunks)>,
) {
  let mut edit_chunk = false;
  for event in mouse_inputs.iter() {
    if event.mouse_button_input.state == ButtonState::Pressed 
    && event.mouse_button_input.button == MouseButton::Left {
      edit_chunk = true;
    }
  }
  if !edit_chunk {
    return;
  }

  let mut voxel = 0;
  for i in 0..hotbar_res.bars.len() {
    let bar = &hotbar_res.bars[i];
    if  hotbar_res.selected_keycode == bar.key_code {
      voxel = bar.voxel;
    }
  }

  for (edit, params, mut chunks) in &mut edits {
    if edit.position.is_none() { continue; }

    let point = edit.position.unwrap();
    let mut res = HashMap::new();
    let min = 0;
    let max = params.size as i64;
    for x in min..max {
      for y in min..max {
        for z in min..max {

          let pos = [
            point.x as i64 + x,
            point.y as i64 + y,
            point.z as i64 + z
          ];
          let chunks = game_res.chunk_manager.set_voxel2(&pos, voxel);
          for (key, chunk) in chunks.iter() {
            res.insert(key.clone(), chunk.clone());
          }
        }
      }
    }

    let config = game_res.chunk_manager.config.clone();
    for (key, chunk) in res.iter() {
      'inner: for i in 0..chunks.data.len() {
        let m = &chunks.data[i];

        if key == &m.key {
          physics.remove_collider(m.handle);
          chunks.data.swap_remove(i);
          break 'inner;
        }
      }
      

      let data = chunk.octree.compute_mesh(
        VoxelMode::SurfaceNets, 
        &mut game_res.chunk_manager.voxel_reuse.clone(),
        &game_res.colors,
      );

      
      if data.indices.len() > 0 {
        let pos_f32 = key_to_world_coord_f32(key, config.seamless_size);
        let mut pos = Vec::new();
        for d in data.positions.iter() {
          pos.push(Point::from([d[0], d[1], d[2]]));
        }
    
        let mut indices = Vec::new();
        for ind in data.indices.chunks(3) {
          // println!("i {:?}", ind);
          indices.push([ind[0], ind[1], ind[2]]);
        }
    
        let mut collider = ColliderBuilder::trimesh(pos, indices)
          .collision_groups(InteractionGroups::new(Group::GROUP_1, Group::GROUP_2))
          .build();
        collider.set_position(Isometry::from(pos_f32));
    
        let handle = physics.collider_set.insert(collider);

        let mut c = chunk.clone();
        c.is_default = false;
        chunks.data.push(Mesh {
          key: key.clone(),
          chunk: c,
          data: data.clone(),
          handle: handle,
        });
      }
    }
  }

}




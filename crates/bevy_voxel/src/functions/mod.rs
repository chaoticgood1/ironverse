mod sphere;
mod cube;


use bevy::prelude::*;
use utils::Utils;
use crate::{BevyVoxelResource, Selected, Preview, Chunks, Center, ChunkData, ShapeState, EditState, MeshComponent};

use cfg_if::cfg_if;
cfg_if! {
  if #[cfg(target_arch = "wasm32")] {
    use multithread::plugin::PluginResource;
  }
}

// cfg_if! {
//   if #[cfg(not(target_arch = "wasm32"))] {
    mod async_loading;
//   }
// }


pub struct CustomPlugin;
impl Plugin for CustomPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugin(sphere::CustomPlugin)
      .add_plugin(cube::CustomPlugin)
      .insert_resource(BevyVoxelResource::default())
      .add_startup_system(startup)
      .add_system(update)
      .add_system(detect_selected_voxel_position)
      .add_system(load_main_chunks)
      // .add_system(load_lod_chunks.after(load_main_chunks))
      .add_system(center_changed)
      // .add_system(receive_chunks)
      .add_system(receive_mesh)
      .add_system(load_lod_center_changed)
      .add_system(shape_state_changed);

    // cfg_if! {
    //   if #[cfg(not(target_arch = "wasm32"))] {
        app
          .add_plugin(async_loading::CustomPlugin);
    //   }
    // }
  }
}

fn startup() {
  println!("startup BevyVoxel");
}

fn update(
  mut res: ResMut<BevyVoxelResource>,
  shape_state: Res<State<ShapeState>>,
  edit_state: Res<State<EditState>>,
) {
  res.physics.step();
  res.shape_state = shape_state.0;
  res.edit_state = edit_state.0;
}

fn detect_selected_voxel_position(
  mut cam: Query<(&Transform, &mut Selected), With<Selected>>,
  bevy_voxel_res: Res<BevyVoxelResource>,
) {
  for (cam_trans, mut selected) in &mut cam {
    let hit = bevy_voxel_res.get_raycast_hit(cam_trans);
    if hit.is_none() {
      continue;
    }

    let pos = bevy_voxel_res.get_hit_voxel_pos(hit.unwrap());
    if pos.is_none() && selected.pos.is_some() {
      selected.pos = pos;
    }

    if pos.is_some() {
      if selected.pos.is_some() {
        let p = pos.unwrap();
        let current = selected.pos.unwrap();
        if current != p {
          selected.pos = pos;
        }
      }
      
      if selected.pos.is_none() {
        selected.pos = pos;
      }
    }
  }
}

fn load_main_chunks(
  mut res: ResMut<BevyVoxelResource>,
  mut chunks: Query<(&Center, &mut Chunks, &mut MeshComponent), Added<Chunks>>
) {
  for (center, mut chunks, mut mesh_comp) in &mut chunks {
    let lod = res.chunk_manager.depth as u8;
    let keys = res.get_keys_by_lod(center.key, lod);
    let tmp_c = res.load_chunks(&keys);
    for c in tmp_c.iter() {
      chunks.data.insert(c.key, c.clone());
    }
    chunks.added_keys.append(&mut keys.clone());
    
    mesh_comp.added.clear();
    let data = res.load_mesh_data(&tmp_c);
    for d in data.iter() {
      mesh_comp.data.insert(d.key, d.clone());
      mesh_comp.added.push(d.clone());
    }
  }
}

fn load_lod_chunks(
  mut res: ResMut<BevyVoxelResource>,
  mut chunks: Query<(&Center, &mut Chunks, &mut MeshComponent), Added<Chunks>>
) {
  for (center, mut chunks, mut mesh_comp) in &mut chunks {
    let lod = res.chunk_manager.depth as u8;
    let keys = res.get_keys_by_lod(center.key, lod - 1);
    request_load_chunk(&keys, &mut res);
  }
}

fn center_changed(
  mut res: ResMut<BevyVoxelResource>,
  mut centers: Query<(&Center, &mut Chunks, &mut MeshComponent), Changed<Center>>
) {
  for (center, mut chunks, mut mesh_comp) in &mut centers {
    let lod = res.chunk_manager.depth as u8;
    let keys = res.get_delta_keys_by_lod(
      center.prev_key, center.key, lod
    );

    let tmp_c = res.load_chunks(&keys);
    for c in tmp_c.iter() {
      chunks.data.insert(c.key, c.clone());
    }
    chunks.added_keys.clear();
    chunks.added_keys.append(&mut keys.clone());


    mesh_comp.added.clear();
    let data = res.load_mesh_data(&tmp_c);
    for d in data.iter() {
      mesh_comp.data.insert(d.key, d.clone());
      mesh_comp.added.push(d.clone());
    }
  }
}

fn load_lod_center_changed(
  mut res: ResMut<BevyVoxelResource>,
  mut centers: Query<(&Center, &mut Chunks, &mut MeshComponent), Changed<Center>>
) {
  for (center, mut chunks, mut mesh_comp) in &mut centers {
    let lod = res.chunk_manager.depth as u8;
    let keys = res.get_delta_keys_by_lod(
      center.prev_key, center.key, lod - 1
    );
    request_load_chunk(&keys, &mut res);
  }
}





fn shape_state_changed(
  shape_state: Res<State<ShapeState>>,
  mut local: Local<ShapeState>,
  mut previews: Query<&mut Preview>,

  edit_state: Res<State<EditState>>,
  mut local1: Local<EditState>,
) {
  if *local != shape_state.0 {
    *local = shape_state.0;
    for mut preview in &mut previews {
      preview.size = preview.size;
    }
  }

  if *local1 != edit_state.0 {
    *local1 = edit_state.0;
    for mut preview in &mut previews {
      preview.size = preview.size;
    }
  }
  
}



fn request_load_chunk(
  keys: &Vec<[i64; 3]>, 
  bevy_voxel_res: &mut BevyVoxelResource
) {
  for key in keys.iter() {
    let _ = bevy_voxel_res.send_key.send(*key);
  }
}

fn receive_chunks(
  mut res: ResMut<BevyVoxelResource>,
  mut queries: Query<(&Center, &mut Chunks, &mut MeshComponent)>
) {
  for c in res.recv_chunk.drain() {
    for (center, mut chunks, mut mesh_comp) in &mut queries {
      chunks.data.insert(c.key, c.clone());

      
      res.send_process_mesh.send(c.clone());
    }
  }
}

fn receive_mesh(
  mut res: ResMut<BevyVoxelResource>,
  mut queries: Query<(&Center, &mut Chunks, &mut MeshComponent)>
) {
  let max_lod = res.chunk_manager.depth as u8;
  let ranges = res.ranges.clone();
  for data in res.recv_mesh.drain() {
    for (center, mut chunks, mut mesh_comp) in &mut queries {
      let d = data.clone();
      mesh_comp.data.insert(d.key, d);

      mesh_comp.added.push(data.clone());


      // if data.lod == max_lod {
      //   let max_range = ranges[1] as i64;
      //   if Utils::get_tile_range(&center.key, &data.key) <= max_range {
      //     mesh_comp.added.push(data.clone());
      //   }
      // }

      // if data.lod == max_lod - 1 {
      //   let min = ranges[1] as i64 + 1;
      //   let max = ranges[2] as i64;
      //   if Utils::get_tile_range(&center.key, &data.key) >= min &&
      //   Utils::get_tile_range(&center.key, &data.key) <= max {
      //     mesh_comp.added.push(data.clone());
      //   }
      // }
      
    }
  }
}


#[derive(Resource)]
struct LocalResource {

}





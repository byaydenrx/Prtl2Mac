# Upstream "Known gaps" audit (2026-10-21)

Every "Known gaps" item in the READMEs of the engine we started from
(github.com/Stink-O/Portal2-rust-engine, checked at 18230b3; our crate READMEs
were never updated, so each item was checked against our code). Items marked
FIXED have code doing it here; OPEN items are tasks in TASKS.md.

| Upstream item (crate) | Here | Where / task |
|---|---|---|
| No levels, toolkit only (root) | FIXED | the whole game loop plays the maps |
| Cubes can't be picked up or go through portals (root) | FIXED | props.rs carry; T-102 owner-confirmed |
| One portal pair (root, p2_render) | OPEN (unverified) | single-player gun pair + map portals work; several linked pairs at once not checked: T-205 |
| Chell: no head look, turn in place, foot IK; no body shadow (root, p2_body) | OPEN | T-208 |
| Placement sparks / sprite sheets not decoded (root, p2_fx) | OPEN | render.rs SheetGrids empty: every sheet particle draws the whole atlas: T-199 |
| Sounds not panned (root, p2_fx) | OPEN | distance gain only: T-206 |
| WAV only (p2_fx) | FIXED | audio.rs decodes PCM and the MP3 frames in .wav |
| Portal rim faint, projectile dim (p2_fx) | OPEN (minor) | T-210 |
| Refraction particles ignore $bluramount (p2_fx) | OPEN (minor) | T-200 |
| Collision via traces, Lerp Initial Scalar, $depthblend soft particles, operator time offset/scale (p2_fx) | OPEN | T-200 (Restart Effect after Duration is FIXED) |
| No-hit (sky) shots play nothing (p2_fx) | OPEN (minor) | T-210 |
| Fizzle when the portal's surface goes away (p2_gun) | PARTIAL | portals.rs fizzles on listed movers; general re-test missing: T-205 |
| Eye behind a portal / shooting out of the far side (p2_gun) | OPEN | T-205 |
| No-portal volumes, noportal surfaces (p2_gun) | FIXED | portals.rs func_noportal_volume, map.rs noportal |
| Placement helpers (p2_gun) | FIXED | portals.rs info_placement_helper |
| Player standing on a floor portal spot (p2_gun) | OPEN | T-205 |
| Kinematic bodies block shots (p2_gun) | OPEN (minor) | T-210 |
| Footsteps, use key (p2_movement) | FIXED | p2_fx step_sounds, use in props.rs |
| Mouse sensitivity vs a real config (p2_movement) | OPEN (minor) | T-210 |
| Viewmodel/particles through portals (p2_render) | as Source | Source doesn't draw the viewmodel through portals either |
| p2_room test-room gaps | n/a | test room only, not used by the game |
| Transit copies only cuboid remote geometry (p2_transit) | OPEN | T-209 |
| No mobile portals / bowie / non-player transit (p2_transit) | PARTIAL | props transit fixed (T-102); mobile portals T-205 |
| View punch on fire (p2_view) | FIXED | p2_view punch_on_fire |
| Roll compensation, viewmodel portal clip, viewmodel room lighting (p2_view) | OPEN | T-207 |
| Shutdown crash (p2_view) | OPEN (minor) | T-210 |
| Refract, water, sky, sprites on fallback (source_assets) | FIXED | glass.rs (glass, water), p2_fx sprites |
| UnlitTwoTexture monitors on fallback (source_assets) | OPEN | T-201 |
| $phong / bump on models (source_assets) | OPEN | T-152 (on hold for screenshots) |
| env_cubemap on props/brushes (source_assets) | OPEN | no BSP cubemaps loaded: T-204 |
| Dynamic local lights on props (source_assets) | OPEN | T-204 |
| Auto-exposure (source_assets) | FIXED | p2_game exposure.rs (env_tonemap_controller) |
| Bloom, color-correction LUT (source_assets) | OPEN | T-202 |
| Lightmap styles (switchable lights) (source_assets) | OPEN | T-203 |
| Animated textures, scrolling, proxies (source_assets) | FIXED | proxy.rs |
| Brush entities static, props one frame (source_assets) | FIXED | movers.rs, anim_props.rs |
| Displacement lightmap UVs approximate (source_assets) | OPEN (minor) | T-204 |
| Skybox face orientation unverified (source_assets) | OPEN (minor) | T-210 |
| Physics adapter, Bevy 0.20 (root) | n/a | engine upkeep, not gameplay |

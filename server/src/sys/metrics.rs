use crate::{
    metrics::{EcsSystemMetrics, PhysicsMetrics, TickMetrics},
    Tick, TickStart,
};
use common::{
    metrics::SysMetrics,
    resources::TimeOfDay,
    terrain::TerrainGrid,
    vsystem::{Origin, Phase, VJob, VSystem},
};
use specs::{Entities, Join, Read, ReadExpect};
use std::time::Instant;

/// This system exports metrics
#[derive(Default)]
pub struct Sys;
impl<'a> VSystem<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Option<Entities<'a>>,
        ReadExpect<'a, Tick>,
        ReadExpect<'a, TimeOfDay>,
        ReadExpect<'a, TickStart>,
        Option<Read<'a, TerrainGrid>>,
        Read<'a, SysMetrics>,
        Read<'a, common::metrics::PhysicsMetrics>,
        ReadExpect<'a, EcsSystemMetrics>,
        ReadExpect<'a, TickMetrics>,
        ReadExpect<'a, PhysicsMetrics>,
    );

    const NAME: &'static str = "metrics";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Apply;

    fn run(
        _job: &mut VJob<Self>,
        (
            entities,
            tick,
            time_of_day,
            tick_start,
            terrain,
            sys_metrics,
            phys_metrics,
            export_ecs,
            export_tick,
            export_physics,
        ): Self::SystemData,
    ) {
        const NANOSEC_PER_SEC: f64 = std::time::Duration::from_secs(1).as_nanos() as f64;

        let start = Instant::now();

        let mut state = sys_metrics.stats.lock().unwrap();
        //this system hasn't run yet
        state.remove(Self::NAME);

        for (name, stat) in common::vsystem::gen_stats(&state, tick_start.0, 8, 8) {
            export_ecs
                .system_start_time
                .with_label_values(&[&name])
                .set(stat.start_ns() as i64);
            export_ecs
                .system_thread_avg
                .with_label_values(&[&name])
                .set(stat.avg_threads() as f64);
            let len = stat.length_ns() as i64;
            export_ecs
                .system_length_time
                .with_label_values(&[&name])
                .set(len);
            export_ecs
                .system_length_hist
                .with_label_values(&[&name])
                .observe(len as f64 / NANOSEC_PER_SEC);
        }

        // Report other info
        export_tick.time_of_day.set(time_of_day.0);
        if tick.0.rem_euclid(100) == 0 {
            if let Some(terrain) = terrain {
                let mut chonk_cnt = 0;
                let mut group_cnt = 0;
                let chunk_cnt = terrain.iter().fold(0, |a, (_, c)| {
                    chonk_cnt += 1;
                    group_cnt += c.sub_chunk_groups();
                    a + c.sub_chunks_len()
                });
                export_tick.chonks_count.set(chonk_cnt as i64);
                export_tick.chunks_count.set(chunk_cnt as i64);
                export_tick.chunk_groups_count.set(group_cnt as i64);
            }

            if let Some(entities) = entities {
                let entity_count = entities.join().count();
                export_tick.entity_count.set(entity_count as i64);
            }
        }

        //detailed physics metrics
        export_physics
            .entity_entity_collision_checks_count
            .inc_by(phys_metrics.entity_entity_collision_checks);
        export_physics
            .entity_entity_collisions_count
            .inc_by(phys_metrics.entity_entity_collisions);

        // export self time as best as possible
        export_ecs
            .system_start_time
            .with_label_values(&["metrics"])
            .set(start.duration_since(tick_start.0).as_nanos() as i64);
        export_ecs
            .system_thread_avg
            .with_label_values(&["metrics"])
            .set(1.0);
        let len = start.elapsed().as_nanos() as i64;
        export_ecs
            .system_length_time
            .with_label_values(&["metrics"])
            .set(len);
        export_ecs
            .system_length_hist
            .with_label_values(&["metrics"])
            .observe(len as f64 / NANOSEC_PER_SEC);
    }
}

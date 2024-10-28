//====================================================================

use std::collections::HashMap;

use shipyard::World;

use crate::builder::{Label, StageData};

//====================================================================

pub struct WorkloadRunner {
    stages: HashMap<Label, StageData>,
    stage_order: Vec<Label>,
}

impl WorkloadRunner {
    pub fn new(stages: HashMap<Label, StageData>) -> Self {
        let mut stage_data = stages
            .iter()
            .filter_map(|(label, data)| match data.priority != 0 {
                true => Some((label, data)),
                false => None,
            })
            .collect::<Vec<_>>();

        stage_data.sort_by(|a, b| a.1.priority.cmp(&b.1.priority));
        let stage_order = stage_data
            .into_iter()
            .map(|(label, _)| label.clone())
            .collect();

        Self {
            stages,
            stage_order,
        }
    }

    pub fn prep(&self, world: &World) {
        self.stages.iter().for_each(|(stage, data)| {
            if data.priority == 0 {
                log::info!("Running setup system {:?}", stage);
                world.run_workload(stage.clone()).unwrap()
            }
        });
    }

    pub fn run(&self, world: &World) {
        self.stage_order.iter().for_each(|stage| {
            let data = self.stages.get(stage).unwrap();

            if data.disabled {
                return;
            }

            let run = match &data.run_condition {
                Some(run_condition) => run_condition(world),
                None => true,
            };

            if !run {
                return;
            }

            world.run_workload(stage.clone()).unwrap();
        });
    }
}

//====================================================================

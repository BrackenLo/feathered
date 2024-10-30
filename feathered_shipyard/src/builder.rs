//====================================================================

use std::{collections::HashMap, fmt::Debug, hash::Hash};

use shipyard::{info::TypeId, AsLabel, WorkloadModificator, World};

use crate::runner::WorkloadRunner;

//====================================================================

pub type Label = Box<dyn shipyard::Label>;

//====================================================================

pub trait Stage: shipyard::Label + Debug {}

stage_macros::create_stage!(Setup);
stage_macros::create_stage!(First);
stage_macros::create_stage!(FixedUpdate);
stage_macros::create_stage!(Update);
stage_macros::create_stage!(Render);
stage_macros::create_stage!(Last);

pub fn register_main_stages(builder: &mut WorkloadBuilder) {
    builder
        .register_stage(Setup, StageData::from_priority(0), None)
        .register_stage(First, StageData::from_priority(10), None)
        .register_stage(Update, StageData::from_priority(20), None)
        .register_stage(Render, StageData::from_priority(30), None)
        .register_stage(Last, StageData::from_priority(40), None);
}

mod stage_macros {
    macro_rules! create_stage {
        (
            $stage_name: ident
        ) => {
            #[derive(shipyard::Label, Debug, Clone, Hash, PartialEq)]
            pub struct $stage_name;
            impl Stage for $stage_name {}
        };
    }

    pub(crate) use create_stage;
}

#[derive(shipyard::Label, Hash, Debug, Clone, Copy, PartialEq, Eq, enum_iterator::Sequence)]
pub enum SubStages {
    First,
    Pre,
    Main,
    Post,
    Last,
}

impl Iterator for SubStages {
    type Item = SubStages;

    fn next(&mut self) -> Option<Self::Item> {
        let next = enum_iterator::Sequence::next(self);

        if let Some(next) = next {
            *self = next;
        }
        next
    }
}

//====================================================================

pub struct StageData {
    pub priority: u32,
    pub run_condition: Option<Box<dyn Fn(&World) -> bool>>,
    pub disabled: bool,
}

impl StageData {
    pub fn from_priority(priority: u32) -> Self {
        Self {
            priority,
            run_condition: None,
            disabled: false,
        }
    }
}

//====================================================================

pub struct WorkloadBuilder<'a> {
    world: &'a World,
    inner: WorkloadBuilderInner,
}

pub struct WorkloadBuilderInner {
    stages: HashMap<Label, StageData>,
    workloads: HashMap<Label, WorkloadToBuild>,

    registered_workload_names: HashMap<String, String>, // Type ID : Workload Name
    registered_plugins: Vec<TypeId>,

    build_tabs: u8,
    build_text: String,
}

pub struct WorkloadToBuild {
    pub main: shipyard::Workload,
    pub substages: HashMap<SubStages, shipyard::Workload>,
}

impl WorkloadToBuild {
    pub fn new(stage: impl shipyard::Label) -> Self {
        Self {
            main: shipyard::Workload::new(stage),
            substages: HashMap::new(),
        }
    }
}

//--------------------------------------------------

impl<'a> WorkloadBuilder<'a> {
    pub fn new(world: &'a World) -> Self {
        Self {
            world,
            inner: WorkloadBuilderInner {
                stages: HashMap::default(),
                workloads: HashMap::default(),

                registered_workload_names: HashMap::new(),
                registered_plugins: Vec::new(),

                build_tabs: 0,
                build_text: "Setting up Workload Builder".to_string(),
            },
        }
    }

    pub fn build(mut self) -> WorkloadRunner {
        log::trace!("{}", self.inner.build_text);

        // self.inner.stages.iter().for_each(|(id, (stage, _))| {
        //     match self.inner.workloads.remove(id) {
        //         Some(mut to_build) => {
        //             // Iterate through all substages in this stage and add them to the main workload
        //             // Afterwards, add main workload to world

        //             enum_iterator::all::<SubStages>()
        //                 .into_iter()
        //                 .fold(to_build.main, |acc, substage| {
        //                     // Check and add substage if it exists
        //                     if let Some(workload) = to_build.substages.remove(&substage) {
        //                         let workload = workload.tag(substage);
        //                         return acc.merge(
        //                             substage.into_iter().fold(workload, |acc, substage_after| {
        //                                 acc.before_all(substage_after)
        //                             }),
        //                         );
        //                     }

        //                     acc
        //                 })
        //                 .add_to_world(self.world)
        //                 .unwrap();
        //         }

        //         // Stage doesn't exist. Throw Warning
        //         None => log::warn!("Failed to get stage {:?}", stage),
        //     }
        // });

        self.inner.workloads.drain().for_each(|(_, mut to_build)| {
            enum_iterator::all::<SubStages>()
                .into_iter()
                .fold(to_build.main, |acc, substage| {
                    // Check and add substage if it exists
                    if let Some(workload) = to_build.substages.remove(&substage) {
                        let workload = workload.tag(substage);
                        return acc.merge(
                            substage.into_iter().fold(workload, |acc, substage_after| {
                                acc.before_all(substage_after)
                            }),
                        );
                    }

                    acc
                })
                .add_to_world(self.world)
                .unwrap();
        });

        // Print debug data
        let data = self.world.workloads_info().0.iter().fold(
            String::from("Building workloads. Registered Stages and functions:"),
            |acc, (name, workload_info)| {
                let name = match self.inner.registered_workload_names.get(name) {
                    Some(event_name) => event_name,
                    None => name,
                };

                let acc = format!("{}\n{}", acc, name);

                workload_info
                    .batch_info
                    .iter()
                    .fold(acc, |acc, batch_info| {
                        batch_info
                            .systems()
                            .fold(acc, |acc, system| format!("{}\n    {}", acc, system.name))
                    })
            },
        );

        log::debug!("{data}");

        WorkloadRunner::new(self.inner.stages)
    }
}

//====================================================================

impl<'a> WorkloadBuilder<'a> {
    #[inline]
    pub fn get_inner(&mut self) -> &mut WorkloadBuilderInner {
        &mut self.inner
    }

    #[inline]
    pub fn get_world(&mut self) -> &World {
        &self.world
    }
}

//====================================================================

pub trait Plugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder);
}

impl<'a> WorkloadBuilder<'a> {
    pub fn add_plugin<T: Plugin + 'static>(&mut self, plugin: T) -> &mut Self {
        let plugin_id = TypeId::of::<T>();

        if self.inner.registered_plugins.contains(&plugin_id) {
            self.inner.log(format!(
                "Skipping already added plugin '{}'",
                std::any::type_name::<T>()
            ));
            return self;
        }

        self.inner
            .log(format!("Adding plugin '{}'", std::any::type_name::<T>()));
        self.inner.build_tabs += 1;

        plugin.build_plugin(self);

        self.inner.registered_plugins.push(plugin_id);
        self.inner.build_tabs -= 1;
        self
    }
}

//====================================================================

impl<'a> WorkloadBuilder<'a> {
    pub fn register_stage<S: Stage + Clone>(
        &mut self,
        stage: S,
        data: StageData,
        on_insert: Option<Box<dyn FnOnce(&mut WorkloadBuilder)>>,
    ) -> &mut Self {
        if let Some(on_insert) = on_insert {
            on_insert(self);
        }

        let label = stage.as_label();

        self.inner.stages.insert(stage.as_label(), data);
        self.inner
            .workloads
            .insert(label, WorkloadToBuild::new(stage));

        self
    }

    workload_macros::create_workload_stage!(add_workload_first, SubStages::First);
    workload_macros::create_workload_stage!(add_workload_pre, SubStages::Pre);
    workload_macros::create_workload_stage!(add_workload, SubStages::Main);
    workload_macros::create_workload_stage!(add_workload_post, SubStages::Post);
    workload_macros::create_workload_stage!(add_workload_last, SubStages::Last);

    #[inline]
    pub fn insert<U: shipyard::Unique + Send + Sync>(&mut self, unique: U) -> &mut Self {
        self.world.add_unique(unique);
        self
    }
}

//--------------------------------------------------

mod workload_macros {
    macro_rules! create_workload_stage {
        ($loader_name: ident, $sub_stage: expr) => {
            pub fn $loader_name<Views, R, Sys>(
                &mut self,
                stage: impl Stage,
                workload: Sys,
            ) -> &mut Self
            where
                Sys: shipyard::IntoWorkload<Views, R>,
                R: 'static,
            {
                self.inner
                    .add_workload_sub(stage, $sub_stage, workload.into_workload());
                self
            }
        };
    }
    pub(crate) use create_workload_stage;
}

//====================================================================

impl WorkloadBuilderInner {
    #[inline]
    pub fn get_workloads(&mut self) -> &mut HashMap<Label, WorkloadToBuild> {
        &mut self.workloads
    }

    #[inline]
    pub fn register_workload_name<T: 'static>(&mut self, name: String) -> &mut Self {
        self.registered_workload_names
            .insert(format!("{:?}", shipyard::info::TypeId::of::<T>()), name);
        self
    }

    pub fn log(&mut self, text: String) {
        let tabs = (0..self.build_tabs).map(|_| "\t").collect::<String>();
        self.build_text = format!("{}\n{}⌙ {}", self.build_text, tabs, text);
    }

    pub fn add_workload_sub<S: shipyard::Label + Debug>(
        &mut self,
        workload_id: S,
        substage: SubStages,
        workload: shipyard::Workload,
    ) {
        self.log(format!(
            "Adding workload for '{:?}' - substage {:?}",
            workload_id, substage
        ));

        let label = workload_id.as_label();

        let mut old_workload = self
            .workloads
            .remove(&label)
            .unwrap_or(WorkloadToBuild::new(workload_id));

        let new_substage = match old_workload.substages.remove(&substage) {
            Some(old_substage) => old_substage.merge(workload),
            None => workload,
        };

        old_workload.substages.insert(substage, new_substage);

        self.workloads.insert(label, old_workload);
    }
}

//====================================================================

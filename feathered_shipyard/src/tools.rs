//====================================================================

//====================================================================

pub(crate) trait GetWorld {
    fn get_world(&self) -> &shipyard::World;
}

//--------------------------------------------------

#[allow(dead_code)]
pub trait WorldTools {
    fn and_run<B, S: shipyard::System<(), B>>(&self, system: S) -> &Self;

    fn and_run_with_data<Data, B, S: shipyard::System<(Data,), B>>(
        &self,
        system: S,
        data: Data,
    ) -> &Self;
}

#[allow(dead_code)]
pub trait UniqueTools {
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self;
    fn insert_default<U: shipyard::Unique + Send + Sync + Default>(&self) -> &Self {
        self.insert(U::default())
    }
}

//====================================================================

impl GetWorld for shipyard::World {
    #[inline]
    fn get_world(&self) -> &shipyard::World {
        &self
    }
}

impl<T: GetWorld> WorldTools for T {
    #[inline]
    fn and_run<B, S: shipyard::System<(), B>>(&self, system: S) -> &Self {
        self.get_world().run(system);
        self
    }

    #[inline]
    fn and_run_with_data<Data, B, S: shipyard::System<(Data,), B>>(
        &self,
        system: S,
        data: Data,
    ) -> &Self {
        self.get_world().run_with_data(system, data);
        self
    }
}

impl<T: GetWorld> UniqueTools for T {
    #[inline]
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self {
        self.get_world().add_unique(unique);
        self
    }
}

//====================================================================

impl UniqueTools for shipyard::AllStoragesView<'_> {
    #[inline]
    fn insert<U: shipyard::Unique + Send + Sync>(&self, unique: U) -> &Self {
        self.add_unique(unique);
        self
    }
}

//====================================================================

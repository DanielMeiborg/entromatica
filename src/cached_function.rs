use std::sync::Arc;

use hashbrown::HashMap;
use rayon::prelude::*;

#[derive(Clone)]
pub struct CachedFunction<I, O> {
    cache: HashMap<I, O>,
    function: Arc<dyn Fn(I) -> O + Send + Sync>,
}

impl<I, O> CachedFunction<I, O>
where
    I: Eq + std::hash::Hash + Clone + Send + Sync,
    O: Clone + Send + Sync,
{
    pub fn new(function: Arc<dyn Fn(I) -> O + Send + Sync>) -> Self {
        Self {
            cache: HashMap::new(),
            function,
        }
    }

    #[allow(dead_code)]
    pub fn call(&mut self, input: I) -> O {
        if let Some(output) = self.cache.get(&input) {
            output.clone()
        } else {
            let output = self.bypass(input.clone());
            self.cache.insert(input, output.clone());
            output
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn bypass(&self, input: I) -> O {
        (self.function)(input)
    }

    #[allow(dead_code)]
    pub fn call_many(&mut self, inputs: impl Iterator<Item = I>) -> Vec<O> {
        inputs.map(|input| self.call(input)).collect()
    }

    pub fn call_many_parallel(&mut self, inputs: impl IntoParallelIterator<Item = I>) -> Vec<O> {
        let pairs = inputs
            .into_par_iter()
            .map(|input| (input.clone(), self.bypass(input)))
            .collect::<Vec<(I, O)>>();
        pairs.iter().for_each(|(input, output)| {
            self.cache.insert(input.clone(), output.clone());
        });
        pairs.into_iter().map(|(_, output)| output).collect()
    }

    #[allow(dead_code)]
    pub fn function(&self) -> Arc<dyn Fn(I) -> O + Send + Sync> {
        self.function.clone()
    }
}

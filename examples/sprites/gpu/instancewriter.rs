use crate::gpu::rendering::Instance;

pub struct InstanceWriter<'a> {
    instances: &'a mut Vec<Instance>,
}


impl<'a> InstanceWriter<'a> {
    pub fn new(instances: &'a mut Vec<Instance>) -> Self {
        Self {
            instances,
        }
    }

    pub fn write(&mut self, instance: Instance) {
        self.instances.push(instance);
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub fn cast(&self) -> &[u8] {
        bytemuck::cast_slice(&self.instances)
    }
}

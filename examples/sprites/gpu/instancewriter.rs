use crate::gpu::rendering::Instance;

pub struct InstanceWriter {
    instances: Vec<Instance>,
}


impl InstanceWriter {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
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

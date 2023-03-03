#![feature(allocator_api)]
use metal::{ComputePipelineDescriptor, Device, DeviceRef, MTLResourceOptions, MTLResourceUsage};

use crate::alloc::PageAlignedAllocator;

pub mod alloc;

const LIB_DATA: &[u8] = include_bytes!("metal/memory.metallib");

fn main() {
    let device: &DeviceRef = &Device::system_default().expect("No device found");
    let lib = device.new_library_with_data(LIB_DATA).unwrap();
    let kernel = lib.get_function("assign", None).unwrap();

    let pipeline_state_descriptor = ComputePipelineDescriptor::new();
    pipeline_state_descriptor.set_compute_function(Some(&kernel));

    let pipeline = device
        .new_compute_pipeline_state_with_function(
            pipeline_state_descriptor.compute_function().unwrap(),
        )
        .unwrap();

    let mut data = [0_u32; 16].to_vec_in(PageAlignedAllocator);
    let data_size = (data.len() * core::mem::size_of::<u32>()) as u64;

    let ptr = data.as_mut_ptr();
    let buffer = device.new_buffer_with_bytes_no_copy( // for some reason this creates a buffer
                                                       // with a null ptr.
        ptr as *mut core::ffi::c_void,
        data_size,
        MTLResourceOptions::StorageModeShared,
        None,
    );

    let command_queue = device.new_command_queue();
    let command_buffer = command_queue.new_command_buffer();
    let compute_encoder = command_buffer.new_compute_command_encoder();
    compute_encoder.set_compute_pipeline_state(&pipeline);
    compute_encoder.set_buffer(0, Some(&buffer), 0); // code panics here, the first call which
                                                     // takes buffer as arg.

    compute_encoder.use_resource(&buffer, MTLResourceUsage::Write);

    let grid_size = metal::MTLSize::new(data.len() as u64, 1, 1);
    let threadgroup_size = metal::MTLSize::new(data.len() as u64, 1, 1);

    compute_encoder.dispatch_threads(grid_size, threadgroup_size);
    compute_encoder.end_encoding();

    command_buffer.commit();
    command_buffer.wait_until_completed();

    unsafe {
        println!("via contents(): {:?}", *(buffer.contents() as *mut [u32; 16]));
    }
    println!("rust vector: {:?}", data);
}
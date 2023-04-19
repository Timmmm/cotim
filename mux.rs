use std::sync::{Arc, Mutex};
use super::Instance;
pub struct Inputs {
    pub i_clk: SvBits<1>,
    pub i_rst: SvBits<1>,
    pub i_sel: SvBits<1>,
    pub i_a: SvBits<1>,
    pub i_b: SvBits<1>,
    pub i_double: SvBits<121>,
    pub i_u16_array: [SvBits<16>; 8],
}
pub struct Outputs {
    pub o_aorb: SvBits<1>,
    pub o_slice: SvBits<4>,
    pub o_double_slice: SvBits<65>,
    pub o_wide: [SvBits<128>; 2],
}
extern "C" fn mux_new(module_path: &str) -> *const Mutex<Instance> {
    match Instance::new(module_path) {
        Ok(instance) => Arc::into_raw(instance),
        Err(e) => {
            eprintln!("Error in mux_new: {e:?}");
            std::ptr::null()
        }
    }
}
#[repr(u8)]
enum ReturnCode {
    Success = 0,
    Failure = 1,
}
extern "C" fn mux_tick(
    instance: *const Mutex<Instance>,
    i_clk: *mut u32,
    i_rst: *mut u32,
    i_sel: *mut u32,
    i_a: *mut u32,
    i_b: *mut u32,
    i_double: *mut u32,
    i_u16_array___0: *mut u32,
    i_u16_array___1: *mut u32,
    i_u16_array___2: *mut u32,
    i_u16_array___3: *mut u32,
    i_u16_array___4: *mut u32,
    i_u16_array___5: *mut u32,
    i_u16_array___6: *mut u32,
    i_u16_array___7: *mut u32,
    o_aorb: *mut u32,
    o_slice: *mut u32,
    o_double_slice: *mut u32,
    o_wide___0: *mut u32,
    o_wide___1: *mut u32,
) -> ReturnCode {
    unsafe {
        let inputs = Inputs {
            i_clk: i_clk.into(),
            i_rst: i_rst.into(),
            i_sel: i_sel.into(),
            i_a: i_a.into(),
            i_b: i_b.into(),
            i_double: i_double.into(),
            i_u16_array: [
                i_u16_array___0.into(),
                i_u16_array___1.into(),
                i_u16_array___2.into(),
                i_u16_array___3.into(),
                i_u16_array___4.into(),
                i_u16_array___5.into(),
                i_u16_array___6.into(),
                i_u16_array___7.into(),
            ],
        };
        let outputs = Outputs {
            o_aorb: o_aorb.into(),
            o_slice: o_slice.into(),
            o_double_slice: o_double_slice.into(),
            o_wide: [o_wide___0.into(), o_wide___1.into()],
        };
        let mut guard = instance.as_ref().unwrap().lock().unwrap();
        match guard.tick(&inputs, &outputs) {
            Ok() => ReturnCode::Success,
            Err(e) => {
                eprintln!("Error in mux_tick: {e:?}");
                ReturnCode::Failure
            }
        }
    }
}
extern "C" fn mux_free(instance: *const Mutex<Instance>) {
    unsafe {
        Arc::from_raw(instance);
    }
}

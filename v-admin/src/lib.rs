// 控制器模块自动生成 / api modules are auto-generated
include!(concat!(env!("OUT_DIR"), "/auto_mod.rs"));

pub mod model {
    include!(concat!(env!("OUT_DIR"), "/model_tree.rs"));
    include!(concat!(env!("OUT_DIR"), "/model_auto.rs"));
}
pub mod errors;
// pub mod repo;
// pub mod service;

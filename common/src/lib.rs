#![deny(clippy::unwrap_used)] // 禁止使用 unwrap()
#![deny(clippy::expect_used)] // 禁止使用 expect()
#![deny(clippy::panic)] // 禁止显式 panic!
#![warn(clippy::todo)] // 提示遗留的 todo!()
#![warn(clippy::unimplemented)] // 提示遗留的 /*unimplemented!*/
#![warn(clippy::print_stdout)] // 禁止使用 println! 打印日志
#![warn(clippy::print_stderr)] // 禁止使用 eprintln! 打印日志
#![warn(clippy::unreachable)] // 提示遗留的 /*unreachable!*/
#![warn(clippy::unused_async)] // 提示不必要的 async
#![warn(clippy::unused_io_amount)] // 提示不必要的 io::Result
#![warn(clippy::unused_unit)] // 提示不必要的 unit 返回值
#![warn(clippy::dbg_macro)] // 提示 dbg! 调试遗留
#![warn(clippy::clone_on_ref_ptr)] // Arc/Rc 不要随便 clone
#![warn(clippy::large_enum_variant)] // 枚举成员体积太大
#![warn(clippy::needless_collect)] // 多余的 collect()

pub mod api;
pub mod dto;
mod filter;
pub mod po;
pub mod utils;
pub use filter::*;

pub const ACCESS_TOKEN: &str = "access_token";
pub const REFRESH_TOKEN: &str = "refresh_token";

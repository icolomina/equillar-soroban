#![no_std]

pub mod collateral;
pub mod contract;
pub mod emergency;
pub mod interface;
mod constants;
pub mod investment;
pub mod payments;
pub mod shared;
pub mod treasury;
pub mod validation;

pub use validation::Error;

#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
    ($($cond:expr, $err:expr),+) => {
        $(
            if !$cond {
                return Err($err);
            }
        )+
    };
}
#![no_std]

mod collateral;
mod constants;
mod emergency;
mod investment;
mod payments;
mod treasury;
mod validation;

pub mod contract;
pub mod shared;

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

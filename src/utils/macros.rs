

#[macro_export]
macro_rules! if_result_fail_continue {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(_) => {
                // warn!("An error: {}; skipped.", e);
                continue;
            }
        }
    };
}

#[macro_export]
macro_rules! if_option_fail_continue {
    ($res:expr) => {
        match $res {
            Some(val) => val,
            None => {
                continue;
            }
        }
    };
}




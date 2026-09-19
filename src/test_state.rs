use bc_utils_lg::prelude::*;

pub static INDICATIONS_STATE: LazyLock<MAP<&str, f64>> =
    LazyLock::new(|| MAP::from_iter([("rma_1", 2.253957492828369), ("sma_1", 2.2545588732817268)]));

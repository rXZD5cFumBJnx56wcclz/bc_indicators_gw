use criterion::{Criterion, criterion_group, criterion_main};

use bc_pack_indicators::PACK;
use bc_test_kit::prelude::*;
use bc_utils_lg::structs::settings::{SETTINGS_IND, SETTINGS_INDS, SETTINGS_USED_USIZE};
use bc_utils_lg::types::maps::MAP;

use bc_indicators_gw::gw::{Indicators, IndicatorsGateway};

fn indications_series_1(c: &mut Criterion) {
    let s = SETTINGS_INDS::from_iter([(
        "rsi_1".to_string(),
        SETTINGS_IND {
            key: "rsi".to_string(),
            kwargs_usize: MAP::from_iter([("window".to_string(), 2)]),
            kwargs_f64: MAP::default(),
            kwargs_string: MAP::default(),
            used_src: vec![SETTINGS_USED_USIZE {
                index: 1,
                sub_from_last_i: 0,
            }],
            used_ind: vec![],
            procedure_used: vec![],
        },
    )]);
    let indicators = Indicators::new(&s, &PACK(), &SRC_TRANSPOSE);
    let indicators_gw = IndicatorsGateway::new(&indicators, &s);
    c.bench_function("indications_series_1", |b| {
        b.iter(|| indicators_gw.indications_series(&SRC_TRANSPOSE))
    });
}

fn indications_series_2(c: &mut Criterion) {
    let s = SETTINGS_INDS::from_iter([
        (
            "avg_1".to_string(),
            SETTINGS_IND {
                key: "avg".to_string(),
                kwargs_usize: MAP::from_iter([]),
                kwargs_f64: MAP::default(),
                kwargs_string: MAP::default(),
                used_src: vec![
                    SETTINGS_USED_USIZE {
                        index: 1,
                        sub_from_last_i: 0,
                    },
                    SETTINGS_USED_USIZE {
                        index: 1,
                        sub_from_last_i: 1,
                    },
                    SETTINGS_USED_USIZE {
                        index: 2,
                        sub_from_last_i: 1,
                    },
                    SETTINGS_USED_USIZE {
                        index: 3,
                        sub_from_last_i: 1,
                    },
                ],
                used_ind: vec![],
                procedure_used: vec![],
            },
        ),
        (
            "ema_1".to_string(),
            SETTINGS_IND {
                key: "ema".to_string(),
                kwargs_usize: MAP::from_iter([("window".to_string(), 4)]),
                kwargs_f64: MAP::default(),
                kwargs_string: MAP::default(),
                used_src: vec![],
                used_ind: vec!["avg_1".to_string()],
                procedure_used: vec![],
            },
        ),
        (
            "trend_ma_1".to_string(),
            SETTINGS_IND {
                key: "trend_ma".to_string(),
                kwargs_usize: MAP::from_iter([]),
                kwargs_f64: MAP::default(),
                kwargs_string: MAP::default(),
                used_src: vec![],
                used_ind: vec!["ema_1".to_string()],
                procedure_used: vec![],
            },
        ),
    ]);
    let indicators = Indicators::new(&s, &PACK(), &SRC_TRANSPOSE);
    let indicators_gw = IndicatorsGateway::new(&indicators, &s);
    c.bench_function("indications_series_2", |b| {
        b.iter(|| indicators_gw.indications_series(&SRC_TRANSPOSE))
    });
}

criterion_group!(benches, indications_series_1, indications_series_2);
criterion_main!(benches);

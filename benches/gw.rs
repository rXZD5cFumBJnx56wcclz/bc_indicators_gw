use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use bc_packs::PACK_IND;
use bc_test_kit::prelude::*;

use bc_indicators_gw::gw::Indicators;

fn indications_series_1(c: &mut Criterion) {
    let mut indicators = Indicators::default();
    indicators.init(&SRC_TRANSPOSE, &INDICATIONS_RSI, &PACK_IND);
    c.bench_function("indications_series_1", |b| {
        b.iter(|| indicators.series(black_box(&SRC_TRANSPOSE), black_box(&INDICATIONS_RSI)))
    });
}

criterion_group!(benches, indications_series_1,);
criterion_main!(benches);

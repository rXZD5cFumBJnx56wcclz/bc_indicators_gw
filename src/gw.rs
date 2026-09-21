use bc_indicators::main_trait::Indicator;
use bc_utils_lg::structs::settings::{SETTINGS_IND, SETTINGS_INDS};
use bc_utils_lg::traits::w::{W, w_scan, w_src, w_sum};
use bc_utils_lg::types::maps::{MAP, MAP_LINK, PACK};

use bc_gw_utils::prelude::*;

#[derive(Default, Clone)]
pub struct Indicators<'a>(pub MAP<&'a str, Box<dyn Indicator>>);

impl W for Indicators<'_> {
    fn w(&self) -> usize {
        self.0.values().map(|v| v.w()).max().unwrap_or_default()
    }
}

impl<'a> Indicators<'a> {
    pub fn w_map_all(&self, s: &'a SETTINGS_INDS) -> MAP_LINK<&'a str, usize> {
        w_scan(
            self.0.iter(),
            s.iter(),
            |v| v.w(),
            |setting, init, k| {
                [
                    w_src(&setting.used_src),
                    w_sum(&setting.used_ind, &init),
                    init[k.as_str()],
                ]
            },
        )
    }
    pub fn w_all(&self, s: &SETTINGS_INDS) -> usize {
        self.w_map_all(s)
            .values()
            .copied()
            .max()
            .unwrap_or_default()
    }
}

impl<'a> Indicators<'a> {
    pub fn init_empty(
        &mut self,
        s: &'a SETTINGS_INDS,
        pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
    ) {
        *self = Indicators(
            s.iter()
                .map(|(indicator_name, settings_indicator)| {
                    (
                        indicator_name.as_str(),
                        pack[settings_indicator.key.as_str()](settings_indicator),
                    )
                })
                .collect(),
        );
    }
    pub fn init_bf(&self, buffer: &[Vec<f64>], s: &'a SETTINGS_INDS) {
        let mut map = MAP::default();
        for (k, setting) in s.iter() {
            let indicator = &self.0[k.as_str()];
            let mut src = SrcGw::default();
            src.push_vec(&buffer, &setting.used_src);
            src.push_map(&map, &setting.used_ind);
            src.all_check(&setting.procedure_used);
            indicator.init_bf(&src[..indicator.w()]);
            map.insert(k.as_str(), indicator.ind_vec(&src[indicator.w()..]));
            // This step is optional, but it improves numerical accuracy.
            //
            // A buffer obtained through incremental updates (`w + n` source values)
            // may differ slightly from a buffer initialized directly from the minimum
            // required window (`w` source values). The discrepancy is caused by the
            // accumulation of small numerical errors during updates. Reinitialization
            // eliminates this difference.
            indicator.init_bf(&src);
        }
    }

    pub fn init(
        &mut self,
        buffer: &[Vec<f64>],
        s: &'a SETTINGS_INDS,
        pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
    ) {
        self.init_empty(s, pack);
        self.init_bf(buffer, s);
    }
}

impl<'a> Indicators<'a> {
    pub fn series_key(
        &self,
        func: impl Fn(&mut SrcGwSeries, &SETTINGS_IND, &MAP<&str, f64>),
        s: &'a SETTINGS_INDS,
    ) -> MAP<&'a str, f64> {
        s.iter().fold(MAP::default(), |mut map, (k, setting)| {
            let key_uniq_str = k.as_str();
            let indicator = &self.0[key_uniq_str];
            let mut src = SrcGwSeries::default();
            func(&mut src, setting, &map);
            map.insert(key_uniq_str, indicator.ind(&src));
            map
        })
    }

    pub fn vec_key(
        &self,
        func: impl Fn(&mut SrcGw, &SETTINGS_IND, &MAP<&str, Vec<f64>>),
        s: &'a SETTINGS_INDS,
    ) -> MAP<&'a str, Vec<f64>> {
        s.iter().fold(MAP::default(), |mut map, (k, setting)| {
            let key_uniq_str = k.as_str();
            let indicator = &self.0[key_uniq_str];
            let mut src = SrcGw::default();
            func(&mut src, setting, &map);
            map.insert(key_uniq_str, indicator.ind_vec(&src));
            map
        })
    }
    pub fn series(&self, buffer: &[Vec<f64>], s: &'a SETTINGS_INDS) -> MAP<&'a str, f64> {
        self.series_key(
            |src, setting, map| {
                src.push_vec(buffer, &setting.used_src);
                src.push_map(&map, &setting.used_ind);
                src.all_check(&setting.procedure_used);
            },
            s,
        )
    }
    pub fn execute_bf(&self) {
        for ind in self.0.values() {
            ind.execute_bf();
        }
    }
    pub fn vec(&self, buffer: &[Vec<f64>], s: &'a SETTINGS_INDS) -> MAP<&'a str, Vec<f64>> {
        self.vec_key(
            |src, setting, map| {
                src.push_vec(buffer, &setting.used_src);
                src.push_map(&map, &setting.used_ind);
                src.all_check(&setting.procedure_used);
            },
            s,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bc_indicators::prelude::Indicator;
    use bc_indicators::rma::RMA;
    use bc_indicators::sma::SMA;
    use bc_packs::PACK_IND;
    use bc_test_kit::prelude::*;
    use bc_utils::other::transpose;
    use bc_utils_lg::test_state::prelude::*;
    use bc_utils_lg::traits::w::W;
    use bc_utils_lg::types::maps::MAP;

    #[test]
    fn new_empty_bf_res_1() {
        let mut res = Indicators::default();
        res.init_empty(&INDICATIONS, &PACK_IND);
        let res_1 = res.0.get("rma_1").unwrap().as_ref();
        let rma_test_1 = RMA::new(2);
        let rma_test_2 = (res_1 as &dyn Any).downcast_ref::<RMA>().unwrap();
        assert_eq_pr!(&rma_test_1, rma_test_2);
    }

    #[test]
    fn w_all_res_1() {
        let mut res = Indicators::default();
        res.init_empty(&INDICATIONS, &PACK_IND);
        assert_eq_pr!(res.w_all(&INDICATIONS), 24);
    }

    #[test]
    fn init_bf_res_1() {
        let src = transpose(SRC[..49].to_vec());
        let mut indicators = Indicators::default();
        indicators.init(&src, &INDICATIONS, &PACK_IND);
        let res_1 = indicators.series(&SRC_TRANSPOSE, &INDICATIONS);
        let rma = RMA::new(2);
        let mut src_test = SrcGw::default();
        src_test.push_vec(&src, &INDICATIONS["rma_1"].used_src);
        src_test.all_check(&INDICATIONS["rma_1"].procedure_used);
        rma.init_bf(&src_test);
        let rma_res = rma.ind(&[SRC[48][4]]);
        assert_eq_pr!(res_1["rma_1"], rma_res);
    }

    #[test]
    fn series_res_1() {
        let src = transpose(SRC[..49].to_vec());
        let mut indicators = Indicators::default();
        indicators.init(&src, &INDICATIONS, &PACK_IND);
        let res_1 = indicators.series(&SRC_TRANSPOSE, &INDICATIONS);
        let mut src_rma = SrcGw::default();
        src_rma.push_vec(&src, &INDICATIONS["rma_1"].used_src);
        src_rma.all_check(&INDICATIONS["rma_1"].procedure_used);
        let rma = RMA::new(2);
        rma.init_bf(&src_rma);
        let rma_res = rma.ind(&[SRC[48][4]]);
        let rma_sma = RMA::new(2);
        let sma = SMA::new(3);
        rma_sma.init_bf(&src_rma[..rma_sma.w()]);
        let mut src_test = SrcGw::default();
        src_test.push_vec(&src, &INDICATIONS["sma_1"].used_src);
        src_test.push_map(
            &MAP::from_iter([("rma_1", rma_sma.ind_vec(&src_rma[rma_sma.w()..]))]),
            &INDICATIONS["sma_1"].used_ind,
        );
        src_test.all_check(&INDICATIONS["sma_1"].procedure_used);
        sma.init_bf(&src_test);
        assert_eq_pr!(res_1["rma_1"], rma_res);
        assert_eq_pr!(res_1["sma_1"], sma.ind(&[rma_res]));
    }

    #[test]
    fn vec_res_1() {
        let mut indicators = Indicators::default();
        indicators.init_empty(&INDICATIONS, &PACK_IND);
        let (src_buffer, src_vec) = (
            transpose(SRC[..indicators.w_all(&INDICATIONS)].to_vec()),
            transpose(SRC[indicators.w_all(&INDICATIONS)..].to_vec()),
        );
        indicators.init_bf(&src_buffer, &INDICATIONS);
        let res = indicators.vec(&src_vec, &INDICATIONS);
        let rma = RMA::new(2);
        let mut src_rma = SrcGw::default();
        src_rma.push_vec(&src_buffer.clone(), &INDICATIONS["rma_1"].used_src);
        src_rma.all_check(&INDICATIONS["rma_1"].procedure_used);
        rma.init_bf(&src_rma);
        let sma = SMA::new(3);
        let rma_sma = RMA::new(2);
        let mut src_rma_sma = SrcGw::default();
        src_rma_sma.push_vec(&src_buffer.clone(), &INDICATIONS["rma_1"].used_src);
        src_rma_sma.all_check(&INDICATIONS["rma_1"].procedure_used);
        rma_sma.init_bf(&src_rma_sma[..rma_sma.w()]);
        let mut src_sma_ = SrcGw::default();
        src_sma_.push_vec(&src_buffer.clone(), &INDICATIONS["sma_1"].used_src);
        src_sma_.push_map(
            &MAP::from_iter([("rma_1", rma_sma.ind_vec(&src_rma_sma[rma.w()..]))]),
            &INDICATIONS["sma_1"].used_ind,
        );
        src_sma_.all_check(&INDICATIONS["sma_1"].procedure_used);
        sma.init_bf(&src_sma_);
        let mut src_rma_vec = SrcGw::default();
        src_rma_vec.push_vec(&src_vec, &INDICATIONS["rma_1"].used_src);
        src_rma_vec.all_check(&INDICATIONS["rma_1"].procedure_used);
        let map = MAP::from_iter([("rma_1", rma.ind_vec(&src_rma_vec))]);
        assert_eq_pr!(&res["rma_1"], &map["rma_1"],);
        let mut src_sma_vec = SrcGw::default();
        src_sma_vec.push_vec(&src_vec, &INDICATIONS["sma_1"].used_src);
        src_sma_vec.push_map(&map, &INDICATIONS["sma_1"].used_ind);
        src_sma_vec.all_check(&INDICATIONS["sma_1"].procedure_used);
        assert_eq_pr!(&res["sma_1"], &sma.ind_vec(&src_sma_vec));
    }
}

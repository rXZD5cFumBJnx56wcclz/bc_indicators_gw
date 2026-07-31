use bc_indicators::main_trait::Indicator;
use bc_utils::other::{procedure_used, transpose, vec_len_sync_set};
use bc_utils_lg::structs::settings::{SETTINGS_IND, SETTINGS_INDS};
use bc_utils_lg::traits::w::{W, w_scan, w_src, w_sum};
use bc_utils_lg::types::maps::{MAP, MAP_LINK, PACK};

pub fn get_src<'a>(
    buffer: &[Vec<f64>],
    indications: &MAP<&str, Vec<f64>>,
    s: &SETTINGS_IND,
) -> Vec<Vec<f64>> {
    let mut res = vec![];
    for used_src_el in &s.used_src {
        res.push({
            let sk = &buffer[used_src_el.index];
            sk[..sk.len() - used_src_el.sub_from_last_i].to_vec()
        });
    }
    for used_ind in &s.used_ind {
        res.push(indications[used_ind.as_str()].clone());
    }
    if !s.procedure_used.is_empty() {
        res = procedure_used(res, &s.procedure_used);
    }
    if !res.is_empty() {
        vec_len_sync_set(&mut res);
        return transpose(res);
    }
    Default::default()
}

pub fn get_src_series(
    buffer: &[Vec<f64>],
    indications: &MAP<&str, f64>,
    s: &SETTINGS_IND,
) -> Vec<f64> {
    let mut res = vec![];
    for us_el in &s.used_src {
        res.push({
            let sk = &buffer[us_el.index];
            sk[sk.len() - 1 - us_el.sub_from_last_i]
        });
    }
    for ui_el in &s.used_ind {
        res.push(indications[ui_el.as_str()]);
    }
    if !s.procedure_used.is_empty() {
        res = procedure_used(res, &s.procedure_used);
    }
    res
}

#[derive(Default, Clone)]
pub struct Indicators<'a>(pub MAP<&'a str, Box<dyn Indicator>>);

impl W for Indicators<'_> {
    fn w(&self) -> usize {
        self.0.values().map(|v| v.w()).max().unwrap()
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
        self.w_map_all(s).values().copied().max().unwrap()
    }
}

impl<'a> Indicators<'a> {
    pub fn new_empty_bf(
        s: &'a SETTINGS_INDS,
        pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
    ) -> Self {
        Indicators(
            s.iter()
                .map(|(indicator_name, settings_indicator)| {
                    (
                        indicator_name.as_str(),
                        pack[settings_indicator.key.as_str()](settings_indicator),
                    )
                })
                .collect(),
        )
    }
    pub fn init_bf(&self, buffer: &[Vec<f64>], s: &'a SETTINGS_INDS) {
        let mut map = MAP::default();
        for (k, setting) in s.iter() {
            let indicator = &self.0[k.as_str()];
            let src = get_src(buffer, &map, setting);
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
    pub fn new(
        buffer: &[Vec<f64>],
        s: &'a SETTINGS_INDS,
        pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
    ) -> Self {
        let bind = Indicators::new_empty_bf(s, pack);
        bind.init_bf(buffer, s);
        bind
    }
}

impl<'a> Indicators<'a> {
    pub fn series(&self, buffer_in: &[Vec<f64>], s: &'a SETTINGS_INDS) -> MAP<&'a str, f64> {
        s.iter().fold(MAP::default(), |mut map, setting| {
            let key_uniq_str = setting.0.as_str();
            let indicator = &self.0[key_uniq_str];
            map.insert(
                key_uniq_str,
                indicator.ind(&get_src_series(buffer_in, &map, &setting.1)),
            );
            map
        })
    }
    pub fn execute_bf(&self) {
        for ind in self.0.values() {
            ind.execute_bf();
        }
    }
    pub fn vec(&self, src: &[Vec<f64>], s: &'a SETTINGS_INDS) -> MAP<&'a str, Vec<f64>> {
        s.iter().fold(Default::default(), |mut init, (k, setting)| {
            let key_uniq_str = k.as_str();
            let indicator = &self.0[key_uniq_str];
            init.insert(
                key_uniq_str,
                indicator.ind_vec(&get_src(src, &init, setting)),
            );
            init
        })
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;

    use bc_indicators::prelude::Indicator;
    use bc_indicators::rma::RMA;
    use bc_indicators::sma::SMA;
    use bc_packs::PACK_IND;
    use bc_test_kit::prelude::*;

    use bc_utils::other::transpose;

    use bc_utils_lg::traits::w::W;
    use bc_utils_lg::types::maps::MAP;
    use pretty_assertions::assert_eq as assert_eq_pr;

    use super::*;

    #[test]
    fn new_empty_bf_res_1() {
        let res = Indicators::new_empty_bf(&INDICATIONS, &PACK_IND);
        let res_1 = res.0.get("rma_1").unwrap().as_ref();
        let rma_test_1 = RMA::new(2);
        let rma_test_2 = (res_1 as &dyn Any).downcast_ref::<RMA>().unwrap();
        assert_eq_pr!(&rma_test_1, rma_test_2);
    }

    #[test]
    fn w_all_res_1() {
        assert_eq_pr!(
            Indicators::new_empty_bf(&INDICATIONS, &PACK_IND).w_all(&INDICATIONS),
            24
        );
    }

    #[test]
    fn get_src_res_1() {
        assert_eq_pr!(
            get_src(&SRC_TRANSPOSE, &Default::default(), &INDICATIONS["rma_1"]),
            transpose(vec![CLOSE[..49].to_vec()]),
        )
    }

    #[test]
    fn get_src_series_res_1() {
        assert_eq_pr!(
            get_src_series(&SRC_TRANSPOSE, &Default::default(), &INDICATIONS["rma_1"]),
            vec![CLOSE[48]],
        )
    }

    #[test]
    fn init_bf_res_1() {
        let src = transpose(SRC[..49].to_vec());
        let indicators = Indicators::new(&src, &INDICATIONS, &PACK_IND);
        let res_1 = indicators.series(&SRC_TRANSPOSE, &INDICATIONS);
        let rma = RMA::new(2);
        rma.init_bf(&get_src(&src, &Default::default(), &INDICATIONS["rma_1"]));
        let rma_res = rma.ind(&[SRC[48][4]]);
        assert_eq_pr!(res_1["rma_1"], rma_res);
    }

    #[test]
    fn series_res_1() {
        let src = transpose(SRC[..49].to_vec());
        let indicators = Indicators::new(&src, &INDICATIONS, &PACK_IND);
        let res_1 = indicators.series(&SRC_TRANSPOSE, &INDICATIONS);
        let src_rma = get_src(&src, &Default::default(), &INDICATIONS["rma_1"]);
        let rma = RMA::new(2);
        rma.init_bf(&src_rma);
        let rma_res = rma.ind(&[SRC[48][4]]);
        let rma_sma = RMA::new(2);
        let sma = SMA::new(3);
        rma_sma.init_bf(&src_rma[..rma_sma.w()]);
        sma.init_bf(&get_src(
            &src,
            &MAP::from_iter([("rma_1", rma_sma.ind_vec(&src_rma[rma_sma.w()..]))]),
            &INDICATIONS["sma_1"],
        ));
        assert_eq_pr!(res_1["rma_1"], rma_res);
        assert_eq_pr!(res_1["sma_1"], sma.ind(&[rma_res]));
    }

    #[test]
    fn vec_res_1() {
        let indicators = Indicators::new_empty_bf(&INDICATIONS, &PACK_IND);
        let (src_buffer, src_vec) = (
            transpose(SRC[..indicators.w_all(&INDICATIONS)].to_vec()),
            transpose(SRC[indicators.w_all(&INDICATIONS)..].to_vec()),
        );
        indicators.init_bf(&src_buffer, &INDICATIONS);
        let res = indicators.vec(&src_vec, &INDICATIONS);
        let rma = RMA::new(2);
        rma.init_bf(&get_src(
            &src_buffer.clone(),
            &Default::default(),
            &INDICATIONS["rma_1"],
        ));
        let sma = SMA::new(3);
        let rma_sma = RMA::new(2);
        let src_rma_sma = get_src(
            &src_buffer.clone(),
            &Default::default(),
            &INDICATIONS["rma_1"],
        );
        rma_sma.init_bf(&src_rma_sma[..rma_sma.w()]);
        sma.init_bf(&get_src(
            &src_buffer,
            &MAP::from_iter([("rma_1", rma_sma.ind_vec(&src_rma_sma[rma.w()..]))]),
            &INDICATIONS["sma_1"],
        ));
        let map = MAP::from_iter([(
            "rma_1",
            rma.ind_vec(&get_src(
                &src_vec,
                &Default::default(),
                &INDICATIONS["rma_1"],
            )),
        )]);
        assert_eq_pr!(&res["rma_1"], &map["rma_1"],);
        assert_eq_pr!(
            &res["sma_1"],
            &sma.ind_vec(&get_src(&src_vec, &map, &INDICATIONS["sma_1"]))
        );
    }
}

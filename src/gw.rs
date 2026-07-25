use bc_indicators::main_trait::{BF_INDICATOR, Indicator};
use bc_utils::other::{procedure_used, transpose, vec_len_sync_set};
use bc_utils_lg::structs::settings::{SETTINGS_IND, SETTINGS_INDS};
use bc_utils_lg::types::maps::{MAP, PACK};

pub fn get_w_max(s: &SETTINGS_INDS, pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>) -> usize {
    get_map_from_pack(s, pack)
        .values()
        .map(|v| v.w())
        .max()
        .unwrap()
}

pub fn get_src<'a>(
    s: &SETTINGS_IND,
    settings: &SETTINGS_INDS,
    src: &[Vec<f64>],
    map_indicators: &MAP<&'a str, Box<dyn Indicator>>,
) -> Vec<Vec<f64>> {
    let mut res = vec![];
    for used_src_el in &s.used_src {
        res.push({
            let sk = &src[used_src_el.index];
            sk[..sk.len() - used_src_el.sub_from_last_i].to_vec()
        });
    }
    for used_ind_el in &s.used_ind {
        res.push(map_indicators[used_ind_el.as_str()].ind_vec(
            // recursive func
            &get_src(
                &settings[used_ind_el.as_str()],
                settings,
                src,
                map_indicators,
            ),
        ));
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
    s: &SETTINGS_IND,
    src: &[Vec<f64>],
    indications: &MAP<&str, f64>,
) -> Vec<f64> {
    let mut res = vec![];
    for us_el in &s.used_src {
        res.push({
            let sk = &src[us_el.index];
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

pub fn get_map_from_pack<'a>(
    settings: &'a SETTINGS_INDS,
    pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
) -> MAP<&'a str, Box<dyn Indicator>> {
    settings
        .iter()
        .map(|(indicator_name, settings_indicator)| {
            let indicator = pack[settings_indicator.key.as_str()](settings_indicator);
            (indicator_name.as_str(), indicator)
        })
        .collect()
}

pub fn get_map<'a>(
    settings: &'a SETTINGS_INDS,
    pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
    in_: &[Vec<f64>],
    map_indicators: &MAP<&'a str, Box<dyn Indicator>>,
) -> MAP<&'a str, (BF_INDICATOR<'a>, Box<dyn Indicator>)> {
    settings
        .iter()
        .map(|(indicator_name, settings_indicator)| {
            let indicator = pack[settings_indicator.key.as_str()](settings_indicator);
            (
                indicator_name.as_str(),
                (
                    indicator.bf(&get_src(
                        settings_indicator,
                        settings,
                        &in_.into_iter()
                            .map(|v| v[..v.len() - 1].to_vec())
                            .collect::<Vec<Vec<f64>>>(),
                        map_indicators,
                    )),
                    indicator,
                ),
            )
        })
        .collect()
}

#[derive(Default)]
pub struct Indicators<'a> {
    pub indicators_without_bf: MAP<&'a str, Box<dyn Indicator>>,
    pub indicators: MAP<&'a str, (BF_INDICATOR<'a>, Box<dyn Indicator>)>,
}

impl<'a> Indicators<'a> {
    pub fn new(
        settings: &'a SETTINGS_INDS,
        pack: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
        src_transpose: &[Vec<f64>],
    ) -> Self {
        let ind_without_bf = get_map_from_pack(settings, pack);
        Self {
            indicators: get_map(settings, pack, src_transpose, &ind_without_bf),
            indicators_without_bf: ind_without_bf,
        }
    }
    pub fn update_bf(
        &mut self,
        src_transpose: &[Vec<f64>],
        s: &'a SETTINGS_INDS,
        fa: &PACK<SETTINGS_IND, Box<dyn Indicator>>,
    ) {
        self.indicators = get_map(s, fa, src_transpose, &self.indicators_without_bf);
    }
}

#[derive(Default)]
pub struct IndicatorsGateway<'a> {
    pub indicators: *const Indicators<'a>,
    pub settings: *const SETTINGS_INDS,
}

impl<'a> IndicatorsGateway<'a> {
    pub fn new(indicators: *const Indicators<'a>, settings: &'a SETTINGS_INDS) -> Self {
        Self {
            indicators,
            settings,
        }
    }
}
impl<'a> IndicatorsGateway<'a> {
    pub fn indications_series(&self, buffer_in: &[Vec<f64>]) -> MAP<&'a str, f64> {
        unsafe { &*self.settings }
            .iter()
            .fold(MAP::default(), |mut map, setting| {
                let key_uniq_str = setting.0.as_str();
                let indicator = unsafe { &(&(*self.indicators).indicators)[key_uniq_str] };
                map.insert(
                    key_uniq_str,
                    indicator.1.ind_with_bf(
                        &get_src_series(&setting.1, buffer_in, &map),
                        &indicator.0,
                        0,
                    ),
                );
                map
            })
    }
    pub fn indications_vec(&self, src: &[Vec<f64>]) -> MAP<&'a str, Vec<f64>> {
        unsafe { &*self.settings }
            .iter()
            .map(|(k, setting)| {
                let key_uniq = k.as_str();
                let indicator = unsafe { &(&(*self.indicators).indicators)[key_uniq] };
                (
                    key_uniq,
                    indicator.1.ind_vec(&get_src(
                        setting,
                        unsafe { &*self.settings },
                        src,
                        unsafe { &(*self.indicators).indicators_without_bf },
                    )),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;

    use bc_indicators::prelude::Indicator;
    use bc_indicators::{rma::RMA, rsi::RSI};
    use bc_pack_indicators::PACK;
    use bc_test_kit::prelude::*;
    use bc_utils::nums::{nz_coll, round_f};
    use bc_utils::other::transpose;
    use bc_utils_lg::structs::settings::{SETTINGS_IND, SETTINGS_INDS, SETTINGS_USED_USIZE};
    use bc_utils_lg::types::maps::MAP;
    use pretty_assertions::assert_eq as assert_eq_pr;

    use super::*;

    #[test]
    fn indicators_from_settings_without_bf_res_1() {
        let settings = SETTINGS_INDS::from_iter([(
            "rsi_1".to_string(),
            SETTINGS_IND {
                key: "rsi".to_string(),
                kwargs_usize: MAP::from_iter([("window".to_string(), 10)]),
                kwargs_f64: MAP::default(),
                kwargs_string: MAP::default(),
                used_src: vec![],
                used_ind: vec![],
                procedure_used: vec![],
            },
        )]);
        let pack = PACK();
        let res = get_map_from_pack(&settings, &pack);
        let res_1 = res.get("rsi_1").unwrap().as_ref();
        let rsi_test_1 = RSI::new(10);
        let rsi_test_2 = (res_1 as &dyn Any).downcast_ref::<RSI>().unwrap();
        assert_eq_pr!(&rsi_test_1, rsi_test_2);
    }

    #[test]
    fn indication_res_1() {
        let settings = SETTINGS_INDS::from_iter([
            (
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
            ),
            (
                "rma_1".to_string(),
                SETTINGS_IND {
                    key: "rma".to_string(),
                    kwargs_usize: MAP::from_iter([("window".to_string(), 2)]),
                    kwargs_f64: MAP::default(),
                    kwargs_string: MAP::default(),
                    used_src: vec![],
                    used_ind: vec!["rsi_1".to_string()],
                    procedure_used: vec![],
                },
            ),
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
                            index: 4,
                            sub_from_last_i: 2,
                        },
                    ],
                    used_ind: vec!["rma_1".to_string()],
                    procedure_used: vec![],
                },
            ),
            (
                "repeat_1".to_string(),
                SETTINGS_IND {
                    key: "repeat".to_string(),
                    kwargs_f64: MAP::from_iter([("value".to_string(), 1.0)]),
                    ..Default::default()
                },
            ),
            (
                "repeat_2".to_string(),
                SETTINGS_IND {
                    key: "repeat".to_string(),
                    kwargs_f64: MAP::from_iter([("value".to_string(), 2.0)]),
                    ..Default::default()
                },
            ),
            (
                "minus_1".to_string(),
                SETTINGS_IND {
                    key: "minus".to_string(),
                    used_ind: vec!["repeat_1".to_string(), "repeat_2".to_string()],
                    procedure_used: vec![1, 0],
                    ..Default::default()
                },
            ),
        ]);
        let indicators = Indicators::new(&settings, &PACK(), &SRC_TRANSPOSE);
        let indicators_gw = IndicatorsGateway::new(&indicators, &settings);
        let res_1 = indicators_gw.indications_series(&SRC_TRANSPOSE);
        let res_2 = (RMA::new(2).ind_f(
            &RSI::new(2)
                .ind_vec(&OPEN.into_iter().map(|v| vec![v]).collect::<Vec<Vec<f64>>>())
                .into_iter()
                .map(|v| vec![v])
                .collect::<Vec<Vec<f64>>>(),
        ) + CLOSE[47]
            + OPEN_LAST)
            / 3.;
        assert_eq_pr!(round_f(res_1["avg_1"], &4,), round_f(res_2, &4,),);
        assert_eq_pr!(res_1["minus_1"], 1.0);
    }

    #[test]
    fn indications_vec_res_1() {
        let settings = SETTINGS_INDS::from_iter([(
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
        let indicators = Indicators::new(&settings, &PACK(), &SRC_TRANSPOSE);
        let indicators_gw = IndicatorsGateway::new(&indicators, &settings);
        let res_1 = indicators_gw.indications_vec(&SRC_TRANSPOSE)["rsi_1"].clone();
        let res_2 = RSI::new(2).ind_vec(&transpose(vec![OPEN.to_vec()]));
        assert_eq_pr!(
            nz_coll::<Vec<f64>, _, _>(&res_1, 0.0),
            nz_coll::<Vec<f64>, _, _>(&res_2, 0.0)
        );
    }
}

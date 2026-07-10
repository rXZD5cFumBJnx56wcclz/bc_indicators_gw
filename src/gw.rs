use bc_indicators::main_trait::{BF_INDICATOR, Indicator};
use bc_utils::other::{transpose, vec_len_sync_set};
use bc_utils_lg::structs::settings::{SETTINGS_IND, SETTINGS_INDS, SETTINGS_USED_STRING_USIZE};
use bc_utils_lg::types::maps::{FUNCS_EXTRACT_ARGS_TYPE, MAP};

pub fn get_w_max(
    s: &SETTINGS_INDS,
    funcs_extract_args: &MAP<&str, fn(&SETTINGS_IND) -> Box<dyn Indicator>>,
) -> usize {
    get_indicators_from_settings_without_bf(s, funcs_extract_args)
        .values()
        .map(|v| v.w())
        .max()
        .unwrap()
}

pub fn get_in_from_settings<'a>(
    used_ind: &Vec<String>,
    used_src: &Vec<SETTINGS_USED_STRING_USIZE>,
    procedure_used: &Vec<usize>,
    settings: &SETTINGS_INDS,
    src: &[Vec<f64>],
    map_indicators: &MAP<&'a str, Box<dyn Indicator>>,
) -> Vec<Vec<f64>> {
    let mut res = vec![];
    for used_src_el in used_src {
        res.push({
            let sk = &src[used_src_el.index];
            sk[..sk.len() - used_src_el.sub_from_last_i].to_vec()
        });
    }
    for used_ind_el in used_ind {
        res.push(map_indicators[used_ind_el.as_str()].ind_vec(
            // recursive func
            &get_in_from_settings(
                &settings[used_ind_el].used_ind,
                &settings[used_ind_el].used_src,
                &settings[used_ind_el].procedure_used,
                settings,
                src,
                map_indicators,
            ),
        ));
    }
    if !procedure_used.is_empty() {
        let mut bind = res
            .into_iter()
            .enumerate()
            .collect::<Vec<(usize, Vec<f64>)>>();
        res = procedure_used
            .iter()
            .map(|i| {
                bind.remove(bind.iter().enumerate().find(|v| v.1.0 == *i).unwrap().0)
                    .1
            })
            .collect();
    }
    if !res.is_empty() {
        vec_len_sync_set(&mut res);
        return transpose(res);
    }
    Default::default()
}

pub fn get_indicators_from_settings_without_bf<'a>(
    settings: &'a SETTINGS_INDS,
    funcs_extract_args: &MAP<&'a str, fn(&SETTINGS_IND) -> Box<dyn Indicator>>,
) -> MAP<&'a str, Box<dyn Indicator>> {
    settings
        .iter()
        .map(|(indicator_name, settings_indicator)| {
            let indicator = funcs_extract_args[settings_indicator.key.as_str()](settings_indicator);
            (indicator_name.as_str(), indicator)
        })
        .collect()
}

pub fn get_indicators_from_settings<'a>(
    settings: &'a SETTINGS_INDS,
    funcs_extract_args: &MAP<&'a str, fn(&SETTINGS_IND) -> Box<dyn Indicator>>,
    in_: &[Vec<f64>],
    map_indicators: &MAP<&'a str, Box<dyn Indicator>>,
) -> MAP<&'a str, (BF_INDICATOR<'a>, Box<dyn Indicator>)> {
    settings
        .iter()
        .map(|(indicator_name, settings_indicator)| {
            let indicator = funcs_extract_args[settings_indicator.key.as_str()](settings_indicator);
            (
                indicator_name.as_str(),
                (
                    indicator.bf(&get_in_from_settings(
                        &settings_indicator.used_ind,
                        &settings_indicator.used_src,
                        &settings_indicator.procedure_used,
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
        funcs_extract_args: &MAP<&'a str, fn(&SETTINGS_IND) -> Box<dyn Indicator>>,
        src_transpose: &[Vec<f64>],
    ) -> Self {
        let ind_without_bf = get_indicators_from_settings_without_bf(settings, funcs_extract_args);
        Self {
            indicators: get_indicators_from_settings(
                settings,
                funcs_extract_args,
                src_transpose,
                &ind_without_bf,
            ),
            indicators_without_bf: ind_without_bf,
        }
    }
    pub fn update_bf(
        &mut self,
        src_transpose: &[Vec<f64>],
        s: &'a SETTINGS_INDS,
        fa: &FUNCS_EXTRACT_ARGS_TYPE<SETTINGS_IND, Box<dyn Indicator>>,
    ) {
        self.indicators =
            get_indicators_from_settings(s, fa, src_transpose, &self.indicators_without_bf);
    }
}

#[derive(Default)]
pub struct IndicatorsGateway<'a> {
    pub indicators: *const Indicators<'a>,
    pub settings: *const SETTINGS_INDS,
}

impl<'a> IndicatorsGateway<'a> {
    pub fn new(
        indicators: *const Indicators<'a>,
        settings: &'a SETTINGS_INDS,
    ) -> Self {
        Self { indicators, settings }
    }
    pub fn indications_series(
        &self,
        buffer_in: &[Vec<f64>],
    ) -> MAP<&'a str, f64> {
        unsafe { &*self.settings }
            .iter()
            .fold(MAP::default(), |mut map, setting| {
                let key_uniq_str = setting.0.as_str();
                let mut src_arg = vec![];
                for us_el in &setting.1.used_src {
                    src_arg.push({
                        let sk = &buffer_in[us_el.index];
                        sk[sk.len() - 1 - us_el.sub_from_last_i]
                    });
                }
                for ui_el in &setting.1.used_ind {
                    src_arg.push(map[ui_el.as_str()]);
                }
                if setting.1.procedure_used.len() != 0 {
                    src_arg = setting
                        .1
                        .procedure_used
                        .iter()
                        .map(|i| src_arg[*i])
                        .collect();
                }
                let indicator = unsafe { &(&(*self.indicators).indicators)[key_uniq_str] };
                map.insert(
                    key_uniq_str,
                    indicator.1.ind_with_bf(src_arg.as_slice(), &indicator.0, 0),
                );
                map
            })
    }
    pub fn indications_vec(
        &self,
        src: &[Vec<f64>],
    ) -> MAP<&'a str, Vec<f64>> {
        unsafe { &*self.settings }
            .iter()
            .map(|(k, setting)| {
                let key_uniq = k.as_str();
                let indicator = unsafe { &(&(*self.indicators).indicators)[key_uniq] };
                (
                    key_uniq,
                    indicator.1.ind_vec(&get_in_from_settings(
                        &setting.used_ind,
                        &setting.used_src,
                        &setting.procedure_used,
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
    use bc_pack_indicators::FUNCS_EXTRACT_ARGS;
    use bc_utils::nums::{nz_coll, round_f};
    use bc_utils::other::transpose;
    use bc_utils_lg::statics::prices::{CLOSE, OPEN, OPEN_LAST, SRC_TRANSPOSE};
    use bc_utils_lg::structs::settings::{SETTINGS_IND, SETTINGS_INDS, SETTINGS_USED_STRING_USIZE};
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
        let funcs_extract_args = FUNCS_EXTRACT_ARGS();
        let res = get_indicators_from_settings_without_bf(&settings, &funcs_extract_args);
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
                    used_src: vec![SETTINGS_USED_STRING_USIZE { index: 1, sub_from_last_i: 0 }],
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
                        SETTINGS_USED_STRING_USIZE { index: 1, sub_from_last_i: 0 },
                        SETTINGS_USED_STRING_USIZE { index: 4, sub_from_last_i: 2 },
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
        let indicators = Indicators::new(&settings, &FUNCS_EXTRACT_ARGS(), &SRC_TRANSPOSE);
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
                used_src: vec![SETTINGS_USED_STRING_USIZE { index: 1, sub_from_last_i: 0 }],
                used_ind: vec![],
                procedure_used: vec![],
            },
        )]);
        let indicators = Indicators::new(&settings, &FUNCS_EXTRACT_ARGS(), &SRC_TRANSPOSE);
        let indicators_gw = IndicatorsGateway::new(&indicators, &settings);
        let res_1 = indicators_gw.indications_vec(&SRC_TRANSPOSE)["rsi_1"].clone();
        let res_2 = RSI::new(2).ind_vec(&transpose(vec![OPEN.to_vec()]));
        assert_eq_pr!(
            nz_coll::<Vec<f64>, _, _>(&res_1, 0.0),
            nz_coll::<Vec<f64>, _, _>(&res_2, 0.0)
        );
    }
}

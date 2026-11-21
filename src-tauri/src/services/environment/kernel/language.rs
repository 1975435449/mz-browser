//! 语言自动检测模块

use super::types::{ProxyConfig, TIMEZONE_DETECTION_TIMEOUT_SECS};

/// 检测语言
///
/// # 逻辑
/// - 有代理配置：仅通过该代理检测，失败不回退直连
/// - 无代理配置：使用系统代理（reqwest 自动检测）
///
/// # 返回
/// - `Some(language)`: 检测成功
/// - `None`: 所有检测都失败
pub async fn detect_language(proxy: Option<&ProxyConfig>) -> Option<String> {
    if let Some(proxy_cfg) = proxy {
        detect_with_proxy(proxy_cfg).await
    } else {
        detect_with_system_proxy().await
    }
}

async fn detect_with_proxy(proxy_cfg: &ProxyConfig) -> Option<String> {
    let infra_proxy_cfg = proxy_cfg.to_infrastructure_proxy_config();
    let proxy_client = match crate::infrastructure::proxy::client::build_proxy_client(
        &infra_proxy_cfg,
        Some(TIMEZONE_DETECTION_TIMEOUT_SECS),
    ) {
        Ok(client) => client,
        Err(_) => {
            return None;
        }
    };

    let proxy_result = crate::infrastructure::proxy::detector::detect_ip_with_timeout(
        proxy_client,
        TIMEZONE_DETECTION_TIMEOUT_SECS,
    )
    .await;

    if proxy_result.success {
        let ip_info = proxy_result.ip_info.as_ref()?;
        infer_language_from_country_code(ip_info.country_code.as_str())
    } else {
        None
    }
}

async fn detect_with_system_proxy() -> Option<String> {
    let result = crate::infrastructure::proxy::detector::detect_with_system_proxy(
        TIMEZONE_DETECTION_TIMEOUT_SECS,
    )
    .await;

    if result.success {
        let ip_info = result.ip_info.as_ref()?;
        infer_language_from_country_code(ip_info.country_code.as_str())
    } else {
        None
    }
}

/// 国家/地区（ISO 3166-1 两位代码）→ 当地浏览器最常见的语言。
///
/// 大多按 CLDR 的常用语言；有几处按实际上网习惯取：哈萨克斯坦、吉尔吉斯斯坦、白俄罗斯多用俄语，
/// 巴基斯坦、菲律宾、马耳他、卢旺达多用英语，摩洛哥、突尼斯、卢森堡多用法语。
/// 少了某个国家时“语言跟随 IP”会拒绝启动（不能乱猜），所以这里要覆盖全部国家和地区。
const COUNTRY_LANGUAGES: &[(&str, &str)] = &[
    // 东亚
    ("CN", "zh-CN"), ("TW", "zh-TW"), ("HK", "zh-HK"), ("MO", "zh-MO"),
    ("JP", "ja-JP"), ("KR", "ko-KR"), ("KP", "ko-KP"), ("MN", "mn-MN"),
    // 东南亚
    ("SG", "en-SG"), ("MY", "ms-MY"), ("BN", "ms-BN"), ("ID", "id-ID"), ("TH", "th-TH"),
    ("VN", "vi-VN"), ("PH", "en-PH"), ("KH", "km-KH"), ("LA", "lo-LA"), ("MM", "my-MM"),
    ("TL", "pt-TL"),
    // 南亚、中亚
    ("IN", "en-IN"), ("PK", "en-PK"), ("BD", "bn-BD"), ("LK", "si-LK"), ("NP", "ne-NP"),
    ("BT", "dz-BT"), ("MV", "dv-MV"), ("AF", "fa-AF"), ("KZ", "ru-KZ"), ("KG", "ru-KG"),
    ("UZ", "uz-UZ"), ("TJ", "tg-TJ"), ("TM", "tk-TM"),
    // 中东
    ("TR", "tr-TR"), ("IR", "fa-IR"), ("IL", "he-IL"), ("SA", "ar-SA"), ("AE", "ar-AE"),
    ("QA", "ar-QA"), ("KW", "ar-KW"), ("BH", "ar-BH"), ("OM", "ar-OM"), ("YE", "ar-YE"),
    ("IQ", "ar-IQ"), ("JO", "ar-JO"), ("LB", "ar-LB"), ("SY", "ar-SY"), ("PS", "ar-PS"),
    ("AM", "hy-AM"), ("AZ", "az-AZ"), ("GE", "ka-GE"), ("CY", "el-CY"),
    // 欧洲
    ("GB", "en-GB"), ("IE", "en-IE"), ("FR", "fr-FR"), ("DE", "de-DE"), ("AT", "de-AT"),
    ("CH", "de-CH"), ("LI", "de-LI"), ("BE", "nl-BE"), ("NL", "nl-NL"), ("LU", "fr-LU"),
    ("MC", "fr-MC"), ("ES", "es-ES"), ("AD", "ca-AD"), ("PT", "pt-PT"), ("IT", "it-IT"),
    ("SM", "it-SM"), ("VA", "it-VA"), ("MT", "en-MT"), ("GR", "el-GR"), ("DK", "da-DK"),
    ("NO", "nb-NO"), ("SE", "sv-SE"), ("FI", "fi-FI"), ("IS", "is-IS"), ("FO", "fo-FO"),
    ("EE", "et-EE"), ("LV", "lv-LV"), ("LT", "lt-LT"), ("PL", "pl-PL"), ("CZ", "cs-CZ"),
    ("SK", "sk-SK"), ("HU", "hu-HU"), ("SI", "sl-SI"), ("HR", "hr-HR"), ("BA", "bs-BA"),
    ("RS", "sr-RS"), ("ME", "sr-ME"), ("XK", "sq-XK"), ("AL", "sq-AL"), ("MK", "mk-MK"),
    ("BG", "bg-BG"), ("RO", "ro-RO"), ("MD", "ro-MD"), ("UA", "uk-UA"), ("BY", "ru-BY"),
    ("RU", "ru-RU"), ("GI", "en-GI"), ("IM", "en-IM"), ("JE", "en-JE"), ("GG", "en-GG"),
    ("AX", "sv-AX"), ("SJ", "nb-SJ"),
    // 北美
    ("US", "en-US"), ("CA", "en-CA"), ("MX", "es-MX"), ("GL", "da-GL"), ("BM", "en-BM"),
    ("PM", "fr-PM"),
    // 中美洲、加勒比
    ("GT", "es-GT"), ("BZ", "en-BZ"), ("SV", "es-SV"), ("HN", "es-HN"), ("NI", "es-NI"),
    ("CR", "es-CR"), ("PA", "es-PA"), ("CU", "es-CU"), ("DO", "es-DO"), ("PR", "es-PR"),
    ("HT", "fr-HT"), ("JM", "en-JM"), ("TT", "en-TT"), ("BS", "en-BS"), ("BB", "en-BB"),
    ("AG", "en-AG"), ("DM", "en-DM"), ("GD", "en-GD"), ("KN", "en-KN"), ("LC", "en-LC"),
    ("VC", "en-VC"), ("AI", "en-AI"), ("MS", "en-MS"), ("KY", "en-KY"), ("TC", "en-TC"),
    ("VG", "en-VG"), ("VI", "en-VI"), ("AW", "nl-AW"), ("CW", "nl-CW"), ("SX", "nl-SX"),
    ("BQ", "nl-BQ"), ("GP", "fr-GP"), ("MQ", "fr-MQ"), ("BL", "fr-BL"), ("MF", "fr-MF"),
    // 南美
    ("BR", "pt-BR"), ("AR", "es-AR"), ("CL", "es-CL"), ("CO", "es-CO"), ("PE", "es-PE"),
    ("VE", "es-VE"), ("EC", "es-EC"), ("BO", "es-BO"), ("PY", "es-PY"), ("UY", "es-UY"),
    ("GY", "en-GY"), ("SR", "nl-SR"), ("GF", "fr-GF"), ("FK", "en-FK"),
    // 北非
    ("EG", "ar-EG"), ("LY", "ar-LY"), ("TN", "fr-TN"), ("DZ", "ar-DZ"), ("MA", "fr-MA"),
    ("EH", "ar-EH"), ("SD", "ar-SD"), ("MR", "ar-MR"),
    // 撒哈拉以南非洲
    ("NG", "en-NG"), ("GH", "en-GH"), ("KE", "en-KE"), ("TZ", "sw-TZ"), ("UG", "en-UG"),
    ("RW", "en-RW"), ("BI", "fr-BI"), ("ET", "am-ET"), ("ER", "ti-ER"), ("DJ", "fr-DJ"),
    ("SO", "so-SO"), ("SS", "en-SS"), ("ZA", "en-ZA"), ("NA", "en-NA"), ("BW", "en-BW"),
    ("ZW", "en-ZW"), ("ZM", "en-ZM"), ("MW", "en-MW"), ("MZ", "pt-MZ"), ("AO", "pt-AO"),
    ("LS", "en-LS"), ("SZ", "en-SZ"), ("MG", "fr-MG"), ("MU", "en-MU"), ("SC", "en-SC"),
    ("KM", "fr-KM"), ("RE", "fr-RE"), ("YT", "fr-YT"), ("SN", "fr-SN"), ("GM", "en-GM"),
    ("GN", "fr-GN"), ("GW", "pt-GW"), ("SL", "en-SL"), ("LR", "en-LR"), ("CI", "fr-CI"),
    ("BF", "fr-BF"), ("ML", "fr-ML"), ("NE", "fr-NE"), ("TG", "fr-TG"), ("BJ", "fr-BJ"),
    ("CM", "fr-CM"), ("TD", "fr-TD"), ("CF", "fr-CF"), ("GA", "fr-GA"), ("CG", "fr-CG"),
    ("CD", "fr-CD"), ("GQ", "es-GQ"), ("ST", "pt-ST"), ("CV", "pt-CV"), ("SH", "en-SH"),
    // 大洋洲
    ("AU", "en-AU"), ("NZ", "en-NZ"), ("FJ", "en-FJ"), ("PG", "en-PG"), ("SB", "en-SB"),
    ("VU", "en-VU"), ("NC", "fr-NC"), ("PF", "fr-PF"), ("WF", "fr-WF"), ("WS", "en-WS"),
    ("TO", "en-TO"), ("KI", "en-KI"), ("TV", "en-TV"), ("NR", "en-NR"), ("FM", "en-FM"),
    ("MH", "en-MH"), ("PW", "en-PW"), ("GU", "en-GU"), ("MP", "en-MP"), ("AS", "en-AS"),
    ("CK", "en-CK"), ("NU", "en-NU"), ("NF", "en-NF"), ("CX", "en-CX"), ("CC", "en-CC"),
    ("PN", "en-PN"), ("TK", "en-TK"), ("IO", "en-IO"), ("UM", "en-UM"),
];

pub(super) fn infer_language_from_country_code(country_code: &str) -> Option<String> {
    let normalized = country_code.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return None;
    }
    COUNTRY_LANGUAGES
        .iter()
        .find(|(code, _)| *code == normalized)
        .map(|(_, language)| (*language).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn turkey_and_other_previously_missing_countries_resolve() {
        // issue #5：土耳其 IP 以前推断不出语言，环境直接拒绝启动
        for (code, language) in [
            ("TR", "tr-TR"), ("NL", "nl-NL"), ("PL", "pl-PL"), ("VN", "vi-VN"), ("TH", "th-TH"),
            ("ID", "id-ID"), ("UA", "uk-UA"), ("AE", "ar-AE"), ("IL", "he-IL"), ("ZA", "en-ZA"),
            ("AR", "es-AR"), ("SE", "sv-SE"), ("KZ", "ru-KZ"), ("PH", "en-PH"),
        ] {
            assert_eq!(infer_language_from_country_code(code).as_deref(), Some(language), "{code}");
        }
    }

    #[test]
    fn the_original_countries_keep_their_languages() {
        for (code, language) in [
            ("CN", "zh-CN"), ("TW", "zh-TW"), ("HK", "zh-HK"), ("US", "en-US"), ("GB", "en-GB"),
            ("CA", "en-CA"), ("AU", "en-AU"), ("JP", "ja-JP"), ("KR", "ko-KR"), ("FR", "fr-FR"),
            ("DE", "de-DE"), ("ES", "es-ES"), ("IT", "it-IT"), ("RU", "ru-RU"), ("BR", "pt-BR"),
            ("MX", "es-MX"), ("IN", "en-IN"), ("SG", "en-SG"),
        ] {
            assert_eq!(infer_language_from_country_code(code).as_deref(), Some(language), "{code}");
        }
    }

    #[test]
    fn codes_are_case_insensitive_and_unknown_ones_are_refused() {
        assert_eq!(infer_language_from_country_code(" tr ").as_deref(), Some("tr-TR"));
        assert_eq!(infer_language_from_country_code(""), None);
        assert_eq!(infer_language_from_country_code("ZZ"), None);
    }

    #[test]
    fn the_table_is_well_formed_and_covers_almost_every_country() {
        let mut seen = HashSet::new();
        for (code, language) in COUNTRY_LANGUAGES {
            assert!(code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase()), "{code}");
            assert!(seen.insert(*code), "duplicate {code}");
            // 语言-地区，地区就是这个国家本身，Accept-Language 和 navigator.language 才对得上
            let (lang, region) = language.split_once('-').expect(language);
            assert!((2..=3).contains(&lang.len()) && lang.chars().all(|c| c.is_ascii_lowercase()), "{language}");
            assert_eq!(region, *code, "{language} should end with {code}");
        }
        assert!(COUNTRY_LANGUAGES.len() >= 230, "only {} countries", COUNTRY_LANGUAGES.len());
    }
}

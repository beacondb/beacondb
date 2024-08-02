// if this returns None this AP must not be recorded
pub fn normalize_ssid(ssid: Option<&str>) -> Option<&str> {
    if let Some(ssid) = ssid {
        let ssid = ssid.trim().trim_matches('\0');
        if !ssid.is_empty() && !ssid.contains("_nomap") && !ssid.contains("_optout") {
            return Some(ssid);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssid() {
        // whitespace
        assert_eq!(normalize_ssid(Some("testing!")), Some("testing!"));
        assert_eq!(normalize_ssid(Some("  testing!  ")), Some("testing!"));
        assert_eq!(normalize_ssid(Some("testing  !")), Some("testing  !"));
        assert_eq!(normalize_ssid(Some("  testing  !  ")), Some("testing  !"));

        // null
        assert_eq!(normalize_ssid(None), None);
        assert_eq!(
            normalize_ssid(Some("\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0")),
            None
        );

        // opt out
        assert_eq!(normalize_ssid(Some("wifi")), Some("wifi"));
        assert_eq!(normalize_ssid(Some("wifi_nomap")), None);
        assert_eq!(normalize_ssid(Some("wifi_optout")), None);
        assert_eq!(normalize_ssid(Some("wifi_optout_nomap")), None);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct VersionInfo {
    pub name: &'static str,
    pub data_version: i32,
}

pub const DV_26_1: i32 = 4786;

pub const VERSIONS: &[VersionInfo] = &[
    VersionInfo {
        name: "1.18.2",
        data_version: 2975,
    },
    VersionInfo {
        name: "1.19.4",
        data_version: 3337,
    },
    VersionInfo {
        name: "1.20.1",
        data_version: 3465,
    },
    VersionInfo {
        name: "1.20.4",
        data_version: 3700,
    },
    VersionInfo {
        name: "1.20.6",
        data_version: 3839,
    },
    VersionInfo {
        name: "1.21",
        data_version: 3953,
    },
    VersionInfo {
        name: "1.21.1",
        data_version: 3955,
    },
    VersionInfo {
        name: "1.21.3",
        data_version: 4082,
    },
    VersionInfo {
        name: "1.21.4",
        data_version: 4189,
    },
    VersionInfo {
        name: "1.21.5",
        data_version: 4325,
    },
    VersionInfo {
        name: "1.21.6",
        data_version: 4435,
    },
    VersionInfo {
        name: "1.21.7",
        data_version: 4438,
    },
    VersionInfo {
        name: "1.21.8",
        data_version: 4440,
    },
    VersionInfo {
        name: "1.21.9",
        data_version: 4554,
    },
    VersionInfo {
        name: "1.21.10",
        data_version: 4556,
    },
    VersionInfo {
        name: "1.21.11",
        data_version: 4671,
    },
    VersionInfo {
        name: "26.1",
        data_version: 4786,
    },
    VersionInfo {
        name: "26.1.1",
        data_version: 4788,
    },
    VersionInfo {
        name: "26.1.2",
        data_version: 4790,
    },
    VersionInfo {
        name: "26.2",
        data_version: 4903,
    },
];

pub fn find(name: &str) -> Option<&'static VersionInfo> {
    VERSIONS.iter().find(|v| v.name == name)
}

pub fn from_data_version(dv: i32) -> Option<&'static VersionInfo> {
    VERSIONS.iter().find(|v| v.data_version == dv)
}

pub fn is_supported_target(v: &VersionInfo) -> bool {
    v.data_version < DV_26_1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_lookup() {
        assert_eq!(find("1.21.11").unwrap().data_version, 4671);
        assert_eq!(from_data_version(4786).unwrap().name, "26.1");
        assert!(find("1.13").is_none());
    }

    #[test]
    fn supported_target_check() {
        assert!(is_supported_target(find("1.21.11").unwrap()));
        assert!(is_supported_target(find("1.18.2").unwrap()));
        assert!(!is_supported_target(find("26.1").unwrap()));
    }
}

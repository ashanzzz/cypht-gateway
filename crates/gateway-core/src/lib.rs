use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BuildInfo {
    pub version: &'static str,
    pub git_sha: &'static str,
    pub git_short_sha: &'static str,
    pub git_tag: Option<&'static str>,
    pub git_dirty: bool,
    pub build_time: Option<&'static str>,
    pub api_version: &'static str,
}

impl BuildInfo {
    pub fn current() -> Self {
        let tag = env!("GATEWAY_GIT_TAG");
        let build_time = env!("GATEWAY_BUILD_TIME");

        Self {
            version: env!("CARGO_PKG_VERSION"),
            git_sha: env!("GATEWAY_GIT_SHA"),
            git_short_sha: env!("GATEWAY_GIT_SHORT_SHA"),
            git_tag: (!tag.is_empty()).then_some(tag),
            git_dirty: env!("GATEWAY_GIT_DIRTY") == "true",
            build_time: (build_time != "unknown").then_some(build_time),
            api_version: "v1",
        }
    }
}

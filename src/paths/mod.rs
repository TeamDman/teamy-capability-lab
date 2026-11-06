mod app_home;
mod cache;

pub use app_home::*;
pub use cache::*;

pub const APP_HOME_ENV_VAR: &str = "TEAMY_CAPABILITY_LAB_HOME";
pub const APP_HOME_DIR_NAME: &str = "teamy-capability-lab";

pub const APP_CACHE_ENV_VAR: &str = "TEAMY_CAPABILITY_LAB_CACHE";
pub const APP_CACHE_DIR_NAME: &str = "teamy-capability-lab";

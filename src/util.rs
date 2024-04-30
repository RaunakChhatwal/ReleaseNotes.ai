use std::error::Error;

use serde::{Deserialize, Serialize};

pub type DynResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub fn serialize_ok<T: Serialize>(obj: &T) -> String {
    return serde_json::to_string(&Ok::<&T, String>(obj)).unwrap();
}

pub fn serialize_err(error: Box<dyn Error + Send + Sync>) -> String {
    return serde_json::to_string(&Err::<(), String>(error.to_string())).unwrap();
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum TargetAudience {
    NonTechnical,
    ProjectManager,
    Technical    
}

impl Default for TargetAudience {
    fn default() -> Self {
        return TargetAudience::ProjectManager;
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Ticket {
    pub summary: String,
    pub description: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Arguments {
    pub repo_link: String,
    pub product_name: String,
    pub release_tag: String,
    pub prev_release_tag: String,
    pub release_date: chrono::NaiveDate,
    pub target_audience: TargetAudience,
    pub tickets: Vec<Ticket>
}
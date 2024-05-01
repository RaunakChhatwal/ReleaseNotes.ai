use serde::{Deserialize, Serialize};

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

impl Arguments {
    pub fn any_field_empty(&self) -> bool {
        let Arguments { repo_link, product_name, release_tag, prev_release_tag, release_date: _, target_audience: _, tickets } = self;

        if tickets.is_empty() {
            return true;
        }
    
        // iterate through all string fields
        for field in vec![repo_link, product_name, release_tag, prev_release_tag]
            .into_iter()
            .chain(tickets
                .iter()
                .flat_map(|Ticket {summary, description}|
                    vec![summary, description])
        ) {
            if field.is_empty() {
                return true;
            }
        }

        return false;
    }
}
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct MatterHandoff {
    pub matter_id: String,
    pub intake_received: bool,
    pub signed_document_delivered: bool,
    pub deadline_hours: u32,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FollowUp {
    RequestIntake,
    DeliverSignedDocument,
    EscalateDeadline,
    AwaitDeadline,
}

pub fn follow_up(matter: &MatterHandoff) -> FollowUp {
    if !matter.intake_received {
        FollowUp::RequestIntake
    } else if !matter.signed_document_delivered {
        FollowUp::DeliverSignedDocument
    } else if matter.deadline_hours <= 24 {
        FollowUp::EscalateDeadline
    } else {
        FollowUp::AwaitDeadline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_delivery_with_urgent_deadline_escalates() {
        let matter = MatterHandoff {
            matter_id: "case-42".into(),
            intake_received: true,
            signed_document_delivered: true,
            deadline_hours: 12,
        };
        assert_eq!(follow_up(&matter), FollowUp::EscalateDeadline);
        assert_eq!(follow_up(&MatterHandoff { signed_document_delivered: false, ..matter }), FollowUp::DeliverSignedDocument);
    }
}

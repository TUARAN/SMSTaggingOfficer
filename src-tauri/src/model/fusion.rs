use crate::model::schema::LabelOutput;

#[derive(Debug, Clone)]
pub struct FusionInput {
  pub rule: Option<LabelOutput>,
  pub model: Option<LabelOutput>,
  pub rule_strong_hit: bool,
}

pub fn fuse(input: FusionInput) -> LabelOutput {
  match (input.rule, input.model) {
    (Some(rule), None) => rule,
    (None, Some(model)) => model,
    (Some(rule), Some(model)) => {
      let rule_industry = rule.industry.clone();
      let rule_type = rule.sms_type.clone();
      let model_industry = model.industry.clone();
      let model_type = model.sms_type.clone();

      // If strong rule hit, prefer rule; if conflict, mark needs_review.
      let mut out = if input.rule_strong_hit {
        rule
      } else if model.confidence >= rule.confidence {
        model
      } else {
        rule
      };

      // Conflict detection
      let conflict = (rule_industry != model_industry) || (rule_type != model_type);
      if conflict {
        out.needs_review = true;
        out.confidence = (out.confidence * 0.85).min(0.85);
        out.reasons.push("fusion_conflict".to_string());
      }
      out
    }
    (None, None) => LabelOutput {
      industry: "其他".to_string(),
      sms_type: "其他".to_string(),
      entities: Default::default(),
      confidence: 0.4,
      needs_review: true,
      reasons: vec!["no_rule_no_model".to_string()],
      signals: Default::default(),
      rules_version: crate::model::schema::RULES_VERSION.to_string(),
      model_version: "n/a".to_string(),
      schema_version: crate::model::schema::SCHEMA_VERSION.to_string(),
    },
  }
}

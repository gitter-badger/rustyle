use crate::core::node::{DeclarationNode, Node};

pub struct RulesetNode {
  pub declarations: Vec<DeclarationNode>,
}

impl Node for RulesetNode {
  fn name(&self) -> &'static str {
    "Ruleset"
  }

  fn generate_code(&self, base_class: &str) -> String {
    let mut result = String::new();
    result.push_str(".");
    result.push_str(base_class);
    result.push_str(" {\n");

    for declaration in &self.declarations {
      result.push_str("  ");
      result.push_str(&declaration.generate_code(base_class));
      result.push_str("\n");
    }

    result.push_str("}");

    result
  }
}

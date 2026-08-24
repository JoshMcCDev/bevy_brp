//! `brp_execute_agent_tool` executes only developer-published agent-safe BRP methods.

use async_trait::async_trait;

use super::brp_execute;
use super::brp_execute::ExecuteParams;
use super::brp_execute::ExecuteResult;
use super::brp_list_agent_tools;
use super::brp_list_agent_tools::ListedAgentTool;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;
use crate::tool::ToolFn;

/// MCP handler for catalog-authorized dynamic BRP execution.
pub struct BrpExecuteAgentTool;

#[async_trait]
impl ToolFn for BrpExecuteAgentTool {
    type Output = ExecuteResult;
    type Params = ExecuteParams;

    async fn handle_impl(&self, params: ExecuteParams) -> Result<ExecuteResult> {
        let catalog = brp_list_agent_tools::fetch_catalog(params.port).await?;
        ensure_method_is_published(&catalog.tools, &params.method, params.port)?;
        brp_execute::execute_discovered(&params).await
    }
}

fn ensure_method_is_published(
    catalog: &[ListedAgentTool],
    requested_method: &str,
    port: Port,
) -> Result<()> {
    if catalog.iter().any(|tool| tool.method == requested_method) {
        return Ok(());
    }

    let available_methods = catalog
        .iter()
        .map(|tool| tool.method.clone())
        .collect::<Vec<_>>();
    Err(Error::tool_call_failed_with_details(
        format!(
            "BRP method `{requested_method}` is not published for agent execution on port {port}"
        ),
        serde_json::json!({
            "stage": "catalog_authorization",
            "method": requested_method,
            "port": port,
            "available_methods": available_methods,
        }),
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::ListedAgentTool;
    use super::ensure_method_is_published;
    use crate::brp_tools::Port;

    fn catalog() -> Vec<ListedAgentTool> {
        vec![ListedAgentTool {
            name: String::from("test_alpha"),
            method: String::from("test/alpha"),
            description: String::from("Published test method"),
            params_schema: None,
            result_schema: None,
        }]
    }

    #[test]
    fn curated_execution_accepts_an_exact_published_method() {
        assert!(ensure_method_is_published(&catalog(), "test/alpha", Port(21_234)).is_ok());
    }

    #[test]
    fn curated_execution_rejects_an_uncatalogued_method() {
        assert!(
            ensure_method_is_published(&catalog(), "world.insert_components", Port(21_234))
                .is_err()
        );
    }

    #[test]
    fn curated_execution_rejects_a_prefix_collision() {
        assert!(ensure_method_is_published(&catalog(), "test/alpha_more", Port(21_234)).is_err());
    }
}

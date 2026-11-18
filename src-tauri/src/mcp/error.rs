use rmcp::ErrorData;

#[derive(Debug, Clone)]
pub enum McpToolError {
    InvalidParams(String),
    /// 权限或授权不足（没有开启对应档位、环境未授权）
    Forbidden(String),
    Upstream(String),
    Internal(String),
}

impl McpToolError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams(message.into())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    pub fn upstream(message: impl Into<String>) -> Self {
        Self::Upstream(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}

impl From<anyhow::Error> for McpToolError {
    fn from(value: anyhow::Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<reqwest::Error> for McpToolError {
    fn from(value: reqwest::Error) -> Self {
        Self::upstream(value.to_string())
    }
}

impl From<McpToolError> for ErrorData {
    fn from(value: McpToolError) -> Self {
        match value {
            // MCP 客户端原来就把权限不足当作参数错误收到，这里保持不变
            McpToolError::InvalidParams(message) | McpToolError::Forbidden(message) => {
                ErrorData::invalid_params(message, None)
            }
            McpToolError::Upstream(message) | McpToolError::Internal(message) => {
                ErrorData::internal_error(message, None)
            }
        }
    }
}

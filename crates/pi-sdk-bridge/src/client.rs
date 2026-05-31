use pi_bridge_types::{
    BridgeCommand, BridgeResponse, CoreStateSnapshot, GetAuthStatusCommand, InitCommand,
    MessageCommand, ModelDescriptor, PromptCommand, ProviderAuthStatus, RemoveAuthCommand,
    SetApiKeyCommand,
};

use crate::{BridgeClientError, BridgeTransport, Result};

pub struct BridgeClient<T> {
    transport: T,
}

impl<T> BridgeClient<T>
where
    T: BridgeTransport,
{
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub async fn init(&self, command: InitCommand) -> Result<CoreStateSnapshot> {
        match self.transport.request(BridgeCommand::Init(command)).await? {
            BridgeResponse::State { state } => Ok(state),
            _other => Err(BridgeClientError::UnexpectedResponse("state")),
        }
    }

    pub async fn prompt(&self, text: impl Into<String>) -> Result<()> {
        let command = PromptCommand {
            text: text.into(),
            images: Vec::new(),
            streaming_behavior: None,
        };
        self.expect_ack(BridgeCommand::Prompt(command)).await
    }

    pub async fn steer(&self, text: impl Into<String>) -> Result<()> {
        self.expect_ack(BridgeCommand::Steer(MessageCommand {
            text: text.into(),
            images: Vec::new(),
        }))
        .await
    }

    pub async fn follow_up(&self, text: impl Into<String>) -> Result<()> {
        self.expect_ack(BridgeCommand::FollowUp(MessageCommand {
            text: text.into(),
            images: Vec::new(),
        }))
        .await
    }

    pub async fn available_models(&self) -> Result<Vec<ModelDescriptor>> {
        match self
            .transport
            .request(BridgeCommand::GetAvailableModels)
            .await?
        {
            BridgeResponse::Models { models } => Ok(models),
            _other => Err(BridgeClientError::UnexpectedResponse("models")),
        }
    }

    pub async fn auth_status(&self, provider: Option<String>) -> Result<Vec<ProviderAuthStatus>> {
        match self
            .transport
            .request(BridgeCommand::GetAuthStatus(GetAuthStatusCommand {
                provider,
            }))
            .await?
        {
            BridgeResponse::AuthStatus { statuses } => Ok(statuses),
            _other => Err(BridgeClientError::UnexpectedResponse("auth status")),
        }
    }

    pub async fn set_runtime_api_key(
        &self,
        provider: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<()> {
        self.set_api_key(provider, api_key, false).await
    }

    pub async fn set_persisted_api_key(
        &self,
        provider: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<()> {
        self.set_api_key(provider, api_key, true).await
    }

    pub async fn remove_auth(&self, provider: impl Into<String>) -> Result<()> {
        self.expect_ack(BridgeCommand::RemoveAuth(RemoveAuthCommand {
            provider: provider.into(),
        }))
        .await
    }

    async fn set_api_key(
        &self,
        provider: impl Into<String>,
        api_key: impl Into<String>,
        persist: bool,
    ) -> Result<()> {
        self.expect_ack(BridgeCommand::SetApiKey(SetApiKeyCommand {
            provider: provider.into(),
            api_key: api_key.into(),
            persist,
        }))
        .await
    }

    pub async fn state(&self) -> Result<CoreStateSnapshot> {
        match self.transport.request(BridgeCommand::GetState).await? {
            BridgeResponse::State { state } => Ok(state),
            _other => Err(BridgeClientError::UnexpectedResponse("state")),
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.expect_ack(BridgeCommand::Shutdown).await
    }

    pub async fn request(&self, command: BridgeCommand) -> Result<BridgeResponse> {
        self.transport.request(command).await
    }

    async fn expect_ack(&self, command: BridgeCommand) -> Result<()> {
        match self.transport.request(command).await? {
            BridgeResponse::Ack => Ok(()),
            _other => Err(BridgeClientError::UnexpectedResponse("ack")),
        }
    }
}

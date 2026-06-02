use pi_bridge_types::{
    BridgeCommand, BridgeResponse, CoreStateSnapshot, ForkCommand, ForkPosition,
    GetAuthStatusCommand, InitCommand, InstallPackageCommand, InstalledPackage, MessageCommand,
    ModelDescriptor, NewSessionCommand, OAuthLoginCommand, OAuthLoginMethod, PackageScopeCommand,
    PackageSearchResponse, PromptCommand, ProviderAuthStatus, RemoveAuthCommand,
    RemovePackageCommand, SearchPackagesCommand, SessionStateCommand, SetApiKeyCommand,
    SetSessionNameCommand, SwitchSessionCommand,
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
        self.prompt_for_session(None, text).await
    }

    pub async fn prompt_for_session(
        &self,
        session_path: Option<String>,
        text: impl Into<String>,
    ) -> Result<()> {
        let command = PromptCommand {
            session_path,
            text: text.into(),
            images: Vec::new(),
            streaming_behavior: None,
        };
        self.expect_ack(BridgeCommand::Prompt(command)).await
    }

    pub async fn session_state(
        &self,
        session_path: impl Into<String>,
    ) -> Result<CoreStateSnapshot> {
        match self
            .transport
            .request(BridgeCommand::GetSessionState(SessionStateCommand {
                session_path: session_path.into(),
            }))
            .await?
        {
            BridgeResponse::State { state } => Ok(state),
            _other => Err(BridgeClientError::UnexpectedResponse("state")),
        }
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

    pub async fn oauth_login(
        &self,
        provider: impl Into<String>,
        method: Option<OAuthLoginMethod>,
    ) -> Result<()> {
        self.expect_ack(BridgeCommand::OAuthLogin(OAuthLoginCommand {
            provider: provider.into(),
            method,
        }))
        .await
    }

    pub async fn remove_auth(&self, provider: impl Into<String>) -> Result<()> {
        self.expect_ack(BridgeCommand::RemoveAuth(RemoveAuthCommand {
            provider: provider.into(),
        }))
        .await
    }

    pub async fn search_packages(
        &self,
        query: impl Into<String>,
        limit: u32,
    ) -> Result<PackageSearchResponse> {
        match self
            .transport
            .request(BridgeCommand::SearchPackages(SearchPackagesCommand {
                query: query.into(),
                limit,
            }))
            .await?
        {
            BridgeResponse::Json { value } => Ok(serde_json::from_value(value)?),
            _other => Err(BridgeClientError::UnexpectedResponse("package search")),
        }
    }

    pub async fn list_packages(&self, cwd: impl Into<String>) -> Result<Vec<InstalledPackage>> {
        match self
            .transport
            .request(BridgeCommand::ListPackages(PackageScopeCommand {
                cwd: cwd.into(),
            }))
            .await?
        {
            BridgeResponse::Json { value } => Ok(serde_json::from_value(value)?),
            _other => Err(BridgeClientError::UnexpectedResponse("installed packages")),
        }
    }

    pub async fn install_package(
        &self,
        source: impl Into<String>,
        project: bool,
        cwd: impl Into<String>,
    ) -> Result<Vec<InstalledPackage>> {
        match self
            .transport
            .request(BridgeCommand::InstallPackage(InstallPackageCommand {
                source: source.into(),
                project,
                cwd: cwd.into(),
            }))
            .await?
        {
            BridgeResponse::Json { value } => Ok(serde_json::from_value(value)?),
            _other => Err(BridgeClientError::UnexpectedResponse("installed packages")),
        }
    }

    pub async fn remove_package(
        &self,
        source: impl Into<String>,
        project: bool,
        cwd: impl Into<String>,
    ) -> Result<Vec<InstalledPackage>> {
        match self
            .transport
            .request(BridgeCommand::RemovePackage(RemovePackageCommand {
                source: source.into(),
                project,
                cwd: cwd.into(),
            }))
            .await?
        {
            BridgeResponse::Json { value } => Ok(serde_json::from_value(value)?),
            _other => Err(BridgeClientError::UnexpectedResponse("installed packages")),
        }
    }

    pub async fn new_session(&self, parent_session: Option<String>) -> Result<bool> {
        match self
            .transport
            .request(BridgeCommand::NewSession(NewSessionCommand {
                parent_session,
            }))
            .await?
        {
            BridgeResponse::Cancelled { cancelled } => Ok(cancelled),
            _other => Err(BridgeClientError::UnexpectedResponse("cancelled")),
        }
    }

    pub async fn switch_session(
        &self,
        session_path: impl Into<String>,
        cwd_override: Option<String>,
    ) -> Result<bool> {
        match self
            .transport
            .request(BridgeCommand::SwitchSession(SwitchSessionCommand {
                session_path: session_path.into(),
                cwd_override,
            }))
            .await?
        {
            BridgeResponse::Cancelled { cancelled } => Ok(cancelled),
            _other => Err(BridgeClientError::UnexpectedResponse("cancelled")),
        }
    }

    pub async fn fork_session(
        &self,
        entry_id: impl Into<String>,
        position: ForkPosition,
    ) -> Result<()> {
        match self
            .transport
            .request(BridgeCommand::Fork(ForkCommand {
                entry_id: entry_id.into(),
                position,
            }))
            .await?
        {
            BridgeResponse::Json { .. } => Ok(()),
            _other => Err(BridgeClientError::UnexpectedResponse("json")),
        }
    }

    pub async fn set_session_name(&self, name: impl Into<String>) -> Result<()> {
        self.set_session_name_for_session(None, name).await
    }

    pub async fn set_session_name_for_session(
        &self,
        session_path: Option<String>,
        name: impl Into<String>,
    ) -> Result<()> {
        self.expect_ack(BridgeCommand::SetSessionName(SetSessionNameCommand {
            session_path,
            name: name.into(),
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

mod engine;
mod service;
mod store;

pub(crate) use service::InitialMembershipError;
pub use service::{
    Authorization, AuthorizationError, PolicyAdministrationError, ReplaceUserPermissions,
    ReplaceUserRoles,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveRoleGrant {
    pub id: i64,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectivePermissionGrant {
    pub permission: String,
    pub direct: bool,
    pub roles: Vec<EffectiveRoleGrant>,
}

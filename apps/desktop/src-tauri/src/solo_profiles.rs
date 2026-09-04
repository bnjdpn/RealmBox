//! Pure, reversible planning core for RealmBox solo profiles.
//!
//! The module contains no filesystem, process, Docker, or database effect. It
//! reads a worldserver configuration into typed values, produces an inspectable
//! plan and a versioned rollback snapshot, and renders a new configuration only
//! when the source still matches that snapshot. The integration layer remains
//! responsible for persisting the snapshot outside the replaceable runtime and
//! replacing the configuration atomically.
//!
//! Every key in [`ManagedSetting`] was checked in the pinned
//! `worldserver.conf.dist`. The values in [`ProfileCatalog::realm_box_v1`] are
//! RealmBox product choices inspired by solo play; they are not AzerothCore
//! defaults. No free-form key, command, or SQL can enter a plan.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};

pub const PROFILE_CATALOG_SCHEMA_VERSION: u32 = 1;
pub const REALMBOX_PROFILE_CATALOG_VERSION: u32 = 1;
pub const PROFILE_STATE_SCHEMA_VERSION: u32 = 1;
pub const PROFILE_PLAN_SCHEMA_VERSION: u32 = 1;
pub const PROFILE_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

const MIN_RATE_MILLI: u16 = 1_000;
const MAX_RATE_MILLI: u16 = 3_000;
const MAX_PRIMARY_TRADE_SKILLS: u8 = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoloProfile {
    Normal,
    Comfortable,
    Accelerated,
}

impl SoloProfile {
    pub const ALL: [Self; 3] = [Self::Normal, Self::Comfortable, Self::Accelerated];

    pub const fn label_fr(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Comfortable => "Confort",
            Self::Accelerated => "Accéléré",
        }
    }

    pub const fn label_en(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Comfortable => "Comfortable",
            Self::Accelerated => "Accelerated",
        }
    }
}

/// Closed set of keys which a solo profile may change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ManagedSetting {
    #[serde(rename = "Rate.XP.Kill")]
    XpKill,
    #[serde(rename = "Rate.XP.Quest")]
    XpQuest,
    #[serde(rename = "Rate.XP.Quest.DF")]
    XpDungeonFinderQuest,
    #[serde(rename = "Rate.XP.Explore")]
    XpExplore,
    #[serde(rename = "Rate.XP.Pet")]
    XpPet,
    #[serde(rename = "Rate.Reputation.Gain")]
    ReputationGain,
    #[serde(rename = "Rate.Drop.Money")]
    MoneyDrop,
    #[serde(rename = "MaxPrimaryTradeSkill")]
    MaxPrimaryTradeSkill,
    #[serde(rename = "Quests.IgnoreRaid")]
    QuestsIgnoreRaid,
    #[serde(rename = "Instance.IgnoreLevel")]
    InstanceIgnoreLevel,
    #[serde(rename = "Instance.IgnoreRaid")]
    InstanceIgnoreRaid,
}

impl ManagedSetting {
    pub const ALL: [Self; 11] = [
        Self::XpKill,
        Self::XpQuest,
        Self::XpDungeonFinderQuest,
        Self::XpExplore,
        Self::XpPet,
        Self::ReputationGain,
        Self::MoneyDrop,
        Self::MaxPrimaryTradeSkill,
        Self::QuestsIgnoreRaid,
        Self::InstanceIgnoreLevel,
        Self::InstanceIgnoreRaid,
    ];

    pub const fn config_key(self) -> &'static str {
        match self {
            Self::XpKill => "Rate.XP.Kill",
            Self::XpQuest => "Rate.XP.Quest",
            Self::XpDungeonFinderQuest => "Rate.XP.Quest.DF",
            Self::XpExplore => "Rate.XP.Explore",
            Self::XpPet => "Rate.XP.Pet",
            Self::ReputationGain => "Rate.Reputation.Gain",
            Self::MoneyDrop => "Rate.Drop.Money",
            Self::MaxPrimaryTradeSkill => "MaxPrimaryTradeSkill",
            Self::QuestsIgnoreRaid => "Quests.IgnoreRaid",
            Self::InstanceIgnoreLevel => "Instance.IgnoreLevel",
            Self::InstanceIgnoreRaid => "Instance.IgnoreRaid",
        }
    }

    pub fn from_config_key(key: &str) -> Result<Self, SoloProfileError> {
        Self::maybe_from_config_key(key)
            .ok_or_else(|| SoloProfileError::SettingNotAllowed(key.to_owned()))
    }

    fn maybe_from_config_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|setting| setting.config_key() == key)
    }

    const fn value_kind(self) -> ValueKind {
        match self {
            Self::XpKill
            | Self::XpQuest
            | Self::XpDungeonFinderQuest
            | Self::XpExplore
            | Self::XpPet
            | Self::ReputationGain
            | Self::MoneyDrop => ValueKind::Rate,
            Self::MaxPrimaryTradeSkill => ValueKind::Count,
            Self::QuestsIgnoreRaid | Self::InstanceIgnoreLevel | Self::InstanceIgnoreRaid => {
                ValueKind::Toggle
            }
        }
    }
}

impl fmt::Display for ManagedSetting {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.config_key())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    Rate,
    Count,
    Toggle,
}

/// Typed representation of the only value families accepted by the allowlist.
/// Rates use thousandths to avoid non-deterministic floating-point snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum SettingValue {
    RateMilli(u16),
    Count(u8),
    Toggle(bool),
}

impl SettingValue {
    pub const fn whole_rate(multiplier: u16) -> Self {
        Self::RateMilli(multiplier.saturating_mul(1_000))
    }

    pub fn config_value(self) -> String {
        match self {
            Self::RateMilli(milli) => render_rate(milli),
            Self::Count(count) => count.to_string(),
            Self::Toggle(enabled) => u8::from(enabled).to_string(),
        }
    }

    fn parse(setting: ManagedSetting, raw: &str) -> Result<Self, SoloProfileError> {
        let value = match setting.value_kind() {
            ValueKind::Rate => Self::RateMilli(parse_rate_milli(raw).ok_or_else(|| {
                SoloProfileError::InvalidConfigValue {
                    setting,
                    value: raw.to_owned(),
                }
            })?),
            ValueKind::Count => Self::Count(raw.parse::<u8>().map_err(|_| {
                SoloProfileError::InvalidConfigValue {
                    setting,
                    value: raw.to_owned(),
                }
            })?),
            ValueKind::Toggle => match raw {
                "0" => Self::Toggle(false),
                "1" => Self::Toggle(true),
                _ => {
                    return Err(SoloProfileError::InvalidConfigValue {
                        setting,
                        value: raw.to_owned(),
                    });
                }
            },
        };
        value.validate_for(setting)?;
        Ok(value)
    }

    fn validate_for(self, setting: ManagedSetting) -> Result<(), SoloProfileError> {
        match (setting.value_kind(), self) {
            (ValueKind::Rate, Self::RateMilli(milli))
                if (MIN_RATE_MILLI..=MAX_RATE_MILLI).contains(&milli) =>
            {
                Ok(())
            }
            (ValueKind::Rate, Self::RateMilli(milli)) => {
                Err(SoloProfileError::RateOutOfBounds { setting, milli })
            }
            (ValueKind::Count, Self::Count(count)) if count <= MAX_PRIMARY_TRADE_SKILLS => Ok(()),
            (ValueKind::Count, Self::Count(count)) => {
                Err(SoloProfileError::CountOutOfBounds { setting, count })
            }
            (ValueKind::Toggle, Self::Toggle(_)) => Ok(()),
            (expected, found) => Err(SoloProfileError::WrongValueKind {
                setting,
                expected: expected.name(),
                found: found.kind().name(),
            }),
        }
    }

    const fn kind(self) -> ValueKind {
        match self {
            Self::RateMilli(_) => ValueKind::Rate,
            Self::Count(_) => ValueKind::Count,
            Self::Toggle(_) => ValueKind::Toggle,
        }
    }
}

impl ValueKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Rate => "rate",
            Self::Count => "count",
            Self::Toggle => "toggle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProfileDefinition {
    pub profile: SoloProfile,
    pub values: BTreeMap<ManagedSetting, SettingValue>,
}

impl ProfileDefinition {
    pub fn new(
        profile: SoloProfile,
        values: BTreeMap<ManagedSetting, SettingValue>,
    ) -> Result<Self, SoloProfileError> {
        validate_complete_values(&values)?;
        Ok(Self { profile, values })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProfileCatalog {
    pub schema_version: u32,
    pub catalog_version: u32,
    pub profiles: Vec<ProfileDefinition>,
}

impl ProfileCatalog {
    pub fn new(
        catalog_version: u32,
        profiles: Vec<ProfileDefinition>,
    ) -> Result<Self, SoloProfileError> {
        let catalog = Self {
            schema_version: PROFILE_CATALOG_SCHEMA_VERSION,
            catalog_version,
            profiles,
        };
        catalog.validate()?;
        Ok(catalog)
    }

    /// RealmBox catalog version 1.
    ///
    /// Product policy (not upstream defaults):
    /// - Normal: x1 rates, two professions, solo access flags disabled.
    /// - Comfortable: x2 XP/reputation, x1 money, all professions and solo
    ///   access flags enabled.
    /// - Accelerated: x3 XP/reputation, x2 money, all professions and solo
    ///   access flags enabled.
    pub fn realm_box_v1() -> Result<Self, SoloProfileError> {
        Self::new(
            REALMBOX_PROFILE_CATALOG_VERSION,
            vec![
                product_profile(SoloProfile::Normal, 1, 1, 1, 2, false)?,
                product_profile(SoloProfile::Comfortable, 2, 2, 1, 11, true)?,
                product_profile(SoloProfile::Accelerated, 3, 3, 2, 11, true)?,
            ],
        )
    }

    pub fn decode(payload: &str) -> Result<Self, SoloProfileError> {
        let catalog: Self = serde_json::from_str(payload)
            .map_err(|error| SoloProfileError::MalformedDocument(error.to_string()))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn summaries(&self) -> Result<Vec<ProfileSummary>, SoloProfileError> {
        self.validate()?;
        self.profiles
            .iter()
            .map(|definition| {
                Ok(ProfileSummary {
                    catalog_version: self.catalog_version,
                    profile: definition.profile,
                    label_fr: definition.profile.label_fr().to_owned(),
                    label_en: definition.profile.label_en().to_owned(),
                    settings: definition
                        .values
                        .iter()
                        .map(|(setting, value)| ProfileSettingSummary {
                            key: setting.config_key().to_owned(),
                            value: value.config_value(),
                        })
                        .collect(),
                })
            })
            .collect()
    }

    pub fn definition(&self, profile: SoloProfile) -> Result<&ProfileDefinition, SoloProfileError> {
        self.profiles
            .iter()
            .find(|definition| definition.profile == profile)
            .ok_or(SoloProfileError::MissingProfile(profile))
    }

    fn detect_profile(
        &self,
        values: &BTreeMap<ManagedSetting, SettingValue>,
    ) -> Option<SoloProfile> {
        self.profiles
            .iter()
            .find(|definition| definition.values == *values)
            .map(|definition| definition.profile)
    }

    fn validate(&self) -> Result<(), SoloProfileError> {
        if self.schema_version != PROFILE_CATALOG_SCHEMA_VERSION {
            return Err(SoloProfileError::UnknownCatalogSchema(self.schema_version));
        }
        if self.catalog_version == 0 {
            return Err(SoloProfileError::InvalidCatalogVersion);
        }

        let mut found = BTreeSet::new();
        for definition in &self.profiles {
            if !found.insert(definition.profile) {
                return Err(SoloProfileError::DuplicateProfile(definition.profile));
            }
            validate_complete_values(&definition.values)?;
        }
        for profile in SoloProfile::ALL {
            if !found.contains(&profile) {
                return Err(SoloProfileError::MissingProfile(profile));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub catalog_version: u32,
    pub profile: SoloProfile,
    pub label_fr: String,
    pub label_en: String,
    pub settings: Vec<ProfileSettingSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSettingSummary {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProfileState {
    pub schema_version: u32,
    /// Supplied from the installation record by the integration layer.
    pub installation_schema_version: u32,
    pub active_profile: Option<SoloProfile>,
    pub values: BTreeMap<ManagedSetting, SettingValue>,
}

impl ProfileState {
    pub fn new(
        installation_schema_version: u32,
        active_profile: Option<SoloProfile>,
        values: BTreeMap<ManagedSetting, SettingValue>,
    ) -> Result<Self, SoloProfileError> {
        let state = Self {
            schema_version: PROFILE_STATE_SCHEMA_VERSION,
            installation_schema_version,
            active_profile,
            values,
        };
        state.validate_document()?;
        Ok(state)
    }

    pub fn decode(payload: &str) -> Result<Self, SoloProfileError> {
        let state: Self = serde_json::from_str(payload)
            .map_err(|error| SoloProfileError::MalformedDocument(error.to_string()))?;
        state.validate_document()?;
        Ok(state)
    }

    fn validate_document(&self) -> Result<(), SoloProfileError> {
        if self.schema_version != PROFILE_STATE_SCHEMA_VERSION {
            return Err(SoloProfileError::UnknownStateSchema(self.schema_version));
        }
        validate_complete_values(&self.values)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PlannedChange {
    pub setting: ManagedSetting,
    pub previous: SettingValue,
    pub next: SettingValue,
}

impl PlannedChange {
    pub fn config_assignment(&self) -> String {
        format!(
            "{} = {}",
            self.setting.config_key(),
            self.next.config_value()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProfileSnapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub catalog_version: u32,
    pub state_schema_version: u32,
    pub installation_schema_version: u32,
    pub previous_profile: Option<SoloProfile>,
    pub target_profile: SoloProfile,
    pub previous_values: BTreeMap<ManagedSetting, SettingValue>,
    pub applied_values: BTreeMap<ManagedSetting, SettingValue>,
}

impl ProfileSnapshot {
    pub fn encode(&self) -> Result<String, SoloProfileError> {
        serde_json::to_string_pretty(self)
            .map_err(|error| SoloProfileError::MalformedDocument(error.to_string()))
    }

    pub fn decode(
        payload: &str,
        supported_installation_schema_version: u32,
    ) -> Result<Self, SoloProfileError> {
        let snapshot: Self = serde_json::from_str(payload)
            .map_err(|error| SoloProfileError::MalformedDocument(error.to_string()))?;
        snapshot.validate(supported_installation_schema_version)?;
        Ok(snapshot)
    }

    fn validate(&self, supported_installation_schema_version: u32) -> Result<(), SoloProfileError> {
        if self.schema_version != PROFILE_SNAPSHOT_SCHEMA_VERSION {
            return Err(SoloProfileError::UnknownSnapshotSchema(self.schema_version));
        }
        if self.state_schema_version != PROFILE_STATE_SCHEMA_VERSION {
            return Err(SoloProfileError::UnknownStateSchema(
                self.state_schema_version,
            ));
        }
        if self.catalog_version == 0 {
            return Err(SoloProfileError::InvalidCatalogVersion);
        }
        validate_installation_schema(
            self.installation_schema_version,
            supported_installation_schema_version,
        )?;
        if self.revision == 0 {
            return Err(SoloProfileError::InvalidSnapshotRevision);
        }
        validate_complete_values(&self.previous_values)?;
        validate_complete_values(&self.applied_values)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProfilePlan {
    pub schema_version: u32,
    pub catalog_version: u32,
    pub target_profile: SoloProfile,
    pub changes: Vec<PlannedChange>,
    pub snapshot: ProfileSnapshot,
}

impl ProfilePlan {
    fn validate(&self, supported_installation_schema_version: u32) -> Result<(), SoloProfileError> {
        if self.schema_version != PROFILE_PLAN_SCHEMA_VERSION {
            return Err(SoloProfileError::UnknownPlanSchema(self.schema_version));
        }
        self.snapshot
            .validate(supported_installation_schema_version)?;
        if self.catalog_version != self.snapshot.catalog_version {
            return Err(SoloProfileError::PlanCatalogMismatch);
        }
        if self.target_profile != self.snapshot.target_profile {
            return Err(SoloProfileError::PlanTargetMismatch);
        }

        let expected_changes = self
            .snapshot
            .applied_values
            .iter()
            .filter_map(|(setting, next)| {
                let previous = self.snapshot.previous_values.get(setting)?;
                (previous != next).then_some(PlannedChange {
                    setting: *setting,
                    previous: *previous,
                    next: *next,
                })
            })
            .collect::<Vec<_>>();
        if self.changes != expected_changes {
            return Err(SoloProfileError::PlanChangesMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationOutcome {
    Applied,
    AlreadyInRequestedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigTextUpdate {
    pub contents: String,
    pub outcome: MutationOutcome,
}

#[derive(Debug, Clone)]
pub struct SoloProfileEngine {
    supported_installation_schema_version: u32,
    catalog: ProfileCatalog,
}

impl SoloProfileEngine {
    pub fn new(
        supported_installation_schema_version: u32,
        catalog: ProfileCatalog,
    ) -> Result<Self, SoloProfileError> {
        if supported_installation_schema_version == 0 {
            return Err(SoloProfileError::InvalidSupportedInstallationSchema);
        }
        catalog.validate()?;
        Ok(Self {
            supported_installation_schema_version,
            catalog,
        })
    }

    pub fn profile_summaries(&self) -> Result<Vec<ProfileSummary>, SoloProfileError> {
        self.catalog.summaries()
    }

    /// Inspects all managed keys and detects an exact catalog profile. A custom
    /// but in-bounds configuration is reported with no active profile.
    pub fn inspect_config(
        &self,
        installation_schema_version: u32,
        source: &str,
    ) -> Result<ProfileState, SoloProfileError> {
        validate_installation_schema(
            installation_schema_version,
            self.supported_installation_schema_version,
        )?;
        let values = parse_config_values(source)?;
        let active_profile = self.catalog.detect_profile(&values);
        ProfileState::new(installation_schema_version, active_profile, values)
    }

    pub fn plan_transition(
        &self,
        state: &ProfileState,
        target_profile: SoloProfile,
        snapshot_revision: u64,
    ) -> Result<ProfilePlan, SoloProfileError> {
        self.validate_state(state)?;
        if snapshot_revision == 0 {
            return Err(SoloProfileError::InvalidSnapshotRevision);
        }
        self.ensure_active_profile_is_consistent(state)?;

        let definition = self.catalog.definition(target_profile)?;
        let changes = definition
            .values
            .iter()
            .filter_map(|(setting, next)| {
                let previous = state.values.get(setting)?;
                (previous != next).then_some(PlannedChange {
                    setting: *setting,
                    previous: *previous,
                    next: *next,
                })
            })
            .collect();
        let snapshot = ProfileSnapshot {
            schema_version: PROFILE_SNAPSHOT_SCHEMA_VERSION,
            revision: snapshot_revision,
            catalog_version: self.catalog.catalog_version,
            state_schema_version: state.schema_version,
            installation_schema_version: state.installation_schema_version,
            previous_profile: state.active_profile,
            target_profile,
            previous_values: state.values.clone(),
            applied_values: definition.values.clone(),
        };
        let plan = ProfilePlan {
            schema_version: PROFILE_PLAN_SCHEMA_VERSION,
            catalog_version: self.catalog.catalog_version,
            target_profile,
            changes,
            snapshot,
        };
        plan.validate(self.supported_installation_schema_version)?;
        Ok(plan)
    }

    pub fn apply_plan(
        &self,
        state: &mut ProfileState,
        plan: &ProfilePlan,
    ) -> Result<MutationOutcome, SoloProfileError> {
        self.validate_state(state)?;
        self.validate_current_plan(plan)?;
        if state.installation_schema_version != plan.snapshot.installation_schema_version {
            return Err(SoloProfileError::InstallationSchemaChanged {
                expected: plan.snapshot.installation_schema_version,
                found: state.installation_schema_version,
            });
        }

        if state_matches(
            state,
            Some(plan.target_profile),
            &plan.snapshot.applied_values,
        ) {
            return Ok(MutationOutcome::AlreadyInRequestedState);
        }
        require_state_matches(
            state,
            plan.snapshot.previous_profile,
            &plan.snapshot.previous_values,
        )?;

        state.values.clone_from(&plan.snapshot.applied_values);
        state.active_profile = Some(plan.target_profile);
        Ok(MutationOutcome::Applied)
    }

    /// Renders the plan against the exact configuration it was built from.
    /// Unmanaged lines and unchanged managed lines remain byte-for-byte intact.
    pub fn apply_plan_to_config(
        &self,
        source: &str,
        plan: &ProfilePlan,
    ) -> Result<ConfigTextUpdate, SoloProfileError> {
        self.validate_current_plan(plan)?;
        let current = parse_config_values(source)?;
        if current == plan.snapshot.applied_values {
            return Ok(ConfigTextUpdate {
                contents: source.to_owned(),
                outcome: MutationOutcome::AlreadyInRequestedState,
            });
        }
        require_values_match(&current, &plan.snapshot.previous_values)?;
        Ok(ConfigTextUpdate {
            contents: rewrite_config(source, &plan.snapshot.applied_values)?,
            outcome: MutationOutcome::Applied,
        })
    }

    /// Completes an already persisted apply transaction from its validated
    /// snapshot. Unlike a new transition, recovery intentionally does not
    /// consult the current product catalog: the snapshot is the durable source
    /// of truth and still enforces the closed allowlist, typed bounds, document
    /// schemas, and installation schema.
    pub(crate) fn apply_snapshot_config(
        &self,
        source: &str,
        snapshot: &ProfileSnapshot,
    ) -> Result<ConfigTextUpdate, SoloProfileError> {
        snapshot.validate(self.supported_installation_schema_version)?;
        let current = parse_config_values(source)?;
        if current == snapshot.applied_values {
            return Ok(ConfigTextUpdate {
                contents: source.to_owned(),
                outcome: MutationOutcome::AlreadyInRequestedState,
            });
        }
        require_values_match(&current, &snapshot.previous_values)?;
        Ok(ConfigTextUpdate {
            contents: rewrite_config(source, &snapshot.applied_values)?,
            outcome: MutationOutcome::Applied,
        })
    }

    /// Restores the exact values from a persisted snapshot. It does not depend
    /// on the current catalog, so a catalog update cannot make recovery
    /// impossible. Repeating a successful rollback is a no-op.
    pub fn rollback(
        &self,
        state: &mut ProfileState,
        snapshot: &ProfileSnapshot,
    ) -> Result<MutationOutcome, SoloProfileError> {
        self.validate_state(state)?;
        snapshot.validate(self.supported_installation_schema_version)?;
        if state.installation_schema_version != snapshot.installation_schema_version {
            return Err(SoloProfileError::InstallationSchemaChanged {
                expected: snapshot.installation_schema_version,
                found: state.installation_schema_version,
            });
        }

        if state_matches(state, snapshot.previous_profile, &snapshot.previous_values) {
            return Ok(MutationOutcome::AlreadyInRequestedState);
        }
        require_state_matches(
            state,
            Some(snapshot.target_profile),
            &snapshot.applied_values,
        )?;

        state.values.clone_from(&snapshot.previous_values);
        state.active_profile = snapshot.previous_profile;
        Ok(MutationOutcome::Applied)
    }

    pub fn rollback_config(
        &self,
        source: &str,
        snapshot: &ProfileSnapshot,
    ) -> Result<ConfigTextUpdate, SoloProfileError> {
        snapshot.validate(self.supported_installation_schema_version)?;
        let current = parse_config_values(source)?;
        if current == snapshot.previous_values {
            return Ok(ConfigTextUpdate {
                contents: source.to_owned(),
                outcome: MutationOutcome::AlreadyInRequestedState,
            });
        }
        require_values_match(&current, &snapshot.applied_values)?;
        Ok(ConfigTextUpdate {
            contents: rewrite_config(source, &snapshot.previous_values)?,
            outcome: MutationOutcome::Applied,
        })
    }

    fn validate_current_plan(&self, plan: &ProfilePlan) -> Result<(), SoloProfileError> {
        plan.validate(self.supported_installation_schema_version)?;
        if plan.catalog_version != self.catalog.catalog_version {
            return Err(SoloProfileError::CatalogChanged {
                expected: plan.catalog_version,
                found: self.catalog.catalog_version,
            });
        }
        if plan.snapshot.applied_values != self.catalog.definition(plan.target_profile)?.values {
            return Err(SoloProfileError::PlanTargetValuesMismatch);
        }
        Ok(())
    }

    fn validate_state(&self, state: &ProfileState) -> Result<(), SoloProfileError> {
        state.validate_document()?;
        validate_installation_schema(
            state.installation_schema_version,
            self.supported_installation_schema_version,
        )
    }

    fn ensure_active_profile_is_consistent(
        &self,
        state: &ProfileState,
    ) -> Result<(), SoloProfileError> {
        let Some(active_profile) = state.active_profile else {
            return Ok(());
        };
        let active_definition = self.catalog.definition(active_profile)?;
        require_values_match(&state.values, &active_definition.values)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoloProfileError {
    UnknownCatalogSchema(u32),
    UnknownStateSchema(u32),
    UnknownPlanSchema(u32),
    UnknownSnapshotSchema(u32),
    UnknownInstallationSchema {
        supported: u32,
        found: u32,
    },
    InstallationSchemaChanged {
        expected: u32,
        found: u32,
    },
    InvalidSupportedInstallationSchema,
    InvalidCatalogVersion,
    InvalidSnapshotRevision,
    MissingProfile(SoloProfile),
    DuplicateProfile(SoloProfile),
    SettingNotAllowed(String),
    MissingManagedSetting(ManagedSetting),
    DuplicateManagedSetting(ManagedSetting),
    InvalidConfigValue {
        setting: ManagedSetting,
        value: String,
    },
    WrongValueKind {
        setting: ManagedSetting,
        expected: &'static str,
        found: &'static str,
    },
    RateOutOfBounds {
        setting: ManagedSetting,
        milli: u16,
    },
    CountOutOfBounds {
        setting: ManagedSetting,
        count: u8,
    },
    ActiveProfileChanged {
        expected: Option<SoloProfile>,
        found: Option<SoloProfile>,
    },
    SettingChanged {
        setting: ManagedSetting,
        expected: SettingValue,
        found: Option<SettingValue>,
    },
    PlanCatalogMismatch,
    PlanTargetMismatch,
    PlanTargetValuesMismatch,
    PlanChangesMismatch,
    CatalogChanged {
        expected: u32,
        found: u32,
    },
    MalformedDocument(String),
}

impl fmt::Display for SoloProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCatalogSchema(version) => {
                write!(formatter, "unknown solo profile catalog schema {version}")
            }
            Self::UnknownStateSchema(version) => {
                write!(formatter, "unknown solo profile state schema {version}")
            }
            Self::UnknownPlanSchema(version) => {
                write!(formatter, "unknown solo profile plan schema {version}")
            }
            Self::UnknownSnapshotSchema(version) => {
                write!(formatter, "unknown solo profile snapshot schema {version}")
            }
            Self::UnknownInstallationSchema { supported, found } => write!(
                formatter,
                "installation schema {found} is not supported; expected {supported}"
            ),
            Self::InstallationSchemaChanged { expected, found } => write!(
                formatter,
                "installation schema changed after planning: expected {expected}, found {found}"
            ),
            Self::InvalidSupportedInstallationSchema => {
                formatter.write_str("supported installation schema must be non-zero")
            }
            Self::InvalidCatalogVersion => {
                formatter.write_str("profile catalog version must be non-zero")
            }
            Self::InvalidSnapshotRevision => {
                formatter.write_str("snapshot revision must be non-zero")
            }
            Self::MissingProfile(profile) => {
                write!(formatter, "profile {profile:?} is missing from the catalog")
            }
            Self::DuplicateProfile(profile) => {
                write!(formatter, "profile {profile:?} is declared more than once")
            }
            Self::SettingNotAllowed(key) => {
                write!(formatter, "configuration key {key} is not allowlisted")
            }
            Self::MissingManagedSetting(setting) => {
                write!(formatter, "managed setting {setting} is missing")
            }
            Self::DuplicateManagedSetting(setting) => {
                write!(formatter, "managed setting {setting} occurs more than once")
            }
            Self::InvalidConfigValue { setting, value } => {
                write!(formatter, "invalid value {value:?} for {setting}")
            }
            Self::WrongValueKind {
                setting,
                expected,
                found,
            } => write!(
                formatter,
                "{setting} expects a {expected} value, not {found}"
            ),
            Self::RateOutOfBounds { setting, milli } => write!(
                formatter,
                "rate {} for {setting} is outside RealmBox's 1..=3 bound",
                render_rate(*milli)
            ),
            Self::CountOutOfBounds { setting, count } => write!(
                formatter,
                "count {count} for {setting} is outside AzerothCore's 0..=11 bound"
            ),
            Self::ActiveProfileChanged { expected, found } => write!(
                formatter,
                "active profile changed after planning: expected {expected:?}, found {found:?}"
            ),
            Self::SettingChanged {
                setting,
                expected,
                found,
            } => write!(
                formatter,
                "{setting} changed after planning: expected {expected:?}, found {found:?}"
            ),
            Self::PlanCatalogMismatch => {
                formatter.write_str("plan and snapshot catalog versions differ")
            }
            Self::PlanTargetMismatch => {
                formatter.write_str("plan target does not match its snapshot")
            }
            Self::PlanTargetValuesMismatch => {
                formatter.write_str("plan values do not match the current catalog profile")
            }
            Self::PlanChangesMismatch => {
                formatter.write_str("plan changes do not match its typed snapshot")
            }
            Self::CatalogChanged { expected, found } => write!(
                formatter,
                "profile catalog changed after planning: expected {expected}, found {found}"
            ),
            Self::MalformedDocument(error) => {
                write!(formatter, "malformed profile document: {error}")
            }
        }
    }
}

impl Error for SoloProfileError {}

fn product_profile(
    profile: SoloProfile,
    xp_rate: u16,
    reputation_rate: u16,
    money_rate: u16,
    max_professions: u8,
    solo_access: bool,
) -> Result<ProfileDefinition, SoloProfileError> {
    ProfileDefinition::new(
        profile,
        BTreeMap::from([
            (ManagedSetting::XpKill, SettingValue::whole_rate(xp_rate)),
            (ManagedSetting::XpQuest, SettingValue::whole_rate(xp_rate)),
            (
                ManagedSetting::XpDungeonFinderQuest,
                SettingValue::whole_rate(xp_rate),
            ),
            (ManagedSetting::XpExplore, SettingValue::whole_rate(xp_rate)),
            (ManagedSetting::XpPet, SettingValue::whole_rate(xp_rate)),
            (
                ManagedSetting::ReputationGain,
                SettingValue::whole_rate(reputation_rate),
            ),
            (
                ManagedSetting::MoneyDrop,
                SettingValue::whole_rate(money_rate),
            ),
            (
                ManagedSetting::MaxPrimaryTradeSkill,
                SettingValue::Count(max_professions),
            ),
            (
                ManagedSetting::QuestsIgnoreRaid,
                SettingValue::Toggle(solo_access),
            ),
            (
                ManagedSetting::InstanceIgnoreLevel,
                SettingValue::Toggle(solo_access),
            ),
            (
                ManagedSetting::InstanceIgnoreRaid,
                SettingValue::Toggle(solo_access),
            ),
        ]),
    )
}

fn validate_installation_schema(found: u32, supported: u32) -> Result<(), SoloProfileError> {
    if found != supported {
        return Err(SoloProfileError::UnknownInstallationSchema { supported, found });
    }
    Ok(())
}

fn validate_complete_values(
    values: &BTreeMap<ManagedSetting, SettingValue>,
) -> Result<(), SoloProfileError> {
    for setting in ManagedSetting::ALL {
        let value = values
            .get(&setting)
            .copied()
            .ok_or(SoloProfileError::MissingManagedSetting(setting))?;
        value.validate_for(setting)?;
    }
    Ok(())
}

fn parse_config_values(
    source: &str,
) -> Result<BTreeMap<ManagedSetting, SettingValue>, SoloProfileError> {
    let mut values = BTreeMap::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let Some(setting) = ManagedSetting::maybe_from_config_key(raw_key.trim()) else {
            continue;
        };
        if values.contains_key(&setting) {
            return Err(SoloProfileError::DuplicateManagedSetting(setting));
        }
        let value_without_comment = raw_value
            .split_once('#')
            .map_or(raw_value, |(value, _)| value);
        let value = SettingValue::parse(setting, value_without_comment.trim())?;
        values.insert(setting, value);
    }
    validate_complete_values(&values)?;
    Ok(values)
}

fn rewrite_config(
    source: &str,
    replacements: &BTreeMap<ManagedSetting, SettingValue>,
) -> Result<String, SoloProfileError> {
    validate_complete_values(replacements)?;
    let mut output = String::with_capacity(source.len());
    for segment in source.split_inclusive('\n') {
        let (line, ending) = if let Some(line) = segment.strip_suffix("\r\n") {
            (line, "\r\n")
        } else if let Some(line) = segment.strip_suffix('\n') {
            (line, "\n")
        } else {
            (segment, "")
        };
        let trimmed = line.trim();
        let setting = (!trimmed.is_empty() && !trimmed.starts_with('#'))
            .then(|| trimmed.split_once('='))
            .flatten()
            .and_then(|(key, _)| ManagedSetting::maybe_from_config_key(key.trim()));
        if let Some(setting) = setting {
            let value = replacements
                .get(&setting)
                .copied()
                .ok_or(SoloProfileError::MissingManagedSetting(setting))?;
            let raw_value = line
                .split_once('=')
                .map(|(_, raw_value)| raw_value)
                .ok_or_else(|| SoloProfileError::InvalidConfigValue {
                    setting,
                    value: line.to_owned(),
                })?;
            let previous_raw = raw_value
                .split_once('#')
                .map_or(raw_value, |(previous, _)| previous);
            if SettingValue::parse(setting, previous_raw.trim())? == value {
                output.push_str(segment);
                continue;
            }
            let comment = line
                .split_once('=')
                .and_then(|(_, value)| value.find('#').map(|offset| &value[offset..]));
            output.push_str(setting.config_key());
            output.push_str(" = ");
            output.push_str(&value.config_value());
            if let Some(comment) = comment {
                output.push(' ');
                output.push_str(comment.trim_start());
            }
            output.push_str(ending);
        } else {
            output.push_str(segment);
        }
    }
    Ok(output)
}

fn parse_rate_milli(raw: &str) -> Option<u16> {
    if raw.is_empty() || raw.starts_with(['+', '-']) {
        return None;
    }
    let mut parts = raw.split('.');
    let whole = parts.next()?.parse::<u16>().ok()?;
    let fraction = parts.next();
    if parts.next().is_some() {
        return None;
    }
    let fractional_milli = match fraction {
        None | Some("") => 0,
        Some(digits) if digits.len() <= 3 && digits.bytes().all(|byte| byte.is_ascii_digit()) => {
            let parsed = digits.parse::<u16>().ok()?;
            parsed * 10_u16.pow((3 - digits.len()) as u32)
        }
        Some(_) => return None,
    };
    whole.checked_mul(1_000)?.checked_add(fractional_milli)
}

fn render_rate(milli: u16) -> String {
    let whole = milli / 1_000;
    let fraction = milli % 1_000;
    if fraction == 0 {
        return whole.to_string();
    }
    format!("{whole}.{fraction:03}")
        .trim_end_matches('0')
        .to_owned()
}

fn state_matches(
    state: &ProfileState,
    profile: Option<SoloProfile>,
    expected_values: &BTreeMap<ManagedSetting, SettingValue>,
) -> bool {
    state.active_profile == profile && state.values == *expected_values
}

fn require_state_matches(
    state: &ProfileState,
    profile: Option<SoloProfile>,
    expected_values: &BTreeMap<ManagedSetting, SettingValue>,
) -> Result<(), SoloProfileError> {
    if state.active_profile != profile {
        return Err(SoloProfileError::ActiveProfileChanged {
            expected: profile,
            found: state.active_profile,
        });
    }
    require_values_match(&state.values, expected_values)
}

fn require_values_match(
    actual_values: &BTreeMap<ManagedSetting, SettingValue>,
    expected_values: &BTreeMap<ManagedSetting, SettingValue>,
) -> Result<(), SoloProfileError> {
    for setting in ManagedSetting::ALL {
        let expected = expected_values
            .get(&setting)
            .copied()
            .ok_or(SoloProfileError::MissingManagedSetting(setting))?;
        let found = actual_values.get(&setting).copied();
        if found != Some(expected) {
            return Err(SoloProfileError::SettingChanged {
                setting,
                expected,
                found,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALLATION_SCHEMA: u32 = 3;

    fn engine() -> SoloProfileEngine {
        SoloProfileEngine::new(INSTALLATION_SCHEMA, ProfileCatalog::realm_box_v1().unwrap())
            .unwrap()
    }

    fn normal_config() -> String {
        "# untouched comment\n\
Rate.XP.Kill      = 1\n\
Rate.XP.Quest     = 1\n\
Rate.XP.Quest.DF  = 1\n\
Rate.XP.Explore   = 1\n\
Rate.XP.Pet       = 1\n\
Rate.Reputation.Gain = 1\n\
Rate.Drop.Money                 = 1 # keep me\n\
MaxPrimaryTradeSkill = 2\n\
Quests.IgnoreRaid = 0\n\
Instance.IgnoreLevel = 0\n\
Instance.IgnoreRaid = 0\n\
Unmanaged.Setting = 42\n"
            .to_owned()
    }

    #[test]
    fn product_catalog_models_the_three_reviewed_profiles() {
        let catalog = ProfileCatalog::realm_box_v1().unwrap();
        let summaries = catalog.summaries().unwrap();
        assert_eq!(
            summaries
                .iter()
                .map(|summary| summary.profile)
                .collect::<Vec<_>>(),
            SoloProfile::ALL
        );
        assert_eq!(summaries[1].label_fr, "Confort");
        assert_eq!(summaries[2].label_en, "Accelerated");

        let comfortable = catalog.definition(SoloProfile::Comfortable).unwrap();
        assert_eq!(
            comfortable.values[&ManagedSetting::XpQuest],
            SettingValue::whole_rate(2)
        );
        assert_eq!(
            comfortable.values[&ManagedSetting::MoneyDrop],
            SettingValue::whole_rate(1)
        );
        assert_eq!(
            comfortable.values[&ManagedSetting::MaxPrimaryTradeSkill],
            SettingValue::Count(11)
        );
        assert_eq!(
            comfortable.values[&ManagedSetting::InstanceIgnoreRaid],
            SettingValue::Toggle(true)
        );
    }

    #[test]
    fn inspects_config_and_produces_only_typed_allowlisted_changes() {
        let engine = engine();
        let state = engine
            .inspect_config(INSTALLATION_SCHEMA, &normal_config())
            .unwrap();
        assert_eq!(state.active_profile, Some(SoloProfile::Normal));

        let plan = engine
            .plan_transition(&state, SoloProfile::Comfortable, 41)
            .unwrap();
        assert_eq!(plan.snapshot.revision, 41);
        assert_eq!(plan.changes.len(), 10);
        assert!(
            plan.changes
                .iter()
                .all(|change| ManagedSetting::from_config_key(change.setting.config_key()).is_ok())
        );
        assert!(
            plan.changes
                .iter()
                .all(|change| !change.config_assignment().contains(';'))
        );
        assert!(
            plan.changes
                .iter()
                .any(|change| { change.config_assignment() == "MaxPrimaryTradeSkill = 11" })
        );
    }

    #[test]
    fn applies_to_text_without_touching_unmanaged_content() {
        let engine = engine();
        let original = normal_config();
        let state = engine
            .inspect_config(INSTALLATION_SCHEMA, &original)
            .unwrap();
        let plan = engine
            .plan_transition(&state, SoloProfile::Comfortable, 1)
            .unwrap();

        let update = engine.apply_plan_to_config(&original, &plan).unwrap();
        assert_eq!(update.outcome, MutationOutcome::Applied);
        assert!(update.contents.contains("# untouched comment"));
        assert!(update.contents.contains("Unmanaged.Setting = 42"));
        assert!(update.contents.contains("Rate.XP.Kill = 2"));
        assert!(
            update
                .contents
                .contains("Rate.Drop.Money                 = 1 # keep me")
        );
        assert!(update.contents.contains("Instance.IgnoreRaid = 1"));

        let applied_state = engine
            .inspect_config(INSTALLATION_SCHEMA, &update.contents)
            .unwrap();
        assert_eq!(applied_state.active_profile, Some(SoloProfile::Comfortable));
    }

    #[test]
    fn rejects_unknown_keys_duplicate_keys_and_unsafe_values() {
        assert_eq!(
            ManagedSetting::from_config_key("Rate.XP.Bogus"),
            Err(SoloProfileError::SettingNotAllowed(
                "Rate.XP.Bogus".to_owned()
            ))
        );

        let duplicate = format!("{}Rate.XP.Kill = 1\n", normal_config());
        assert_eq!(
            engine().inspect_config(INSTALLATION_SCHEMA, &duplicate),
            Err(SoloProfileError::DuplicateManagedSetting(
                ManagedSetting::XpKill
            ))
        );

        let too_fast = normal_config().replace("Rate.XP.Kill      = 1", "Rate.XP.Kill = 4");
        assert_eq!(
            engine().inspect_config(INSTALLATION_SCHEMA, &too_fast),
            Err(SoloProfileError::RateOutOfBounds {
                setting: ManagedSetting::XpKill,
                milli: 4_000,
            })
        );

        let wrong_toggle =
            normal_config().replace("Instance.IgnoreRaid = 0", "Instance.IgnoreRaid = yes");
        assert_eq!(
            engine().inspect_config(INSTALLATION_SCHEMA, &wrong_toggle),
            Err(SoloProfileError::InvalidConfigValue {
                setting: ManagedSetting::InstanceIgnoreRaid,
                value: "yes".to_owned(),
            })
        );
    }

    #[test]
    fn refuses_unknown_document_and_installation_schemas() {
        let unknown_catalog = r#"{
            "schemaVersion": 999,
            "catalogVersion": 1,
            "profiles": []
        }"#;
        assert_eq!(
            ProfileCatalog::decode(unknown_catalog),
            Err(SoloProfileError::UnknownCatalogSchema(999))
        );
        assert_eq!(
            engine().inspect_config(999, &normal_config()),
            Err(SoloProfileError::UnknownInstallationSchema {
                supported: INSTALLATION_SCHEMA,
                found: 999,
            })
        );
    }

    #[test]
    fn snapshot_round_trip_recovers_text_and_rollback_is_idempotent() {
        let engine = engine();
        let original = normal_config();
        let mut state = engine
            .inspect_config(INSTALLATION_SCHEMA, &original)
            .unwrap();
        let plan = engine
            .plan_transition(&state, SoloProfile::Accelerated, 7)
            .unwrap();
        let applied = engine.apply_plan_to_config(&original, &plan).unwrap();
        engine.apply_plan(&mut state, &plan).unwrap();

        let persisted = plan.snapshot.encode().unwrap();
        let recovered = ProfileSnapshot::decode(&persisted, INSTALLATION_SCHEMA).unwrap();
        assert_eq!(recovered, plan.snapshot);

        let restored = engine
            .rollback_config(&applied.contents, &recovered)
            .unwrap();
        assert_eq!(restored.outcome, MutationOutcome::Applied);
        assert!(restored.contents.contains("Rate.XP.Kill = 1"));
        assert!(restored.contents.contains("MaxPrimaryTradeSkill = 2"));
        assert!(restored.contents.contains("Unmanaged.Setting = 42"));
        assert_eq!(
            engine.rollback(&mut state, &recovered).unwrap(),
            MutationOutcome::Applied
        );
        assert_eq!(state.active_profile, Some(SoloProfile::Normal));

        assert_eq!(
            engine
                .rollback_config(&restored.contents, &recovered)
                .unwrap()
                .outcome,
            MutationOutcome::AlreadyInRequestedState
        );
        assert_eq!(
            engine.rollback(&mut state, &recovered).unwrap(),
            MutationOutcome::AlreadyInRequestedState
        );
    }

    #[test]
    fn apply_and_rollback_refuse_concurrent_config_drift() {
        let engine = engine();
        let original = normal_config();
        let state = engine
            .inspect_config(INSTALLATION_SCHEMA, &original)
            .unwrap();
        let plan = engine
            .plan_transition(&state, SoloProfile::Comfortable, 2)
            .unwrap();

        let drifted_before_apply =
            original.replace("Rate.Drop.Money                 = 1", "Rate.Drop.Money = 2");
        assert_eq!(
            engine.apply_plan_to_config(&drifted_before_apply, &plan),
            Err(SoloProfileError::SettingChanged {
                setting: ManagedSetting::MoneyDrop,
                expected: SettingValue::whole_rate(1),
                found: Some(SettingValue::whole_rate(2)),
            })
        );

        let applied = engine.apply_plan_to_config(&original, &plan).unwrap();
        let drifted_before_rollback = applied
            .contents
            .replace("Rate.XP.Quest = 2", "Rate.XP.Quest = 1.5");
        assert_eq!(
            engine.rollback_config(&drifted_before_rollback, &plan.snapshot),
            Err(SoloProfileError::SettingChanged {
                setting: ManagedSetting::XpQuest,
                expected: SettingValue::whole_rate(2),
                found: Some(SettingValue::RateMilli(1_500)),
            })
        );
    }

    #[test]
    fn stale_catalog_plan_is_rejected_but_snapshot_recovery_stays_available() {
        let old_engine = engine();
        let original = normal_config();
        let state = old_engine
            .inspect_config(INSTALLATION_SCHEMA, &original)
            .unwrap();
        let plan = old_engine
            .plan_transition(&state, SoloProfile::Comfortable, 3)
            .unwrap();
        let applied = old_engine.apply_plan_to_config(&original, &plan).unwrap();

        let mut next_catalog = ProfileCatalog::realm_box_v1().unwrap();
        next_catalog.catalog_version = 2;
        let next_engine = SoloProfileEngine::new(INSTALLATION_SCHEMA, next_catalog).unwrap();
        assert_eq!(
            next_engine.apply_plan_to_config(&original, &plan),
            Err(SoloProfileError::CatalogChanged {
                expected: 1,
                found: 2,
            })
        );
        assert_eq!(
            next_engine
                .apply_snapshot_config(&original, &plan.snapshot)
                .unwrap(),
            applied.clone()
        );
        assert_eq!(
            next_engine
                .rollback_config(&applied.contents, &plan.snapshot)
                .unwrap()
                .outcome,
            MutationOutcome::Applied
        );
    }

    #[test]
    fn preserves_custom_in_bounds_values_in_the_rollback_snapshot() {
        let engine = engine();
        let custom = normal_config()
            .replace("Rate.XP.Kill      = 1", "Rate.XP.Kill = 1.5")
            .replace("MaxPrimaryTradeSkill = 2", "MaxPrimaryTradeSkill = 4");
        let state = engine.inspect_config(INSTALLATION_SCHEMA, &custom).unwrap();
        assert_eq!(state.active_profile, None);
        let plan = engine
            .plan_transition(&state, SoloProfile::Accelerated, 4)
            .unwrap();
        assert_eq!(plan.snapshot.previous_profile, None);
        assert_eq!(
            plan.snapshot.previous_values[&ManagedSetting::XpKill],
            SettingValue::RateMilli(1_500)
        );

        let applied = engine.apply_plan_to_config(&custom, &plan).unwrap();
        let restored = engine
            .rollback_config(&applied.contents, &plan.snapshot)
            .unwrap();
        let restored_state = engine
            .inspect_config(INSTALLATION_SCHEMA, &restored.contents)
            .unwrap();
        assert_eq!(restored_state, state);
    }

    #[test]
    fn refuses_tampered_plan_even_when_its_typed_diff_is_self_consistent() {
        let engine = engine();
        let original = normal_config();
        let state = engine
            .inspect_config(INSTALLATION_SCHEMA, &original)
            .unwrap();
        let mut plan = engine
            .plan_transition(&state, SoloProfile::Comfortable, 5)
            .unwrap();
        plan.snapshot
            .applied_values
            .insert(ManagedSetting::XpKill, SettingValue::RateMilli(1_500));
        plan.changes
            .iter_mut()
            .find(|change| change.setting == ManagedSetting::XpKill)
            .unwrap()
            .next = SettingValue::RateMilli(1_500);

        assert_eq!(
            engine.apply_plan_to_config(&original, &plan),
            Err(SoloProfileError::PlanTargetValuesMismatch)
        );
    }

    #[test]
    fn rejects_unknown_snapshot_and_state_schemas_and_unknown_json_fields() {
        let engine = engine();
        let state = engine
            .inspect_config(INSTALLATION_SCHEMA, &normal_config())
            .unwrap();
        let plan = engine
            .plan_transition(&state, SoloProfile::Comfortable, 6)
            .unwrap();

        let mut snapshot_json = serde_json::to_value(&plan.snapshot).unwrap();
        snapshot_json["schemaVersion"] = 99.into();
        assert_eq!(
            ProfileSnapshot::decode(&snapshot_json.to_string(), INSTALLATION_SCHEMA),
            Err(SoloProfileError::UnknownSnapshotSchema(99))
        );

        let mut state_json = serde_json::to_value(&state).unwrap();
        state_json["schemaVersion"] = 99.into();
        assert_eq!(
            ProfileState::decode(&state_json.to_string()),
            Err(SoloProfileError::UnknownStateSchema(99))
        );

        let mut snapshot_json = serde_json::to_value(&plan.snapshot).unwrap();
        snapshot_json["command"] = "DROP DATABASE characters".into();
        assert!(matches!(
            ProfileSnapshot::decode(&snapshot_json.to_string(), INSTALLATION_SCHEMA),
            Err(SoloProfileError::MalformedDocument(_))
        ));
    }

    #[test]
    fn refuses_missing_managed_keys_and_preserves_crlf_and_final_line() {
        let engine = engine();
        let missing = normal_config().replace("Instance.IgnoreRaid = 0\n", "");
        assert_eq!(
            engine.inspect_config(INSTALLATION_SCHEMA, &missing),
            Err(SoloProfileError::MissingManagedSetting(
                ManagedSetting::InstanceIgnoreRaid
            ))
        );

        let original = normal_config().trim_end_matches('\n').replace('\n', "\r\n");
        let state = engine
            .inspect_config(INSTALLATION_SCHEMA, &original)
            .unwrap();
        let plan = engine
            .plan_transition(&state, SoloProfile::Comfortable, 8)
            .unwrap();
        let applied = engine.apply_plan_to_config(&original, &plan).unwrap();
        assert!(!applied.contents.ends_with('\n'));
        assert!(applied.contents.contains("Rate.XP.Kill = 2\r\n"));
        assert!(applied.contents.ends_with("Unmanaged.Setting = 42"));
    }
}

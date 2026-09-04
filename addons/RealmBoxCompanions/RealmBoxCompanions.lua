local ADDON_NAME = "RealmBoxCompanions"
local MINIMAP_RADIUS = 80
local BEHAVIOR_REAPPLY_DELAY = 1.5
local BEHAVIOR_REAPPLY_TIMEOUT = 30
local FORMATION_CAPTURE_TIMEOUT = 30
local FORMATION_COMMAND_TIMEOUT = 30
local CONFIRMATION_TIMEOUT = 8

local COMMANDS = {
  follow = "follow",
  attack = "attack",
  stay = "stay",
  regroup = "summon",
  leave = "leave",
}

local BEHAVIOR_COMMANDS = {
  escort = "nc +follow,-stay,-new rpg,-grind",
  guard = "nc +stay,-follow,-new rpg,-grind",
  autonomous = "nc +new rpg,+grind,-follow,-stay",
}

-- Playerbots 2f7d9f774987d0157c6a0d0cc08c40bec3db3945:
-- Script/Playerbots.cpp routes WHISPER to the receiver's PlayerbotAI only.
-- ChatCommandHandlerStrategy.cpp registers follow/stay/co/nc. No free-text command.
local TARGET_ACTIONS = {
  follow = true,
  stay = true,
  escort = true,
  guard = true,
  autonomous = true,
  boost = true,
}

-- This allow-list only contains addclass commands already exercised by RealmBox.
-- Presets may repeat a verified class, but never construct a command from player input.
local VERIFIED_CLASS_COMMANDS = {
  PALADIN = ".playerbots bot addclass paladin",
  PRIEST = ".playerbots bot addclass priest",
  MAGE = ".playerbots bot addclass mage",
  HUNTER = ".playerbots bot addclass hunter",
}

local SQUAD_PRESETS = {
  balanced = {
    slots = {
      { classToken = "PALADIN", role = "tank" },
      { classToken = "PRIEST", role = "healer" },
      { classToken = "MAGE", role = "damage" },
      { classToken = "HUNTER", role = "damage" },
    },
  },
  arcane = {
    slots = {
      { classToken = "PALADIN", role = "tank" },
      { classToken = "PRIEST", role = "healer" },
      { classToken = "MAGE", role = "damage" },
      { classToken = "MAGE", role = "damage" },
    },
  },
  wilderness = {
    slots = {
      { classToken = "PALADIN", role = "tank" },
      { classToken = "PRIEST", role = "healer" },
      { classToken = "HUNTER", role = "damage" },
      { classToken = "HUNTER", role = "damage" },
    },
  },
}

local CLASS_NAMES = {
  fr = {
    DEATHKNIGHT = "Chevalier de la mort",
    DRUID = "Druide",
    HUNTER = "Chasseur",
    MAGE = "Mage",
    PALADIN = "Paladin",
    PRIEST = "Prêtre",
    ROGUE = "Voleur",
    SHAMAN = "Chaman",
    WARLOCK = "Démoniste",
    WARRIOR = "Guerrier",
  },
  en = {
    DEATHKNIGHT = "Death Knight",
    DRUID = "Druid",
    HUNTER = "Hunter",
    MAGE = "Mage",
    PALADIN = "Paladin",
    PRIEST = "Priest",
    ROGUE = "Rogue",
    SHAMAN = "Shaman",
    WARLOCK = "Warlock",
    WARRIOR = "Warrior",
  },
}

local STRINGS = {
  fr = {
    title = "COMPAGNONS",
    preset = "Équipe de 5 · intention des rôles",
    presetBalanced = "Polyvalente",
    presetArcane = "Arcanes",
    presetWilderness = "Pistage",
    presetSelected = "Préréglage conservé : %s",
    roleTank = "Tank",
    roleHealer = "Soin",
    roleDamage = "Dégâts",
    savedNamesEmpty = "Membres observés : aucun",
    savedNames = "Membres observés : %s",
    formParty = "Former mon équipe",
    follow = "Me suivre",
    attack = "Attaquer",
    stay = "Attendre ici",
    regroup = "Se regrouper",
    leave = "Libérer l'équipe",
    confirmLeave = "Confirmer la libération",
    behavior = "Comportement",
    behaviorEscort = "Escorte",
    behaviorGuard = "Garde",
    behaviorAutonomous = "Libres",
    behaviorHelp = "Applique une stratégie non-combat bornée. La sélection indique uniquement la dernière préférence envoyée, sans accusé du serveur.",
    behaviorSent = "Préférence envoyée : %s",
    behaviorReapplied = "Préférence réappliquée : %s",
    released = "Libération envoyée · autonomie puis départ du groupe",
    boostDefault = "Capacités fortes : serveur",
    boostOn = "Capacités fortes : demandées",
    boostOff = "Capacités fortes : limitées",
    ready = "Aventuriers autonomes actifs",
    groupEmpty = "Équipe : 1 joueur · 0/4 compagnon",
    groupState = "Équipe : 1 joueur + %d/4 · %s",
    offline = "%d hors ligne",
    noTarget = "Sélectionnez une cible ennemie vivante",
    noPartyTarget = "Ciblez d'abord un membre de votre groupe",
    noParty = "Formez d'abord une équipe",
    noConnectedParty = "Aucun membre du groupe n'est connecté",
    complete = "Votre groupe de cinq est déjà complet",
    forming = "Formation sûre · les membres existants restent dans le groupe",
    remaining = "Formation de l'équipe · %d restant(s)",
    regrouping = "Équipe demandée · regroupement en cours",
    formationPaused = "Formation suspendue pendant le combat",
    formationInProgress = "Attendez la fin de la formation en cours",
    formationExpired = "Formation expirée · les commandes restantes sont annulées",
    namesCaptured = "Composition observée et mémorisée",
    actionRefused = "RealmBox : action refusée",
    commandSent = "Ordre envoyé : %s",
    available = "Action disponible",
    unavailableCombat = "Indisponible en combat · aucune commande ne sera envoyée",
    groupScopeRequired = "Cette action concerne le groupe : choisissez Groupe",
    targetDispatchUnavailable = "Cette action est disponible uniquement pour le groupe",
    targetRequiresPrimary = "Ciblez le compagnon principal enregistré, connecté dans ce groupe",
    targetHelp = "Ordres privés bornés vers le compagnon principal ciblé et connecté. Aucune commande vers un nom saisi.",
    confirmationLeave = "Cliquez encore pour confirmer · la composition actuelle doit rester inchangée",
    confirmationExpired = "Confirmation expirée · aucune commande envoyée",
    scope = "Portée",
    scopeGroup = "Groupe",
    scopeTarget = "Cible",
    scopeSelected = "Portée sélectionnée : %s",
    primary = "Compagnon principal : %s",
    primaryNone = "Compagnon principal : aucun",
    primaryMissing = " (absent)",
    setPrimary = "Définir la cible principale",
    primarySaved = "Compagnon principal conservé : %s",
    preview = "Aperçu",
    previewEmpty = "Survolez une action pour voir la commande exacte",
    previewUnavailable = "Indisponible · %s",
    previewGroup = "Groupe · %s",
    previewTarget = "Cible %s · %s",
    previewLocal = "Local uniquement · aucune commande serveur",
    boostHelp = "Préférence envoyée à la stratégie Playerbots ; le serveur ne fournit pas d'accusé de réception.",
    boostRequestedOn = "Capacités fortes demandées",
    boostRequestedOff = "Capacités fortes limitées",
    tooltipTitle = "RealmBox Companions",
    tooltipToggle = "Clic gauche : afficher ou masquer",
    tooltipDrag = "Glisser : déplacer autour de la minimap",
    tooltipSlash = "/realmbox ou /rb",
    language = "EN",
  },
  en = {
    title = "COMPANIONS",
    preset = "Party of 5 · intended roles",
    presetBalanced = "Versatile",
    presetArcane = "Arcane",
    presetWilderness = "Tracking",
    presetSelected = "Saved preset: %s",
    roleTank = "Tank",
    roleHealer = "Heal",
    roleDamage = "Damage",
    savedNamesEmpty = "Observed members: none",
    savedNames = "Observed members: %s",
    formParty = "Build my party",
    follow = "Follow me",
    attack = "Attack",
    stay = "Stay here",
    regroup = "Regroup",
    leave = "Release party",
    confirmLeave = "Confirm release",
    behavior = "Behavior",
    behaviorEscort = "Escort",
    behaviorGuard = "Guard",
    behaviorAutonomous = "Free",
    behaviorHelp = "Applies one bounded non-combat strategy. The selection only shows the last preference sent, without server acknowledgement.",
    behaviorSent = "Preference sent: %s",
    behaviorReapplied = "Preference reapplied: %s",
    released = "Release sent · autonomy then leave the party",
    boostDefault = "Strong abilities: server",
    boostOn = "Strong abilities: requested",
    boostOff = "Strong abilities: limited",
    ready = "Autonomous adventurers active",
    groupEmpty = "Party: 1 player · 0/4 companion",
    groupState = "Party: 1 player + %d/4 · %s",
    offline = "%d offline",
    noTarget = "Select a living enemy target",
    noPartyTarget = "Target a member of your party first",
    noParty = "Build a party first",
    noConnectedParty = "No party member is connected",
    complete = "Your party of five is already full",
    forming = "Safe formation · existing members stay in the party",
    remaining = "Building party · %d remaining",
    regrouping = "Party requested · regrouping",
    formationPaused = "Party formation paused during combat",
    formationInProgress = "Wait for the current party formation to finish",
    formationExpired = "Party formation expired · remaining commands cancelled",
    namesCaptured = "Observed composition saved",
    actionRefused = "RealmBox: action refused",
    commandSent = "Command sent: %s",
    available = "Action available",
    unavailableCombat = "Unavailable in combat · no command will be sent",
    groupScopeRequired = "This action affects the party: select Party",
    targetDispatchUnavailable = "This action is available only for the party",
    targetRequiresPrimary = "Target your saved primary companion, connected in this party",
    targetHelp = "Bounded private commands to the targeted, connected primary companion. Never sends to a typed name.",
    confirmationLeave = "Click again to confirm · the current composition must remain unchanged",
    confirmationExpired = "Confirmation expired · no command sent",
    scope = "Scope",
    scopeGroup = "Party",
    scopeTarget = "Target",
    scopeSelected = "Selected scope: %s",
    primary = "Primary companion: %s",
    primaryNone = "Primary companion: none",
    primaryMissing = " (absent)",
    setPrimary = "Set targeted companion as primary",
    primarySaved = "Primary companion saved: %s",
    preview = "Preview",
    previewEmpty = "Hover over an action to see its exact command",
    previewUnavailable = "Unavailable · %s",
    previewGroup = "Party · %s",
    previewTarget = "Target %s · %s",
    previewLocal = "Local only · no server command",
    boostHelp = "Preference sent to the Playerbots strategy; the server does not provide an acknowledgement.",
    boostRequestedOn = "Strong abilities requested",
    boostRequestedOff = "Strong abilities limited",
    tooltipTitle = "RealmBox Companions",
    tooltipToggle = "Left-click: show or hide",
    tooltipDrag = "Drag: move around the minimap",
    tooltipSlash = "/realmbox or /rb",
    language = "FR",
  },
}

local partyQueue = {}
local partyQueueElapsed = 0
local partyQueueAge = 0
local initialized = false
local minimapDragging = false
local behaviorReapplyPending = false
local behaviorReapplyElapsed = 0
local behaviorReapplyAge = 0
local behaviorReapplyMinimumMembers = 1
local enteringWorldHandled = false
local formationCapturePending = false
local formationCaptureAge = 0
local formationCapturePreset = nil
local pendingConfirmation = nil
local previewAction = nil

local function CurrentLanguage()
  if RealmBoxCompanionsDB and RealmBoxCompanionsDB.language == "en" then
    return "en"
  end
  return "fr"
end

local function Text(key)
  return STRINGS[CurrentLanguage()][key]
end

local function SetStatus(message)
  if RealmBoxCompanionsFrameStatus then
    RealmBoxCompanionsFrameStatus:SetText(message)
  end
end

local function IsInCombat()
  if (UnitAffectingCombat and UnitAffectingCombat("player"))
      or (InCombatLockdown and InCombatLockdown()) then
    return true
  end
  if UnitAffectingCombat then
    for index = 1, GetNumPartyMembers() do
      if UnitAffectingCombat("party" .. index) then
        return true
      end
    end
  end
  return false
end

local function IsAttackableTarget()
  return UnitExists("target") and UnitCanAttack("player", "target") and not UnitIsDeadOrGhost("target")
end

local function SetButtonEnabled(button, enabled)
  if enabled then
    button:Enable()
  else
    button:Disable()
  end
end

local function PartySnapshot()
  local classCounts = {}
  local connectedClassTokens = {}
  local connectedCount = 0
  local offlineNames = {}
  local members = {}

  for index = 1, GetNumPartyMembers() do
    local unit = "party" .. index
    local name = UnitName(unit)
    if name then
      local _, classToken = UnitClass(unit)
      if classToken then
        classCounts[classToken] = (classCounts[classToken] or 0) + 1
      end
      local connected = UnitIsConnected(unit)
      table.insert(members, { name = name, classToken = classToken, connected = connected })
      if connected then
        if classToken then
          table.insert(connectedClassTokens, classToken)
        end
        connectedCount = connectedCount + 1
      else
        table.insert(offlineNames, name)
      end
    end
  end

  return classCounts, connectedClassTokens, connectedCount, offlineNames, members
end

local function CurrentTargetPartyMember()
  if not UnitExists("target") then
    return nil, nil
  end
  local targetName = UnitName("target")
  if not targetName then
    return nil, nil
  end
  for index = 1, GetNumPartyMembers() do
    local unit = "party" .. index
    if UnitName(unit) == targetName then
      local _, classToken = UnitClass(unit)
      return targetName, classToken, UnitIsConnected(unit)
    end
  end
  return nil, nil
end

local function TargetDispatchName()
  local name, _, connected = CurrentTargetPartyMember()
  if connected and name == RealmBoxCompanionsDB.primaryCompanionName then
    return name
  end
  return nil
end

local function CurrentBehaviorPreference()
  if RealmBoxCompanionsDB.commandScope == "target" then
    return RealmBoxCompanionsDB.primaryBehaviorPreference
  end
  return RealmBoxCompanionsDB.behaviorPreference
end

local function CurrentBoostPreference()
  if RealmBoxCompanionsDB.commandScope == "target" then
    return RealmBoxCompanionsDB.primaryBoostPreference
  end
  return RealmBoxCompanionsDB.boostPreference
end

local function DispatchCommand(command)
  if RealmBoxCompanionsDB.commandScope == "target" then
    local name = TargetDispatchName()
    if not name then
      return false
    end
    SendChatMessage(command, "WHISPER", nil, name)
  else
    SendChatMessage(command, "PARTY")
  end
  return true
end

local function PresetName(preset)
  if preset == "arcane" then
    return Text("presetArcane")
  end
  if preset == "wilderness" then
    return Text("presetWilderness")
  end
  return Text("presetBalanced")
end

local function RoleName(role)
  if role == "tank" then
    return Text("roleTank")
  end
  if role == "healer" then
    return Text("roleHealer")
  end
  return Text("roleDamage")
end

local function BehaviorText(behavior)
  behavior = behavior or CurrentBehaviorPreference()
  if behavior == "guard" then
    return Text("behaviorGuard")
  end
  if behavior == "autonomous" then
    return Text("behaviorAutonomous")
  end
  return Text("behaviorEscort")
end

local function SetButtonSelected(button, selected)
  if selected then
    button:LockHighlight()
  else
    button:UnlockHighlight()
  end
end

local function ActivePreset()
  return SQUAD_PRESETS[RealmBoxCompanionsDB.activePreset] or SQUAD_PRESETS.balanced
end

local function SaveActivePresetPreferences()
  RealmBoxCompanionsDB.presetPreferences[RealmBoxCompanionsDB.activePreset] = {
    behaviorPreference = RealmBoxCompanionsDB.behaviorPreference,
    boostPreference = RealmBoxCompanionsDB.boostPreference,
  }
end

local function LoadActivePresetPreferences()
  local saved = RealmBoxCompanionsDB.presetPreferences[RealmBoxCompanionsDB.activePreset] or {}
  RealmBoxCompanionsDB.behaviorPreference = saved.behaviorPreference
  RealmBoxCompanionsDB.boostPreference = saved.boostPreference
end

local function PresetSummary(preset)
  local parts = {}
  local selected = SQUAD_PRESETS[preset] or ActivePreset()
  for _, slot in ipairs(selected.slots) do
    local className = CLASS_NAMES[CurrentLanguage()][slot.classToken] or slot.classToken
    table.insert(parts, RoleName(slot.role) .. " " .. className)
  end
  return table.concat(parts, " · ")
end

local function BuildFormationQueue()
  local classCounts = PartySnapshot()
  local slotsRemaining = math.max(0, 4 - GetNumPartyMembers())
  local result = {}
  local availableClasses = {}
  for classToken, count in pairs(classCounts) do
    availableClasses[classToken] = count
  end
  for _, slot in ipairs(ActivePreset().slots) do
    if slotsRemaining == 0 then
      break
    end
    if (availableClasses[slot.classToken] or 0) > 0 then
      availableClasses[slot.classToken] = availableClasses[slot.classToken] - 1
    else
      table.insert(result, VERIFIED_CLASS_COMMANDS[slot.classToken])
      slotsRemaining = slotsRemaining - 1
    end
  end
  return result
end

local function PartySignature()
  local names = {}
  for index = 1, GetNumPartyMembers() do
    table.insert(names, UnitName("party" .. index) or "?")
  end
  table.sort(names)
  return table.concat(names, "|")
end

local function SavedNamesSummary()
  local saved = RealmBoxCompanionsDB.squadMembers[RealmBoxCompanionsDB.activePreset]
  if type(saved) ~= "table" or table.getn(saved) == 0 then
    return Text("savedNamesEmpty")
  end
  local names = {}
  for index = 1, math.min(4, table.getn(saved)) do
    local member = saved[index]
    local prefix = ""
    if member.name == RealmBoxCompanionsDB.primaryCompanionName then
      prefix = "★"
    end
    table.insert(names, prefix .. member.name)
  end
  return string.format(Text("savedNames"), table.concat(names, ", "))
end

local function PrimarySummary()
  local primary = RealmBoxCompanionsDB.primaryCompanionName
  if type(primary) ~= "string" or primary == "" then
    return Text("primaryNone")
  end
  local present = false
  for index = 1, GetNumPartyMembers() do
    if UnitName("party" .. index) == primary then
      present = true
      break
    end
  end
  if present then
    return string.format(Text("primary"), primary)
  end
  return string.format(Text("primary"), primary .. Text("primaryMissing"))
end

local function UpdatePanelPosition()
  if not RealmBoxCompanionsDB or not RealmBoxCompanionsFrame then
    return
  end
  local point, _, relativePoint, x, y = RealmBoxCompanionsFrame:GetPoint(1)
  if point and relativePoint and x and y then
    RealmBoxCompanionsDB.panelPoint = point
    RealmBoxCompanionsDB.panelRelativePoint = relativePoint
    RealmBoxCompanionsDB.panelX = x
    RealmBoxCompanionsDB.panelY = y
  end
end

local function PositionMinimapButton()
  if not RealmBoxCompanionsMinimapButton then
    return
  end
  local angle = 225
  if RealmBoxCompanionsDB and type(RealmBoxCompanionsDB.minimapAngle) == "number" then
    angle = RealmBoxCompanionsDB.minimapAngle
  end
  local radians = math.rad(angle)
  RealmBoxCompanionsMinimapButton:ClearAllPoints()
  RealmBoxCompanionsMinimapButton:SetPoint(
    "CENTER",
    Minimap,
    "CENTER",
    MINIMAP_RADIUS * math.cos(radians),
    MINIMAP_RADIUS * math.sin(radians)
  )
end

local function CursorAngleFromMinimap()
  local cursorX, cursorY = GetCursorPosition()
  local scale = Minimap:GetEffectiveScale()
  local centerX, centerY = Minimap:GetCenter()
  cursorX = cursorX / scale
  cursorY = cursorY / scale
  local deltaX = cursorX - centerX
  local deltaY = cursorY - centerY
  if deltaX == 0 then
    if deltaY >= 0 then
      return 90
    end
    return 270
  end
  local angle = math.deg(math.atan(deltaY / deltaX))
  if deltaX < 0 then
    angle = angle + 180
  elseif deltaY < 0 then
    angle = angle + 360
  end
  return angle
end

local function ApplyTranslations()
  RealmBoxCompanionsFrameTitle:SetText(Text("title"))
  RealmBoxCompanionsFramePresetLabel:SetText(Text("preset"))
  RealmBoxCompanionsFramePresetBalanced:SetText(Text("presetBalanced"))
  RealmBoxCompanionsFramePresetArcane:SetText(Text("presetArcane"))
  RealmBoxCompanionsFramePresetWilderness:SetText(Text("presetWilderness"))
  RealmBoxCompanionsFrameFormParty:SetText(Text("formParty"))
  RealmBoxCompanionsFrameScopeLabel:SetText(Text("scope"))
  RealmBoxCompanionsFrameScopeGroup:SetText(Text("scopeGroup"))
  RealmBoxCompanionsFrameScopeTarget:SetText(Text("scopeTarget"))
  RealmBoxCompanionsFrameSetPrimary:SetText(Text("setPrimary"))
  RealmBoxCompanionsFrameFollow:SetText(Text("follow"))
  RealmBoxCompanionsFrameAttack:SetText(Text("attack"))
  RealmBoxCompanionsFrameStay:SetText(Text("stay"))
  RealmBoxCompanionsFrameRegroup:SetText(Text("regroup"))
  RealmBoxCompanionsFrameBehaviorLabel:SetText(Text("behavior"))
  RealmBoxCompanionsFrameBehaviorEscort:SetText(Text("behaviorEscort"))
  RealmBoxCompanionsFrameBehaviorGuard:SetText(Text("behaviorGuard"))
  RealmBoxCompanionsFrameBehaviorFree:SetText(Text("behaviorAutonomous"))
  RealmBoxCompanionsFrameLeave:SetText(Text("leave"))
  RealmBoxCompanionsFramePreviewLabel:SetText(Text("preview"))
  RealmBoxCompanionsFrameLanguage:SetText(Text("language"))
end

local function ClearConfirmation(expired)
  if pendingConfirmation and expired then
    SetStatus(Text("confirmationExpired"))
  end
  pendingConfirmation = nil
  if RealmBoxCompanionsFrameLeave then
    RealmBoxCompanionsFrameLeave:SetText(Text("leave"))
  end
end

local function ConfirmationMatches(action)
  return pendingConfirmation
      and pendingConfirmation.action == action
      and pendingConfirmation.partySignature == PartySignature()
      and pendingConfirmation.scope == RealmBoxCompanionsDB.commandScope
end

local function RequestConfirmation(action)
  pendingConfirmation = {
    action = action,
    age = 0,
    partySignature = PartySignature(),
    scope = RealmBoxCompanionsDB.commandScope,
  }
  RealmBoxCompanionsFrameLeave:SetText(Text("confirmLeave"))
  SetStatus(Text("confirmationLeave"))
end

local function CancelBehaviorReapply()
  behaviorReapplyPending = false
  behaviorReapplyElapsed = 0
  behaviorReapplyAge = 0
  behaviorReapplyMinimumMembers = 1
end

local function ScheduleBehaviorReapply(minimumMembers)
  if not RealmBoxCompanionsDB or not BEHAVIOR_COMMANDS[RealmBoxCompanionsDB.behaviorPreference] then
    CancelBehaviorReapply()
    return
  end
  behaviorReapplyPending = true
  behaviorReapplyElapsed = 0
  behaviorReapplyAge = 0
  behaviorReapplyMinimumMembers = minimumMembers or 1
end

local function TryReapplyBehavior(elapsed)
  if not behaviorReapplyPending then
    return
  end
  behaviorReapplyAge = behaviorReapplyAge + elapsed
  if behaviorReapplyAge > BEHAVIOR_REAPPLY_TIMEOUT then
    CancelBehaviorReapply()
    return
  end
  if IsInCombat() or RealmBoxCompanionsDB.commandScope ~= "group" then
    behaviorReapplyElapsed = 0
    return
  end
  if table.getn(partyQueue) > 0 then
    return
  end
  local _, _, connectedCount, offlineNames = PartySnapshot()
  local partyCount = GetNumPartyMembers()
  if partyCount < behaviorReapplyMinimumMembers
      or connectedCount ~= partyCount
      or table.getn(offlineNames) > 0 then
    behaviorReapplyElapsed = 0
    return
  end
  behaviorReapplyElapsed = behaviorReapplyElapsed + elapsed
  if behaviorReapplyElapsed < BEHAVIOR_REAPPLY_DELAY then
    return
  end
  local behavior = RealmBoxCompanionsDB.behaviorPreference
  local command = BEHAVIOR_COMMANDS[behavior]
  CancelBehaviorReapply()
  if not command then
    return
  end
  SendChatMessage(command, "PARTY")
  SetStatus(string.format(Text("behaviorReapplied"), BehaviorText(behavior)))
end

local function TryCapturePresetMembers(elapsed)
  if not formationCapturePending then
    return
  end
  formationCaptureAge = formationCaptureAge + elapsed
  if formationCaptureAge > FORMATION_CAPTURE_TIMEOUT then
    formationCapturePending = false
    formationCaptureAge = 0
    return
  end
  if IsInCombat() then
    return
  end
  if table.getn(partyQueue) > 0 then
    return
  end
  local _, _, connectedCount, offlineNames, members = PartySnapshot()
  if GetNumPartyMembers() ~= 4 or connectedCount ~= 4 or table.getn(offlineNames) > 0 then
    return
  end
  local saved = {}
  for index = 1, math.min(4, table.getn(members)) do
    table.insert(saved, { name = members[index].name, classToken = members[index].classToken })
  end
  RealmBoxCompanionsDB.squadMembers[formationCapturePreset] = saved
  formationCapturePending = false
  formationCaptureAge = 0
  SetStatus(Text("namesCaptured"))
  return true
end

local function Availability(action)
  if IsInCombat() then
    return false, Text("unavailableCombat")
  end
  if RealmBoxCompanionsDB.commandScope ~= "group" then
    local targetName = CurrentTargetPartyMember()
    if not targetName then
      return false, Text("noPartyTarget")
    end
    if not TargetDispatchName() then
      return false, Text("targetRequiresPrimary")
    end
    if not TARGET_ACTIONS[action] then
      return false, Text("targetDispatchUnavailable")
    end
    return true, Text("available")
  end
  if GetNumPartyMembers() == 0 then
    return false, Text("noParty")
  end
  local _, _, connectedCount = PartySnapshot()
  if connectedCount == 0 then
    return false, Text("noConnectedParty")
  end
  if action == "attack" and not IsAttackableTarget() then
    return false, Text("noTarget")
  end
  return true, Text("available")
end

local function CommandForPreview(action)
  if COMMANDS[action] then
    if action == "leave" then
      return BEHAVIOR_COMMANDS.autonomous .. " ; " .. COMMANDS.leave
    end
    return COMMANDS[action]
  end
  if BEHAVIOR_COMMANDS[action] then
    return BEHAVIOR_COMMANDS[action]
  end
  if action == "boost" then
    if CurrentBoostPreference() == true then
      return "co -boost"
    end
    return "co +boost"
  end
  if action == "form" then
    local commands = table.getn(partyQueue) > 0 and partyQueue or BuildFormationQueue()
    return table.concat(commands, " ; ")
  end
  return nil
end

local function SetCommandPreview(action)
  previewAction = action
  if action == "primary" or action == "preset" or action == "scope" then
    RealmBoxCompanionsFramePreview:SetText(Text("previewLocal"))
    return
  end
  local command = CommandForPreview(action)
  if not command then
    RealmBoxCompanionsFramePreview:SetText(Text("previewEmpty"))
    return
  end
  if action == "form" then
    if IsInCombat() then
      RealmBoxCompanionsFramePreview:SetText(string.format(Text("previewUnavailable"), Text("unavailableCombat")))
    elseif RealmBoxCompanionsDB.commandScope ~= "group" then
      RealmBoxCompanionsFramePreview:SetText(string.format(Text("previewUnavailable"), Text("groupScopeRequired")))
    elseif GetNumPartyMembers() >= 4 then
      RealmBoxCompanionsFramePreview:SetText(string.format(Text("previewUnavailable"), Text("complete")))
    else
      RealmBoxCompanionsFramePreview:SetText(string.format(Text("previewGroup"), command))
    end
    return
  end
  local available, reason = Availability(action)
  if not available then
    RealmBoxCompanionsFramePreview:SetText(string.format(Text("previewUnavailable"), reason))
  elseif RealmBoxCompanionsDB.commandScope == "target" then
    RealmBoxCompanionsFramePreview:SetText(string.format(Text("previewTarget"), TargetDispatchName(), command))
  else
    RealmBoxCompanionsFramePreview:SetText(string.format(Text("previewGroup"), command))
  end
end

local function UpdateGroupState()
  if not initialized then
    return
  end
  local _, connectedClassTokens, connectedCount, offlineNames = PartySnapshot()
  local classNames = {}
  for _, classToken in ipairs(connectedClassTokens) do
    table.insert(classNames, CLASS_NAMES[CurrentLanguage()][classToken] or classToken)
  end
  if connectedCount == 0 and table.getn(offlineNames) == 0 then
    RealmBoxCompanionsFrameGroupStatus:SetText(Text("groupEmpty"))
  else
    local details = table.concat(classNames, ", ")
    if details == "" then
      details = "—"
    end
    if table.getn(offlineNames) > 0 then
      details = details .. " · " .. string.format(Text("offline"), table.getn(offlineNames))
    end
    RealmBoxCompanionsFrameGroupStatus:SetText(string.format(Text("groupState"), connectedCount, details))
  end

  RealmBoxCompanionsFramePresetSummary:SetText(PresetSummary())
  RealmBoxCompanionsFrameSavedNames:SetText(SavedNamesSummary())
  RealmBoxCompanionsFramePrimary:SetText(PrimarySummary())
  local inCombat = IsInCombat()
  local formationBusy = table.getn(partyQueue) > 0
  SetButtonEnabled(RealmBoxCompanionsFramePresetBalanced, not inCombat and not formationBusy)
  SetButtonEnabled(RealmBoxCompanionsFramePresetArcane, not inCombat and not formationBusy)
  SetButtonEnabled(RealmBoxCompanionsFramePresetWilderness, not inCombat and not formationBusy)
  SetButtonSelected(RealmBoxCompanionsFramePresetBalanced, RealmBoxCompanionsDB.activePreset == "balanced")
  SetButtonSelected(RealmBoxCompanionsFramePresetArcane, RealmBoxCompanionsDB.activePreset == "arcane")
  SetButtonSelected(RealmBoxCompanionsFramePresetWilderness, RealmBoxCompanionsDB.activePreset == "wilderness")
  SetButtonEnabled(RealmBoxCompanionsFrameScopeGroup, not inCombat and not formationBusy)
  SetButtonEnabled(RealmBoxCompanionsFrameScopeTarget, not inCombat and not formationBusy)
  SetButtonSelected(RealmBoxCompanionsFrameScopeGroup, RealmBoxCompanionsDB.commandScope == "group")
  SetButtonSelected(RealmBoxCompanionsFrameScopeTarget, RealmBoxCompanionsDB.commandScope == "target")
  SetButtonEnabled(
    RealmBoxCompanionsFrameFormParty,
    not inCombat and not formationBusy and RealmBoxCompanionsDB.commandScope == "group" and GetNumPartyMembers() < 4
  )
  local targetName = CurrentTargetPartyMember()
  SetButtonEnabled(RealmBoxCompanionsFrameSetPrimary, not inCombat and targetName ~= nil)

  local actions = { "follow", "attack", "stay", "regroup", "escort", "guard", "autonomous", "boost", "leave" }
  local buttons = {
    RealmBoxCompanionsFrameFollow,
    RealmBoxCompanionsFrameAttack,
    RealmBoxCompanionsFrameStay,
    RealmBoxCompanionsFrameRegroup,
    RealmBoxCompanionsFrameBehaviorEscort,
    RealmBoxCompanionsFrameBehaviorGuard,
    RealmBoxCompanionsFrameBehaviorFree,
    RealmBoxCompanionsFrameBoost,
    RealmBoxCompanionsFrameLeave,
  }
  for index, action in ipairs(actions) do
    local available = Availability(action)
    SetButtonEnabled(buttons[index], available)
  end
  local behavior = CurrentBehaviorPreference()
  SetButtonSelected(RealmBoxCompanionsFrameBehaviorEscort, behavior == "escort")
  SetButtonSelected(RealmBoxCompanionsFrameBehaviorGuard, behavior == "guard")
  SetButtonSelected(RealmBoxCompanionsFrameBehaviorFree, behavior == "autonomous")
  if CurrentBoostPreference() == true then
    RealmBoxCompanionsFrameBoost:SetText(Text("boostOn"))
  elseif CurrentBoostPreference() == false then
    RealmBoxCompanionsFrameBoost:SetText(Text("boostOff"))
  else
    RealmBoxCompanionsFrameBoost:SetText(Text("boostDefault"))
  end
  if ConfirmationMatches("leave") then
    RealmBoxCompanionsFrameLeave:SetText(Text("confirmLeave"))
  else
    RealmBoxCompanionsFrameLeave:SetText(Text("leave"))
  end
  if previewAction then
    SetCommandPreview(previewAction)
  end
end

local function RestorePanelPosition()
  RealmBoxCompanionsFrame:ClearAllPoints()
  if type(RealmBoxCompanionsDB.panelPoint) == "string"
      and type(RealmBoxCompanionsDB.panelRelativePoint) == "string"
      and type(RealmBoxCompanionsDB.panelX) == "number"
      and type(RealmBoxCompanionsDB.panelY) == "number" then
    RealmBoxCompanionsFrame:SetPoint(
      RealmBoxCompanionsDB.panelPoint,
      UIParent,
      RealmBoxCompanionsDB.panelRelativePoint,
      RealmBoxCompanionsDB.panelX,
      RealmBoxCompanionsDB.panelY
    )
  else
    RealmBoxCompanionsFrame:SetPoint("CENTER", UIParent, "CENTER", 330, 0)
  end
end

local function SanitizeSavedMembers()
  if type(RealmBoxCompanionsDB.squadMembers) ~= "table" then
    RealmBoxCompanionsDB.squadMembers = {}
    return
  end
  for preset in pairs(SQUAD_PRESETS) do
    local source = RealmBoxCompanionsDB.squadMembers[preset]
    local clean = {}
    if type(source) == "table" then
      for index = 1, math.min(4, table.getn(source)) do
        local member = source[index]
        if type(member) == "table" and type(member.name) == "string" and member.name ~= "" then
          table.insert(clean, {
            name = string.sub(member.name, 1, 48),
            classToken = type(member.classToken) == "string" and string.sub(member.classToken, 1, 24) or nil,
          })
        end
      end
    end
    RealmBoxCompanionsDB.squadMembers[preset] = clean
  end
end

local function Initialize()
  local firstRun = type(RealmBoxCompanionsDB) ~= "table" or not RealmBoxCompanionsDB.seen
  if type(RealmBoxCompanionsDB) ~= "table" then
    RealmBoxCompanionsDB = {}
  end
  if RealmBoxCompanionsDB.language ~= "fr" and RealmBoxCompanionsDB.language ~= "en" then
    RealmBoxCompanionsDB.language = GetLocale and string.sub(GetLocale(), 1, 2) == "fr" and "fr" or "en"
  end
  if type(RealmBoxCompanionsDB.minimapAngle) ~= "number" then
    RealmBoxCompanionsDB.minimapAngle = 225
  end
  if RealmBoxCompanionsDB.behaviorPreference ~= nil
      and not BEHAVIOR_COMMANDS[RealmBoxCompanionsDB.behaviorPreference] then
    RealmBoxCompanionsDB.behaviorPreference = nil
  end
  if RealmBoxCompanionsDB.primaryBehaviorPreference ~= nil
      and not BEHAVIOR_COMMANDS[RealmBoxCompanionsDB.primaryBehaviorPreference] then
    RealmBoxCompanionsDB.primaryBehaviorPreference = nil
  end
  if type(RealmBoxCompanionsDB.boostPreference) ~= "boolean" then
    RealmBoxCompanionsDB.boostPreference = nil
  end
  if type(RealmBoxCompanionsDB.primaryBoostPreference) ~= "boolean" then
    RealmBoxCompanionsDB.primaryBoostPreference = nil
  end
  if not SQUAD_PRESETS[RealmBoxCompanionsDB.activePreset] then
    RealmBoxCompanionsDB.activePreset = "balanced"
  end
  if type(RealmBoxCompanionsDB.presetPreferences) ~= "table" then
    RealmBoxCompanionsDB.presetPreferences = {}
  end
  for preset in pairs(SQUAD_PRESETS) do
    local saved = RealmBoxCompanionsDB.presetPreferences[preset]
    if type(saved) == "table" then
      if not BEHAVIOR_COMMANDS[saved.behaviorPreference] then
        saved.behaviorPreference = nil
      end
      if type(saved.boostPreference) ~= "boolean" then
        saved.boostPreference = nil
      end
    else
      RealmBoxCompanionsDB.presetPreferences[preset] = nil
    end
  end
  if RealmBoxCompanionsDB.presetPreferences[RealmBoxCompanionsDB.activePreset] then
    LoadActivePresetPreferences()
  else
    SaveActivePresetPreferences()
  end
  if RealmBoxCompanionsDB.commandScope ~= "target" then
    RealmBoxCompanionsDB.commandScope = "group"
  end
  if type(RealmBoxCompanionsDB.primaryCompanionName) ~= "string" or RealmBoxCompanionsDB.primaryCompanionName == "" then
    RealmBoxCompanionsDB.primaryCompanionName = nil
    RealmBoxCompanionsDB.primaryBehaviorPreference = nil
    RealmBoxCompanionsDB.primaryBoostPreference = nil
  else
    RealmBoxCompanionsDB.primaryCompanionName = string.sub(RealmBoxCompanionsDB.primaryCompanionName, 1, 48)
  end
  SanitizeSavedMembers()
  if firstRun then
    RealmBoxCompanionsDB.panelShown = true
  end
  RealmBoxCompanionsDB.seen = true

  initialized = true
  ApplyTranslations()
  RestorePanelPosition()
  PositionMinimapButton()
  RealmBoxCompanionsFramePreview:SetText(Text("previewEmpty"))
  UpdateGroupState()
  SetStatus(Text("ready"))
  if RealmBoxCompanionsDB.panelShown then
    RealmBoxCompanionsFrame:Show()
  else
    RealmBoxCompanionsFrame:Hide()
  end
end

function RealmBoxCompanions_OnLoad(frame)
  frame:RegisterForDrag("LeftButton")
  frame:SetClampedToScreen(true)
  frame:SetBackdropColor(0.04, 0.05, 0.04, 0.94)
  frame:RegisterEvent("ADDON_LOADED")
  frame:RegisterEvent("PARTY_MEMBERS_CHANGED")
  frame:RegisterEvent("PLAYER_ENTERING_WORLD")
  frame:RegisterEvent("PLAYER_TARGET_CHANGED")
  frame:RegisterEvent("PLAYER_REGEN_DISABLED")
  frame:RegisterEvent("PLAYER_REGEN_ENABLED")
  frame:RegisterEvent("UNIT_FLAGS")
  table.insert(UISpecialFrames, "RealmBoxCompanionsFrame")
end

function RealmBoxCompanions_OnEvent(frame, event, argument)
  if event == "ADDON_LOADED" and argument == ADDON_NAME then
    Initialize()
    return
  end
  if not initialized then
    return
  end
  if event == "PLAYER_ENTERING_WORLD" and not enteringWorldHandled then
    enteringWorldHandled = true
    ScheduleBehaviorReapply(1)
  elseif event == "PARTY_MEMBERS_CHANGED" then
    ClearConfirmation(false)
    if behaviorReapplyPending then
      behaviorReapplyElapsed = 0
    end
  elseif event == "PLAYER_REGEN_DISABLED" or (event == "UNIT_FLAGS" and IsInCombat()) then
    ClearConfirmation(false)
    behaviorReapplyElapsed = 0
    SetStatus(Text("unavailableCombat"))
  elseif event == "PLAYER_REGEN_ENABLED" and table.getn(partyQueue) > 0 then
    SetStatus(string.format(Text("remaining"), table.getn(partyQueue)))
  end
  UpdateGroupState()
end

function RealmBoxCompanions_OnUpdate(frame, elapsed)
  if pendingConfirmation then
    pendingConfirmation.age = pendingConfirmation.age + elapsed
    if pendingConfirmation.age > CONFIRMATION_TIMEOUT then
      ClearConfirmation(true)
      UpdateGroupState()
    end
  end
  if table.getn(partyQueue) > 0 then
    partyQueueAge = partyQueueAge + elapsed
    if partyQueueAge > FORMATION_COMMAND_TIMEOUT then
      partyQueue = {}
      formationCapturePending = false
      CancelBehaviorReapply()
      SetStatus(Text("formationExpired"))
      UpdateGroupState()
      return
    end
    if IsInCombat() then
      SetStatus(Text("formationPaused"))
    elseif GetNumPartyMembers() >= 4 then
      partyQueue = {}
      SetStatus(Text("complete"))
      UpdateGroupState()
    else
      partyQueueElapsed = partyQueueElapsed + elapsed
      if partyQueueElapsed >= 0.8 then
        partyQueueElapsed = 0
        local command = table.remove(partyQueue, 1)
        SendChatMessage(command, "SAY")
        if table.getn(partyQueue) == 0 then
          SetStatus(Text("regrouping"))
        else
          SetStatus(string.format(Text("remaining"), table.getn(partyQueue)))
        end
        UpdateGroupState()
      end
    end
  end
  if TryCapturePresetMembers(elapsed) then
    UpdateGroupState()
  end
  TryReapplyBehavior(elapsed)
end

function RealmBoxCompanions_OnShow()
  if initialized then
    RealmBoxCompanionsDB.panelShown = true
    UpdateGroupState()
  end
end

function RealmBoxCompanions_OnHide()
  if initialized then
    RealmBoxCompanionsDB.panelShown = false
  end
end

function RealmBoxCompanions_OnDragStop(frame)
  frame:StopMovingOrSizing()
  UpdatePanelPosition()
end

function RealmBoxCompanions_Toggle()
  if RealmBoxCompanionsFrame:IsShown() then
    RealmBoxCompanionsFrame:Hide()
  else
    RealmBoxCompanionsFrame:Show()
  end
end

function RealmBoxCompanions_ToggleLanguage()
  RealmBoxCompanionsDB.language = CurrentLanguage() == "fr" and "en" or "fr"
  previewAction = nil
  ApplyTranslations()
  UpdateGroupState()
  RealmBoxCompanionsFramePreview:SetText(Text("previewEmpty"))
  SetStatus(Text("ready"))
end

function RealmBoxCompanions_SelectPreset(preset)
  if not SQUAD_PRESETS[preset] then
    DEFAULT_CHAT_FRAME:AddMessage(Text("actionRefused"))
    return
  end
  if IsInCombat() then
    SetStatus(Text("unavailableCombat"))
    return
  end
  if table.getn(partyQueue) > 0 then
    SetStatus(Text("formationInProgress"))
    return
  end
  ClearConfirmation(false)
  CancelBehaviorReapply()
  formationCapturePending = false
  SaveActivePresetPreferences()
  RealmBoxCompanionsDB.activePreset = preset
  LoadActivePresetPreferences()
  SetStatus(string.format(Text("presetSelected"), PresetName(preset)))
  SetCommandPreview("preset")
  UpdateGroupState()
end

function RealmBoxCompanions_SelectScope(scope)
  if scope ~= "group" and scope ~= "target" then
    DEFAULT_CHAT_FRAME:AddMessage(Text("actionRefused"))
    return
  end
  if IsInCombat() then
    SetStatus(Text("unavailableCombat"))
    return
  end
  if table.getn(partyQueue) > 0 then
    SetStatus(Text("formationInProgress"))
    return
  end
  ClearConfirmation(false)
  CancelBehaviorReapply()
  RealmBoxCompanionsDB.commandScope = scope
  local scopeName = scope == "group" and Text("scopeGroup") or Text("scopeTarget")
  SetStatus(string.format(Text("scopeSelected"), scopeName))
  SetCommandPreview("scope")
  UpdateGroupState()
end

function RealmBoxCompanions_SetPrimary()
  if IsInCombat() then
    SetStatus(Text("unavailableCombat"))
    return
  end
  local name, classToken = CurrentTargetPartyMember()
  if not name then
    SetStatus(Text("noPartyTarget"))
    return
  end
  if RealmBoxCompanionsDB.primaryCompanionName ~= name then
    RealmBoxCompanionsDB.primaryBehaviorPreference = nil
    RealmBoxCompanionsDB.primaryBoostPreference = nil
  end
  RealmBoxCompanionsDB.primaryCompanionName = string.sub(name, 1, 48)
  RealmBoxCompanionsDB.primaryCompanionClassToken = classToken
  SetStatus(string.format(Text("primarySaved"), name))
  SetCommandPreview("primary")
  UpdateGroupState()
end

function RealmBoxCompanions_FormParty()
  if IsInCombat() then
    SetStatus(Text("unavailableCombat"))
    return
  end
  if RealmBoxCompanionsDB.commandScope ~= "group" then
    SetStatus(Text("groupScopeRequired"))
    return
  end
  if table.getn(partyQueue) > 0 then
    SetStatus(Text("formationInProgress"))
    return
  end
  local partyCount = GetNumPartyMembers()
  if partyCount >= 4 then
    ScheduleBehaviorReapply(4)
    SetStatus(Text("complete"))
    return
  end
  partyQueue = BuildFormationQueue()
  if table.getn(partyQueue) == 0 then
    ScheduleBehaviorReapply(4)
    SetStatus(Text("complete"))
    return
  end
  ScheduleBehaviorReapply(4)
  formationCapturePending = true
  formationCaptureAge = 0
  formationCapturePreset = RealmBoxCompanionsDB.activePreset
  partyQueueElapsed = 0.8
  partyQueueAge = 0
  SetStatus(Text("forming"))
  SetCommandPreview("form")
  UpdateGroupState()
end

function RealmBoxCompanions_Run(action)
  local command = COMMANDS[action]
  if not command then
    DEFAULT_CHAT_FRAME:AddMessage(Text("actionRefused"))
    return
  end
  local available, reason = Availability(action)
  if not available then
    if IsInCombat() then
      ClearConfirmation(false)
    end
    SetStatus(reason)
    return
  end
  if action == "leave" then
    if not ConfirmationMatches("leave") then
      RequestConfirmation("leave")
      SetCommandPreview("leave")
      return
    end
    ClearConfirmation(false)
    CancelBehaviorReapply()
    formationCapturePending = false
    partyQueue = {}
    RealmBoxCompanionsDB.behaviorPreference = "autonomous"
    SaveActivePresetPreferences()
    SendChatMessage(BEHAVIOR_COMMANDS.autonomous, "PARTY")
    SendChatMessage(command, "PARTY")
    SetStatus(Text("released"))
    UpdateGroupState()
    return
  end
  DispatchCommand(command)
  SetStatus(string.format(Text("commandSent"), Text(action)))
  SetCommandPreview(action)
  UpdateGroupState()
end

function RealmBoxCompanions_SetBehavior(behavior)
  local command = BEHAVIOR_COMMANDS[behavior]
  if not command then
    DEFAULT_CHAT_FRAME:AddMessage(Text("actionRefused"))
    return
  end
  local available, reason = Availability(behavior)
  if not available then
    SetStatus(reason)
    return
  end
  CancelBehaviorReapply()
  if RealmBoxCompanionsDB.commandScope == "target" then
    RealmBoxCompanionsDB.primaryBehaviorPreference = behavior
  else
    RealmBoxCompanionsDB.behaviorPreference = behavior
    SaveActivePresetPreferences()
  end
  DispatchCommand(command)
  if RealmBoxCompanionsDB.commandScope == "group" and table.getn(partyQueue) > 0 then
    ScheduleBehaviorReapply(4)
  end
  SetStatus(string.format(Text("behaviorSent"), BehaviorText(behavior)))
  SetCommandPreview(behavior)
  UpdateGroupState()
end

function RealmBoxCompanions_CycleBehavior()
  local current = CurrentBehaviorPreference()
  local nextBehavior = "escort"
  if current == "escort" or current == nil then
    nextBehavior = "guard"
  elseif current == "guard" then
    nextBehavior = "autonomous"
  end
  RealmBoxCompanions_SetBehavior(nextBehavior)
end

function RealmBoxCompanions_ToggleBoost()
  local available, reason = Availability("boost")
  if not available then
    SetStatus(reason)
    return
  end
  local nextPreference = CurrentBoostPreference() ~= true
  if RealmBoxCompanionsDB.commandScope == "target" then
    RealmBoxCompanionsDB.primaryBoostPreference = nextPreference
  else
    RealmBoxCompanionsDB.boostPreference = nextPreference
    SaveActivePresetPreferences()
  end
  if nextPreference then
    DispatchCommand("co +boost")
    SetStatus(Text("boostRequestedOn"))
  else
    DispatchCommand("co -boost")
    SetStatus(Text("boostRequestedOff"))
  end
  SetCommandPreview("boost")
  UpdateGroupState()
end

function RealmBoxCompanions_Minimap_OnLoad(button)
  button:RegisterForClicks("LeftButtonUp")
  button:RegisterForDrag("LeftButton")
  PositionMinimapButton()
end

function RealmBoxCompanions_Minimap_OnDragStart(button)
  minimapDragging = true
  button:LockHighlight()
end

function RealmBoxCompanions_Minimap_OnUpdate()
  if not minimapDragging then
    return
  end
  RealmBoxCompanionsDB.minimapAngle = CursorAngleFromMinimap()
  PositionMinimapButton()
end

function RealmBoxCompanions_Minimap_OnDragStop(button)
  minimapDragging = false
  button:UnlockHighlight()
end

function RealmBoxCompanions_Minimap_OnEnter(button)
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(Text("tooltipTitle"))
  GameTooltip:AddLine(Text("tooltipToggle"), 1, 1, 1)
  GameTooltip:AddLine(Text("tooltipDrag"), 0.8, 0.8, 0.8)
  GameTooltip:AddLine(Text("tooltipSlash"), 0.8, 0.8, 0.8)
  GameTooltip:Show()
end

function RealmBoxCompanions_Action_OnEnter(button, action)
  SetCommandPreview(action)
  local title = Text(action)
  if action == "boost" then
    if CurrentBoostPreference() == true then
      title = Text("boostOn")
    elseif CurrentBoostPreference() == false then
      title = Text("boostOff")
    else
      title = Text("boostDefault")
    end
  elseif action == "form" then
    title = Text("formParty")
  end
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(title)
  local available, reason = Availability(action)
  if action == "form" then
    available = not IsInCombat()
        and RealmBoxCompanionsDB.commandScope == "group"
        and table.getn(partyQueue) == 0
        and GetNumPartyMembers() < 4
    if not available then
      if IsInCombat() then
        reason = Text("unavailableCombat")
      elseif RealmBoxCompanionsDB.commandScope ~= "group" then
        reason = Text("groupScopeRequired")
      elseif GetNumPartyMembers() >= 4 then
        reason = Text("complete")
      else
        reason = Text("formationInProgress")
      end
    end
  end
  if not available then
    GameTooltip:AddLine(reason, 1, 0.35, 0.35)
  elseif action == "boost" then
    GameTooltip:AddLine(Text("boostHelp"), 0.8, 0.8, 0.8, true)
  else
    GameTooltip:AddLine(Text("available"), 0.8, 0.8, 0.8)
  end
  GameTooltip:Show()
end

function RealmBoxCompanions_Behavior_OnEnter(button, behavior)
  SetCommandPreview(behavior)
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(BehaviorText(behavior))
  local available, reason = Availability(behavior)
  if not available then
    GameTooltip:AddLine(reason, 1, 0.35, 0.35)
  else
    GameTooltip:AddLine(Text("behaviorHelp"), 0.8, 0.8, 0.8, true)
  end
  GameTooltip:Show()
end

function RealmBoxCompanions_Preset_OnEnter(button, preset)
  SetCommandPreview("preset")
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(PresetName(preset))
  GameTooltip:AddLine(PresetSummary(preset), 0.8, 0.8, 0.8, true)
  if IsInCombat() then
    GameTooltip:AddLine(Text("unavailableCombat"), 1, 0.35, 0.35)
  end
  GameTooltip:Show()
end

function RealmBoxCompanions_Scope_OnEnter(button, scope)
  SetCommandPreview("scope")
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(scope == "group" and Text("scopeGroup") or Text("scopeTarget"))
  if scope == "target" then
    GameTooltip:AddLine(Text("targetHelp"), 0.8, 0.8, 0.8, true)
  else
    GameTooltip:AddLine(Text("available"), 0.8, 0.8, 0.8)
  end
  GameTooltip:Show()
end

function RealmBoxCompanions_Primary_OnEnter(button)
  SetCommandPreview("primary")
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(Text("setPrimary"))
  local targetName = CurrentTargetPartyMember()
  if IsInCombat() then
    GameTooltip:AddLine(Text("unavailableCombat"), 1, 0.35, 0.35)
  elseif not targetName then
    GameTooltip:AddLine(Text("noPartyTarget"), 1, 0.35, 0.35)
  else
    GameTooltip:AddLine(Text("previewLocal"), 0.8, 0.8, 0.8)
  end
  GameTooltip:Show()
end

SLASH_REALMBOXCOMPANIONS1 = "/realmbox"
SLASH_REALMBOXCOMPANIONS2 = "/rb"
SlashCmdList.REALMBOXCOMPANIONS = function(message)
  local command = string.lower(message or "")
  if command == "fr" or command == "en" then
    RealmBoxCompanionsDB.language = command
    previewAction = nil
    ApplyTranslations()
    UpdateGroupState()
    RealmBoxCompanionsFramePreview:SetText(Text("previewEmpty"))
    SetStatus(Text("ready"))
    RealmBoxCompanionsFrame:Show()
    return
  end
  RealmBoxCompanions_Toggle()
end

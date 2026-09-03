local ADDON_NAME = "RealmBoxCompanions"
local MINIMAP_RADIUS = 80
local BEHAVIOR_REAPPLY_DELAY = 1.5
local BEHAVIOR_REAPPLY_TIMEOUT = 30

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

local PARTY_TEMPLATE = {
  { classToken = "PALADIN", command = ".playerbots bot addclass paladin" },
  { classToken = "PRIEST", command = ".playerbots bot addclass priest" },
  { classToken = "MAGE", command = ".playerbots bot addclass mage" },
  { classToken = "HUNTER", command = ".playerbots bot addclass hunter" },
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
    formParty = "Former mon équipe",
    follow = "Me suivre",
    attack = "Attaquer",
    stay = "Attendre ici",
    regroup = "Se regrouper",
    leave = "Libérer l'équipe",
    behavior = "Comportement",
    behaviorEscort = "Escorte",
    behaviorGuard = "Garde",
    behaviorAutonomous = "Libres",
    behaviorHelp = "Applique une stratégie non-combat bornée. La sélection indique uniquement la dernière préférence envoyée, sans accusé du serveur.",
    behaviorSent = "Préférence envoyée : %s",
    behaviorReapplied = "Préférence réappliquée : %s",
    released = "Équipe libérée · les bots reprennent leurs activités",
    boostDefault = "Capacités fortes : serveur",
    boostOn = "Capacités fortes : demandées",
    boostOff = "Capacités fortes : limitées",
    ready = "Aventuriers autonomes actifs",
    groupEmpty = "Équipe : 0/4",
    groupState = "Équipe : %d/4 · %s",
    offline = "%d hors ligne",
    noTarget = "Sélectionnez une cible ennemie",
    noParty = "Formez d'abord une équipe",
    complete = "Votre groupe est déjà complet",
    reconnecting = "Reconnexion des compagnons hors ligne…",
    forming = "Formation d'une équipe équilibrée…",
    remaining = "Formation de l'équipe · %d restant(s)",
    regrouping = "Équipe demandée · regroupement en cours",
    actionRefused = "RealmBox : action refusée",
    commandSent = "Ordre envoyé : %s",
    available = "Action disponible",
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
    formParty = "Build my party",
    follow = "Follow me",
    attack = "Attack",
    stay = "Stay here",
    regroup = "Regroup",
    leave = "Release party",
    behavior = "Behavior",
    behaviorEscort = "Escort",
    behaviorGuard = "Guard",
    behaviorAutonomous = "Free",
    behaviorHelp = "Applies one bounded non-combat strategy. The selection only shows the last preference sent, without server acknowledgement.",
    behaviorSent = "Preference sent: %s",
    behaviorReapplied = "Preference reapplied: %s",
    released = "Party released · bots resume their activities",
    boostDefault = "Strong abilities: server",
    boostOn = "Strong abilities: requested",
    boostOff = "Strong abilities: limited",
    ready = "Autonomous adventurers active",
    groupEmpty = "Party: 0/4",
    groupState = "Party: %d/4 · %s",
    offline = "%d offline",
    noTarget = "Select an enemy target",
    noParty = "Build a party first",
    complete = "Your party is already full",
    reconnecting = "Replacing offline companions…",
    forming = "Building a balanced party…",
    remaining = "Building party · %d remaining",
    regrouping = "Party requested · regrouping",
    actionRefused = "RealmBox: action refused",
    commandSent = "Command sent: %s",
    available = "Action available",
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
local initialized = false
local minimapDragging = false
local behaviorReapplyPending = false
local behaviorReapplyElapsed = 0
local behaviorReapplyAge = 0
local behaviorReapplyMinimumMembers = 1
local enteringWorldHandled = false

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
  local connectedClasses = {}
  local connectedClassTokens = {}
  local connectedCount = 0
  local offlineNames = {}

  for index = 1, GetNumPartyMembers() do
    local unit = "party" .. index
    local name = UnitName(unit)
    if name then
      if UnitIsConnected(unit) then
        local _, classToken = UnitClass(unit)
        if classToken then
          connectedClasses[classToken] = true
          table.insert(connectedClassTokens, classToken)
        end
        connectedCount = connectedCount + 1
      else
        table.insert(offlineNames, name)
      end
    end
  end

  return connectedClasses, connectedClassTokens, connectedCount, offlineNames
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
  RealmBoxCompanionsFrameFormParty:SetText(Text("formParty"))
  RealmBoxCompanionsFrameFollow:SetText(Text("follow"))
  RealmBoxCompanionsFrameAttack:SetText(Text("attack"))
  RealmBoxCompanionsFrameStay:SetText(Text("stay"))
  RealmBoxCompanionsFrameRegroup:SetText(Text("regroup"))
  RealmBoxCompanionsFrameBehaviorLabel:SetText(Text("behavior"))
  RealmBoxCompanionsFrameBehaviorEscort:SetText(Text("behaviorEscort"))
  RealmBoxCompanionsFrameBehaviorGuard:SetText(Text("behaviorGuard"))
  RealmBoxCompanionsFrameBehaviorFree:SetText(Text("behaviorAutonomous"))
  RealmBoxCompanionsFrameLeave:SetText(Text("leave"))
  RealmBoxCompanionsFrameLanguage:SetText(Text("language"))
end

local function BehaviorText(behavior)
  behavior = behavior or RealmBoxCompanionsDB.behaviorPreference
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

  local hasParty = GetNumPartyMembers() > 0
  SetButtonEnabled(RealmBoxCompanionsFrameFollow, hasParty)
  SetButtonEnabled(RealmBoxCompanionsFrameAttack, hasParty and IsAttackableTarget())
  SetButtonEnabled(RealmBoxCompanionsFrameStay, hasParty)
  SetButtonEnabled(RealmBoxCompanionsFrameRegroup, hasParty)
  SetButtonEnabled(RealmBoxCompanionsFrameBehaviorEscort, hasParty)
  SetButtonEnabled(RealmBoxCompanionsFrameBehaviorGuard, hasParty)
  SetButtonEnabled(RealmBoxCompanionsFrameBehaviorFree, hasParty)
  SetButtonEnabled(RealmBoxCompanionsFrameBoost, hasParty)
  SetButtonEnabled(RealmBoxCompanionsFrameLeave, hasParty)
  local behavior = RealmBoxCompanionsDB.behaviorPreference
  SetButtonSelected(RealmBoxCompanionsFrameBehaviorEscort, behavior == "escort")
  SetButtonSelected(RealmBoxCompanionsFrameBehaviorGuard, behavior == "guard")
  SetButtonSelected(RealmBoxCompanionsFrameBehaviorFree, behavior == "autonomous")

  if RealmBoxCompanionsDB.boostPreference == true then
    RealmBoxCompanionsFrameBoost:SetText(Text("boostOn"))
  elseif RealmBoxCompanionsDB.boostPreference == false then
    RealmBoxCompanionsFrameBoost:SetText(Text("boostOff"))
  else
    RealmBoxCompanionsFrameBoost:SetText(Text("boostDefault"))
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

local function Initialize()
  local firstRun = type(RealmBoxCompanionsDB) ~= "table" or not RealmBoxCompanionsDB.seen
  if type(RealmBoxCompanionsDB) ~= "table" then
    RealmBoxCompanionsDB = {}
  end

  if RealmBoxCompanionsDB.language ~= "fr" and RealmBoxCompanionsDB.language ~= "en" then
    if GetLocale and string.sub(GetLocale(), 1, 2) == "fr" then
      RealmBoxCompanionsDB.language = "fr"
    else
      RealmBoxCompanionsDB.language = "en"
    end
  end
  if type(RealmBoxCompanionsDB.minimapAngle) ~= "number" then
    RealmBoxCompanionsDB.minimapAngle = 225
  end
  if RealmBoxCompanionsDB.behaviorPreference ~= nil
      and not BEHAVIOR_COMMANDS[RealmBoxCompanionsDB.behaviorPreference] then
    RealmBoxCompanionsDB.behaviorPreference = nil
  end
  if firstRun then
    RealmBoxCompanionsDB.panelShown = true
  end
  RealmBoxCompanionsDB.seen = true

  initialized = true
  ApplyTranslations()
  RestorePanelPosition()
  PositionMinimapButton()
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
  table.insert(UISpecialFrames, "RealmBoxCompanionsFrame")
end

function RealmBoxCompanions_OnEvent(frame, event, argument)
  if event == "ADDON_LOADED" and argument == ADDON_NAME then
    Initialize()
    return
  end
  if initialized then
    if event == "PLAYER_ENTERING_WORLD" and not enteringWorldHandled then
      enteringWorldHandled = true
      ScheduleBehaviorReapply(1)
    elseif event == "PARTY_MEMBERS_CHANGED" and behaviorReapplyPending then
      behaviorReapplyElapsed = 0
    end
    UpdateGroupState()
  end
end

function RealmBoxCompanions_OnUpdate(frame, elapsed)
  if table.getn(partyQueue) > 0 then
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
    end
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
  if CurrentLanguage() == "fr" then
    RealmBoxCompanionsDB.language = "en"
  else
    RealmBoxCompanionsDB.language = "fr"
  end
  ApplyTranslations()
  UpdateGroupState()
  SetStatus(Text("ready"))
end

function RealmBoxCompanions_FormParty()
  local connectedClasses, _, connectedCount, offlineNames = PartySnapshot()

  if connectedCount >= 4 and table.getn(offlineNames) == 0 then
    ScheduleBehaviorReapply(4)
    SetStatus(Text("complete"))
    return
  end

  for _, name in ipairs(offlineNames) do
    UninviteUnit(name)
  end

  partyQueue = {}
  local slotsRemaining = 4 - connectedCount
  for _, companion in ipairs(PARTY_TEMPLATE) do
    if slotsRemaining == 0 then
      break
    end
    if not connectedClasses[companion.classToken] then
      table.insert(partyQueue, companion.command)
      slotsRemaining = slotsRemaining - 1
    end
  end

  if table.getn(partyQueue) == 0 then
    ScheduleBehaviorReapply(4)
    SetStatus(Text("complete"))
    return
  end

  ScheduleBehaviorReapply(4)
  partyQueueElapsed = 0.8
  if table.getn(offlineNames) > 0 then
    SetStatus(Text("reconnecting"))
  else
    SetStatus(Text("forming"))
  end
end

function RealmBoxCompanions_Run(action)
  local command = COMMANDS[action]
  if not command then
    DEFAULT_CHAT_FRAME:AddMessage(Text("actionRefused"))
    return
  end
  if GetNumPartyMembers() == 0 then
    SetStatus(Text("noParty"))
    return
  end
  if action == "attack" and not IsAttackableTarget() then
    SetStatus(Text("noTarget"))
    return
  end
  if action == "leave" then
    CancelBehaviorReapply()
    RealmBoxCompanionsDB.behaviorPreference = "autonomous"
    SendChatMessage(BEHAVIOR_COMMANDS.autonomous, "PARTY")
    SendChatMessage(command, "PARTY")
    SetStatus(Text("released"))
    UpdateGroupState()
    return
  end
  SendChatMessage(command, "PARTY")
  SetStatus(string.format(Text("commandSent"), Text(action)))
  UpdateGroupState()
end

function RealmBoxCompanions_SetBehavior(behavior)
  local command = BEHAVIOR_COMMANDS[behavior]
  if not command then
    DEFAULT_CHAT_FRAME:AddMessage(Text("actionRefused"))
    return
  end
  if GetNumPartyMembers() == 0 then
    SetStatus(Text("noParty"))
    return
  end

  CancelBehaviorReapply()
  RealmBoxCompanionsDB.behaviorPreference = behavior
  SendChatMessage(command, "PARTY")
  if table.getn(partyQueue) > 0 then
    ScheduleBehaviorReapply(4)
  end
  SetStatus(string.format(Text("behaviorSent"), BehaviorText(behavior)))
  UpdateGroupState()
end

function RealmBoxCompanions_CycleBehavior()
  local current = RealmBoxCompanionsDB.behaviorPreference
  local nextBehavior = "escort"
  if current == "escort" or current == nil then
    nextBehavior = "guard"
  elseif current == "guard" then
    nextBehavior = "autonomous"
  end
  RealmBoxCompanions_SetBehavior(nextBehavior)
end

function RealmBoxCompanions_ToggleBoost()
  if GetNumPartyMembers() == 0 then
    SetStatus(Text("noParty"))
    return
  end

  RealmBoxCompanionsDB.boostPreference = RealmBoxCompanionsDB.boostPreference ~= true
  if RealmBoxCompanionsDB.boostPreference then
    SendChatMessage("co +boost", "PARTY")
    SetStatus(Text("boostRequestedOn"))
  else
    SendChatMessage("co -boost", "PARTY")
    SetStatus(Text("boostRequestedOff"))
  end
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
  local title = Text(action)
  if action == "behavior" then
    title = BehaviorText()
  elseif action == "boost" then
    if RealmBoxCompanionsDB.boostPreference == true then
      title = Text("boostOn")
    elseif RealmBoxCompanionsDB.boostPreference == false then
      title = Text("boostOff")
    else
      title = Text("boostDefault")
    end
  end
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(title)
  if GetNumPartyMembers() == 0 then
    GameTooltip:AddLine(Text("noParty"), 1, 0.35, 0.35)
  elseif action == "attack" and not IsAttackableTarget() then
    GameTooltip:AddLine(Text("noTarget"), 1, 0.35, 0.35)
  elseif action == "behavior" then
    GameTooltip:AddLine(Text("behaviorHelp"), 0.8, 0.8, 0.8, true)
  elseif action == "boost" then
    GameTooltip:AddLine(Text("boostHelp"), 0.8, 0.8, 0.8, true)
  else
    GameTooltip:AddLine(Text("available"), 0.8, 0.8, 0.8)
  end
  GameTooltip:Show()
end

function RealmBoxCompanions_Behavior_OnEnter(button, behavior)
  GameTooltip:SetOwner(button, "ANCHOR_LEFT")
  GameTooltip:SetText(BehaviorText(behavior))
  if GetNumPartyMembers() == 0 then
    GameTooltip:AddLine(Text("noParty"), 1, 0.35, 0.35)
  else
    GameTooltip:AddLine(Text("behaviorHelp"), 0.8, 0.8, 0.8, true)
  end
  GameTooltip:Show()
end

SLASH_REALMBOXCOMPANIONS1 = "/realmbox"
SLASH_REALMBOXCOMPANIONS2 = "/rb"
SlashCmdList.REALMBOXCOMPANIONS = function(message)
  local command = string.lower(message or "")
  if command == "fr" or command == "en" then
    RealmBoxCompanionsDB.language = command
    ApplyTranslations()
    UpdateGroupState()
    SetStatus(Text("ready"))
    RealmBoxCompanionsFrame:Show()
    return
  end
  RealmBoxCompanions_Toggle()
end

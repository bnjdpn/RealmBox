import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { lauxlib, lua, lualib, to_jsstring, to_luastring } from "fengari";

const addonUrl = new URL("../addons/RealmBoxCompanions/RealmBoxCompanions.lua", import.meta.url);
const xmlUrl = new URL("../addons/RealmBoxCompanions/RealmBoxCompanions.xml", import.meta.url);
const tocUrl = new URL("../addons/RealmBoxCompanions/RealmBoxCompanions.toc", import.meta.url);
const [addon, xml, toc] = await Promise.all([
  readFile(addonUrl, "utf8"),
  readFile(xmlUrl, "utf8"),
  readFile(tocUrl, "utf8"),
]);

const wowApiMock = String.raw`
UIParent = {}
UISpecialFrames = {}
SlashCmdList = {}
table.getn = function(values) return #values end
sentMessages = {}
uninvited = {}
party = {}
targetAttackable = false
targetName = nil
playerCombat = false
cursorX = 500
cursorY = 500

function NewWidget(name)
  local widget = {
    name = name,
    text = "",
    enabled = true,
    shown = false,
    point = { "CENTER", UIParent, "CENTER", 330, 0 },
    events = {},
  }
  function widget:SetText(value) self.text = value end
  function widget:Enable() self.enabled = true end
  function widget:Disable() self.enabled = false end
  function widget:IsShown() return self.shown end
  function widget:Show()
    self.shown = true
    if RealmBoxCompanions_OnShow then RealmBoxCompanions_OnShow() end
  end
  function widget:Hide()
    self.shown = false
    if RealmBoxCompanions_OnHide then RealmBoxCompanions_OnHide() end
  end
  function widget:ClearAllPoints() self.point = nil end
  function widget:SetPoint(point, relativeTo, relativePoint, x, y)
    self.point = { point, relativeTo, relativePoint, x, y }
  end
  function widget:GetPoint() return table.unpack(self.point) end
  function widget:RegisterForDrag() end
  function widget:RegisterForClicks() end
  function widget:SetClampedToScreen() end
  function widget:SetBackdropColor() end
  function widget:RegisterEvent(event) self.events[event] = true end
  function widget:StartMoving() self.moving = true end
  function widget:StopMovingOrSizing() self.moving = false end
  function widget:LockHighlight() self.highlighted = true end
  function widget:UnlockHighlight() self.highlighted = false end
  return widget
end

RealmBoxCompanionsFrame = NewWidget("frame")
RealmBoxCompanionsFrameTitle = NewWidget("title")
RealmBoxCompanionsFrameGroupStatus = NewWidget("groupStatus")
RealmBoxCompanionsFrameStatus = NewWidget("status")
RealmBoxCompanionsFrameLanguage = NewWidget("language")
RealmBoxCompanionsFramePresetLabel = NewWidget("presetLabel")
RealmBoxCompanionsFramePresetBalanced = NewWidget("presetBalanced")
RealmBoxCompanionsFramePresetArcane = NewWidget("presetArcane")
RealmBoxCompanionsFramePresetWilderness = NewWidget("presetWilderness")
RealmBoxCompanionsFramePresetSummary = NewWidget("presetSummary")
RealmBoxCompanionsFrameSavedNames = NewWidget("savedNames")
RealmBoxCompanionsFrameFormParty = NewWidget("formParty")
RealmBoxCompanionsFrameScopeLabel = NewWidget("scopeLabel")
RealmBoxCompanionsFrameScopeGroup = NewWidget("scopeGroup")
RealmBoxCompanionsFrameScopeTarget = NewWidget("scopeTarget")
RealmBoxCompanionsFramePrimary = NewWidget("primary")
RealmBoxCompanionsFrameSetPrimary = NewWidget("setPrimary")
RealmBoxCompanionsFrameFollow = NewWidget("follow")
RealmBoxCompanionsFrameAttack = NewWidget("attack")
RealmBoxCompanionsFrameStay = NewWidget("stay")
RealmBoxCompanionsFrameRegroup = NewWidget("regroup")
RealmBoxCompanionsFrameBehaviorLabel = NewWidget("behaviorLabel")
RealmBoxCompanionsFrameBehaviorEscort = NewWidget("behaviorEscort")
RealmBoxCompanionsFrameBehaviorGuard = NewWidget("behaviorGuard")
RealmBoxCompanionsFrameBehaviorFree = NewWidget("behaviorFree")
RealmBoxCompanionsFrameBoost = NewWidget("boost")
RealmBoxCompanionsFrameLeave = NewWidget("leave")
RealmBoxCompanionsFramePreviewLabel = NewWidget("previewLabel")
RealmBoxCompanionsFramePreview = NewWidget("preview")
RealmBoxCompanionsMinimapButton = NewWidget("minimapButton")

Minimap = NewWidget("minimap")
function Minimap:GetEffectiveScale() return 1 end
function Minimap:GetCenter() return 500, 500 end

GameTooltip = NewWidget("tooltip")
function GameTooltip:SetOwner(owner, anchor) self.owner = owner; self.anchor = anchor end
function GameTooltip:AddLine(line) table.insert(self, line) end

DEFAULT_CHAT_FRAME = { messages = {} }
function DEFAULT_CHAT_FRAME:AddMessage(message) table.insert(self.messages, message) end

function GetLocale() return "frFR" end
function GetNumPartyMembers() return #party end
function UnitName(unit)
  if unit == "target" then return targetName end
  local index = tonumber(string.match(unit, "party(%d+)"))
  return index and party[index] and party[index].name or nil
end
function UnitIsConnected(unit)
  local index = tonumber(string.match(unit, "party(%d+)"))
  return index and party[index] and party[index].connected or false
end
function UnitClass(unit)
  local index = tonumber(string.match(unit, "party(%d+)"))
  local member = index and party[index]
  if not member then return nil, nil end
  return member.classToken, member.classToken
end
function UnitExists(unit) return unit == "target" and (targetAttackable or targetName ~= nil) end
function UnitCanAttack(source, target) return source == "player" and target == "target" and targetAttackable end
function UnitIsDeadOrGhost() return false end
function UnitAffectingCombat(unit)
  if unit == "player" then return playerCombat end
  local index = tonumber(string.match(unit, "party(%d+)"))
  return index and party[index] and party[index].combat or false
end
function InCombatLockdown() return playerCombat end
function UninviteUnit(name) table.insert(uninvited, name) end
function SendChatMessage(message, channel, language, recipient)
  table.insert(sentMessages, { message = message, channel = channel, recipient = recipient })
end
function GetCursorPosition() return cursorX, cursorY end
`;

function createLuaState(savedVariables = "RealmBoxCompanionsDB = nil") {
  const state = lauxlib.luaL_newstate();
  lualib.luaL_openlibs(state);
  runLua(state, wowApiMock, "WoW API mock");
  runLua(state, savedVariables, "saved variables");
  runLua(state, addon, "RealmBoxCompanions.lua");
  return state;
}

function runLua(state, source, label = "test script") {
  const status = lauxlib.luaL_dostring(state, to_luastring(source));
  if (status !== lua.LUA_OK) {
    const message = to_jsstring(lua.lua_tostring(state, -1));
    lua.lua_pop(state, 1);
    assert.fail(`${label}: ${message}`);
  }
}

function initialize(state) {
  runLua(state, String.raw`
    RealmBoxCompanions_OnLoad(RealmBoxCompanionsFrame)
    RealmBoxCompanions_Minimap_OnLoad(RealmBoxCompanionsMinimapButton)
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "ADDON_LOADED", "RealmBoxCompanions")
  `, "addon initialization");
}

test("declares a native minimap control, close button, persistence, and 3.3.5a metadata", () => {
  assert.match(xml, /name="RealmBoxCompanionsMinimapButton" parent="Minimap"/);
  assert.match(xml, /inherits="UIPanelCloseButton"/);
  assert.match(xml, /hidden="true"/);
  assert.match(xml, /RealmBoxCompanions_OnEvent\(self, event, \.\.\.\)/);
  assert.match(xml, /RealmBoxCompanions_Minimap_OnDragStart/);
  assert.match(xml, /RealmBoxCompanions_SetBehavior\("escort"\)/);
  assert.match(xml, /RealmBoxCompanions_SetBehavior\("guard"\)/);
  assert.match(xml, /RealmBoxCompanions_SetBehavior\("autonomous"\)/);
  assert.match(xml, /RealmBoxCompanions_SelectPreset\("balanced"\)/);
  assert.match(xml, /RealmBoxCompanions_SelectPreset\("arcane"\)/);
  assert.match(xml, /RealmBoxCompanions_SelectPreset\("wilderness"\)/);
  assert.match(xml, /RealmBoxCompanions_SelectScope\("target"\)/);
  assert.match(xml, /RealmBoxCompanions_SetPrimary\(\)/);
  assert.match(xml, /name="\$parentPreview"/);
  assert.match(xml, /name="RealmBoxCompanionsDriver" parent="UIParent"/);
  assert.doesNotMatch(addon, /UninviteUnit\(/);
  assert.doesNotMatch(xml, /RealmBoxCompanions_CycleBehavior\(\)/);
  assert.match(toc, /## Interface: 30300/);
  assert.match(toc, /## SavedVariables: RealmBoxCompanionsDB/);
  assert.match(toc, /## Version: 0\.4\.0-dev/);
});

test("opens only on first run, toggles from slash commands, and persists positions", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    assert(RealmBoxCompanionsDB.seen == true)
    assert(RealmBoxCompanionsDB.language == "fr")
    assert(RealmBoxCompanionsFrame:IsShown() == true)
    assert(RealmBoxCompanionsFrameFollow.enabled == false)
    assert(RealmBoxCompanionsFrameAttack.enabled == false)
    assert(RealmBoxCompanionsFrameBehaviorLabel.text == "Comportement")
    assert(RealmBoxCompanionsFrameBehaviorEscort.enabled == false)
    assert(RealmBoxCompanionsFrameBehaviorGuard.enabled == false)
    assert(RealmBoxCompanionsFrameBehaviorFree.enabled == false)
    assert(RealmBoxCompanionsFrameBehaviorEscort.text == "Escorte")
    assert(RealmBoxCompanionsFrameBehaviorGuard.text == "Garde")
    assert(RealmBoxCompanionsFrameBehaviorFree.text == "Libres")
    assert(RealmBoxCompanionsFrameBehaviorEscort.highlighted ~= true)
    assert(RealmBoxCompanionsFrameBoost.text == "Capacités fortes : serveur")
    assert(RealmBoxCompanionsDB.boostPreference == nil)
    assert(RealmBoxCompanionsDB.activePreset == "balanced")
    assert(RealmBoxCompanionsDB.commandScope == "group")
    assert(RealmBoxCompanionsFramePresetBalanced.highlighted == true)
    assert(RealmBoxCompanionsFrameScopeGroup.highlighted == true)
    assert(RealmBoxCompanionsFrameSetPrimary.enabled == false)
    assert(SLASH_REALMBOXCOMPANIONS1 == "/realmbox")
    assert(SLASH_REALMBOXCOMPANIONS2 == "/rb")
    assert(UISpecialFrames[1] == "RealmBoxCompanionsFrame")

    SlashCmdList.REALMBOXCOMPANIONS("")
    assert(RealmBoxCompanionsFrame:IsShown() == false)
    assert(RealmBoxCompanionsDB.panelShown == false)
    SlashCmdList.REALMBOXCOMPANIONS("")
    assert(RealmBoxCompanionsFrame:IsShown() == true)

    RealmBoxCompanionsFrame:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 120, -80)
    RealmBoxCompanions_OnDragStop(RealmBoxCompanionsFrame)
    assert(RealmBoxCompanionsDB.panelPoint == "TOPLEFT")
    assert(RealmBoxCompanionsDB.panelX == 120)
    assert(RealmBoxCompanionsDB.panelY == -80)

    cursorX = 500
    cursorY = 600
    RealmBoxCompanions_Minimap_OnDragStart(RealmBoxCompanionsMinimapButton)
    RealmBoxCompanions_Minimap_OnUpdate()
    RealmBoxCompanions_Minimap_OnDragStop(RealmBoxCompanionsMinimapButton)
    assert(math.abs(RealmBoxCompanionsDB.minimapAngle - 90) < 0.001)
    assert(math.abs(RealmBoxCompanionsMinimapButton.point[5] - 80) < 0.001)
  `);
});

test("restores a returning player's hidden panel, language, and saved position", () => {
  const state = createLuaState(String.raw`
    RealmBoxCompanionsDB = {
      seen = true,
      panelShown = false,
      language = "en",
      boostPreference = true,
      behaviorPreference = "guard",
      minimapAngle = 0,
      panelPoint = "TOPLEFT",
      panelRelativePoint = "TOPLEFT",
      panelX = 42,
      panelY = -24,
    }
  `);
  initialize(state);
  runLua(state, String.raw`
    assert(RealmBoxCompanionsFrame:IsShown() == false)
    assert(RealmBoxCompanionsFrameTitle.text == "COMPANIONS")
    assert(RealmBoxCompanionsFrameBoost.text == "Strong abilities: requested")
    assert(RealmBoxCompanionsFrameBehaviorGuard.highlighted == true)
    assert(RealmBoxCompanionsFrameBehaviorEscort.highlighted ~= true)
    assert(RealmBoxCompanionsFrame.point[1] == "TOPLEFT")
    assert(RealmBoxCompanionsFrame.point[4] == 42)
    assert(RealmBoxCompanionsMinimapButton.point[4] == 80)
  `);
});

test("tracks the live party, localizes classes, and gates attack on an enemy target", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = {
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
      { name = "Manuela", connected = true, classToken = "MAGE" },
      { name = "Tank", connected = true, classToken = "WARRIOR" },
      { name = "Garea", connected = false, classToken = "HUNTER" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")
    assert(RealmBoxCompanionsFrameGroupStatus.text == "Équipe : 1 joueur + 3/4 · Paladin, Mage, Guerrier · 1 hors ligne")
    assert(RealmBoxCompanionsFrameFollow.enabled == true)
    assert(RealmBoxCompanionsFrameAttack.enabled == false)

    targetAttackable = true
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_TARGET_CHANGED")
    assert(RealmBoxCompanionsFrameAttack.enabled == true)
    assert(RealmBoxCompanionsFrameBehaviorEscort.enabled == true)
    assert(RealmBoxCompanionsFrameBehaviorGuard.enabled == true)
    assert(RealmBoxCompanionsFrameBehaviorFree.enabled == true)

    RealmBoxCompanions_ToggleLanguage()
    assert(RealmBoxCompanionsDB.language == "en")
    assert(RealmBoxCompanionsFrameTitle.text == "COMPANIONS")
    assert(RealmBoxCompanionsFrameGroupStatus.text == "Party: 1 player + 3/4 · Paladin, Mage, Warrior · 1 offline")
    assert(RealmBoxCompanionsFrameBehaviorEscort.text == "Escort")
    assert(RealmBoxCompanionsFrameBehaviorGuard.text == "Guard")
    assert(RealmBoxCompanionsFrameBehaviorFree.text == "Free")
    SlashCmdList.REALMBOXCOMPANIONS("fr")
    assert(RealmBoxCompanionsDB.language == "fr")
  `);
});

test("runs only bounded commands and uses the pinned Playerbots boost strategy", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = {
      { name = "Manuela", connected = true, classToken = "MAGE" },
      { name = "Offline", connected = false, classToken = "HUNTER" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")

    RealmBoxCompanions_Run("attack")
    assert(#sentMessages == 0)
    assert(RealmBoxCompanionsFrameStatus.text == "Sélectionnez une cible ennemie vivante")
    targetAttackable = true
    RealmBoxCompanions_Run("attack")
    assert(sentMessages[1].message == "attack")
    assert(sentMessages[1].channel == "PARTY")

    RealmBoxCompanions_ToggleBoost()
    assert(sentMessages[2].message == "co +boost")
    assert(RealmBoxCompanionsDB.boostPreference == true)
    RealmBoxCompanions_ToggleBoost()
    assert(sentMessages[3].message == "co -boost")
    assert(RealmBoxCompanionsDB.boostPreference == false)

    RealmBoxCompanions_FormParty()
    assert(#uninvited == 0)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    assert(sentMessages[4].message == ".playerbots bot addclass paladin")
    assert(sentMessages[5].message == ".playerbots bot addclass priest")
    assert(#sentMessages == 5)
    assert(sentMessages[4].channel == "SAY")

    RealmBoxCompanions_Run("not-allowed")
    assert(#sentMessages == 5)
    assert(#DEFAULT_CHAT_FRAME.messages == 1)
  `);
});

test("offers three explicit bounded behaviors and keeps the legacy cycle wrapper", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = {
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
      { name = "Manuela", connected = true, classToken = "MAGE" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")

    RealmBoxCompanions_SetBehavior("escort")
    assert(sentMessages[1].message == "nc +follow,-stay,-new rpg,-grind")
    assert(sentMessages[1].channel == "PARTY")
    assert(RealmBoxCompanionsDB.behaviorPreference == "escort")
    assert(RealmBoxCompanionsFrameBehaviorEscort.highlighted == true)
    assert(RealmBoxCompanionsFrameBehaviorGuard.highlighted ~= true)
    assert(RealmBoxCompanionsFrameStatus.text == "Préférence envoyée : Escorte")

    RealmBoxCompanions_SetBehavior("guard")
    assert(sentMessages[2].message == "nc +stay,-follow,-new rpg,-grind")
    assert(RealmBoxCompanionsDB.behaviorPreference == "guard")
    assert(RealmBoxCompanionsFrameBehaviorGuard.highlighted == true)

    RealmBoxCompanions_SetBehavior("autonomous")
    assert(sentMessages[3].message == "nc +new rpg,+grind,-follow,-stay")
    assert(RealmBoxCompanionsDB.behaviorPreference == "autonomous")
    assert(RealmBoxCompanionsFrameBehaviorFree.highlighted == true)

    RealmBoxCompanions_SetBehavior("not-allowed")
    assert(#sentMessages == 3)
    assert(#DEFAULT_CHAT_FRAME.messages == 1)

    RealmBoxCompanions_Run("follow")
    assert(sentMessages[4].message == "follow")
    assert(RealmBoxCompanionsDB.behaviorPreference == "autonomous")

    RealmBoxCompanions_CycleBehavior()
    assert(sentMessages[5].message == "nc +follow,-stay,-new rpg,-grind")
    assert(RealmBoxCompanionsDB.behaviorPreference == "escort")

    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 5)
    assert(RealmBoxCompanionsFrameLeave.text == "Confirmer la libération")
    RealmBoxCompanions_Run("leave")
    assert(sentMessages[6].message == "nc +new rpg,+grind,-follow,-stay")
    assert(sentMessages[7].message == "leave")
    assert(RealmBoxCompanionsDB.behaviorPreference == "autonomous")
    assert(RealmBoxCompanionsFrameStatus.text == "Libération envoyée · autonomie puis départ du groupe")
  `);
});

test("reapplies the saved behavior only after a formed party is complete and stable", () => {
  const state = createLuaState(String.raw`
    RealmBoxCompanionsDB = { seen = true, language = "fr", behaviorPreference = "guard" }
  `);
  initialize(state);
  runLua(state, String.raw`
    party = {
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
      { name = "Offline", connected = false, classToken = "HUNTER" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")
    RealmBoxCompanions_FormParty()
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    assert(#sentMessages == 2)
    assert(sentMessages[1].channel == "SAY")

    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 3.0)
    assert(#sentMessages == 2)

    party = {
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
      { name = "Jillo", connected = true, classToken = "PRIEST" },
      { name = "Manuela", connected = true, classToken = "MAGE" },
      { name = "Garea", connected = true, classToken = "HUNTER" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 1.0)
    assert(#sentMessages == 2)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.5)
    assert(sentMessages[3].message == "nc +stay,-follow,-new rpg,-grind")
    assert(sentMessages[3].channel == "PARTY")
    assert(RealmBoxCompanionsFrameStatus.text == "Préférence réappliquée : Garde")
  `);
});

test("reapplies a saved behavior once after reconnect when the party is stable", () => {
  const state = createLuaState(String.raw`
    RealmBoxCompanionsDB = { seen = true, language = "en", behaviorPreference = "escort" }
  `);
  runLua(state, String.raw`
    party = {
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
      { name = "Jillo", connected = true, classToken = "PRIEST" },
    }
  `);
  initialize(state);
  runLua(state, String.raw`
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_ENTERING_WORLD")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 1.49)
    assert(#sentMessages == 0)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.01)
    assert(sentMessages[1].message == "nc +follow,-stay,-new rpg,-grind")
    assert(sentMessages[1].channel == "PARTY")
    assert(RealmBoxCompanionsFrameStatus.text == "Preference reapplied: Escort")

    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_ENTERING_WORLD")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 2.0)
    assert(#sentMessages == 1)
  `);
});

test("expires a reconnect behavior instead of applying it to an unrelated future party", () => {
  const state = createLuaState(String.raw`
    RealmBoxCompanionsDB = { seen = true, language = "en", behaviorPreference = "guard" }
  `);
  initialize(state);
  runLua(state, String.raw`
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_ENTERING_WORLD")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 30.01)
    assert(#sentMessages == 0)

    party = {
      { name = "Unrelated", connected = true, classToken = "WARRIOR" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 2.0)
    assert(#sentMessages == 0)
  `);
});

test("persists closed squad presets and captures only names observed after formation", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    RealmBoxCompanions_SelectPreset("arcane")
    assert(RealmBoxCompanionsDB.activePreset == "arcane")
    assert(RealmBoxCompanionsFramePresetArcane.highlighted == true)
    assert(string.find(RealmBoxCompanionsFramePresetSummary.text, "Dégâts Mage · Dégâts Mage", 1, true))
    RealmBoxCompanions_FormParty()
    for index = 1, 4 do RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8) end
    assert(#sentMessages == 4)
    assert(sentMessages[1].message == ".playerbots bot addclass paladin")
    assert(sentMessages[2].message == ".playerbots bot addclass priest")
    assert(sentMessages[3].message == ".playerbots bot addclass mage")
    assert(sentMessages[4].message == ".playerbots bot addclass mage")
    assert(RealmBoxCompanionsDB.squadMembers.arcane == nil)
    party = {
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
      { name = "Jillo", connected = true, classToken = "PRIEST" },
      { name = "Manuela", connected = true, classToken = "MAGE" },
      { name = "Sage", connected = true, classToken = "MAGE" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.1)
    assert(#RealmBoxCompanionsDB.squadMembers.arcane == 4)
    assert(RealmBoxCompanionsDB.squadMembers.arcane[3].name == "Manuela")
    assert(string.find(RealmBoxCompanionsFrameSavedNames.text, "Kayarid, Jillo, Manuela, Sage", 1, true))
    targetName = "Manuela"
    RealmBoxCompanions_SetPrimary()
    assert(RealmBoxCompanionsDB.primaryCompanionName == "Manuela")
    assert(RealmBoxCompanionsDB.primaryCompanionClassToken == "MAGE")
    assert(RealmBoxCompanionsFramePrimary.text == "Compagnon principal : Manuela")
    assert(string.find(RealmBoxCompanionsFrameSavedNames.text, "★Manuela", 1, true))
    assert(#uninvited == 0)
  `);
});

test("restores preset, observed names, primary companion and scoped preferences", () => {
  const state = createLuaState(String.raw`
    RealmBoxCompanionsDB = {
      seen = true,
      language = "en",
      activePreset = "wilderness",
      commandScope = "target",
      primaryCompanionName = "Garea",
      primaryBehaviorPreference = "guard",
      primaryBoostPreference = false,
      behaviorPreference = "escort",
      squadMembers = { wilderness = { { name = "Garea", classToken = "HUNTER" } } },
    }
  `);
  initialize(state);
  runLua(state, String.raw`
    assert(RealmBoxCompanionsFramePresetWilderness.highlighted == true)
    assert(RealmBoxCompanionsFrameScopeTarget.highlighted == true)
    assert(RealmBoxCompanionsFramePrimary.text == "Primary companion: Garea (absent)")
    assert(RealmBoxCompanionsFrameSavedNames.text == "Observed members: ★Garea")
    assert(RealmBoxCompanionsFrameBehaviorGuard.highlighted == true)
    assert(RealmBoxCompanionsFrameBoost.text == "Strong abilities: limited")
    assert(RealmBoxCompanionsFrameFollow.enabled == false)
    RealmBoxCompanions_SelectScope("group")
    assert(RealmBoxCompanionsFrameBehaviorEscort.highlighted == true)
    assert(RealmBoxCompanionsFrameBoost.text == "Strong abilities: server")
    assert(#sentMessages == 0)
  `);
});

test("keeps each squad's group preferences separate without sending commands on selection", () => {
  const state = createLuaState(String.raw`
    RealmBoxCompanionsDB = { seen = true, behaviorPreference = "guard", boostPreference = false }
  `);
  initialize(state);
  runLua(state, String.raw`
    assert(RealmBoxCompanionsDB.presetPreferences.balanced.behaviorPreference == "guard")
    assert(RealmBoxCompanionsDB.presetPreferences.balanced.boostPreference == false)
    RealmBoxCompanions_SelectPreset("arcane")
    assert(RealmBoxCompanionsDB.behaviorPreference == nil)
    assert(RealmBoxCompanionsDB.boostPreference == nil)
    assert(#sentMessages == 0)
    party = { { name = "Manuela", connected = true, classToken = "MAGE" } }
    RealmBoxCompanions_SetBehavior("escort")
    RealmBoxCompanions_ToggleBoost()
    assert(#sentMessages == 2)
    assert(RealmBoxCompanionsDB.presetPreferences.arcane.behaviorPreference == "escort")
    assert(RealmBoxCompanionsDB.presetPreferences.arcane.boostPreference == true)
    RealmBoxCompanions_SelectPreset("balanced")
    assert(RealmBoxCompanionsDB.behaviorPreference == "guard")
    assert(RealmBoxCompanionsDB.boostPreference == false)
    assert(#sentMessages == 2)
    RealmBoxCompanions_SelectPreset("arcane")
    assert(RealmBoxCompanionsDB.behaviorPreference == "escort")
    assert(RealmBoxCompanionsDB.boostPreference == true)
    assert(#sentMessages == 2)
  `);
});

test("fills only genuinely free slots and never removes offline human party members", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = {
      { name = "HumanFriend", connected = false, classToken = "PALADIN" },
      { name = "OtherHuman", connected = true, classToken = "MAGE" },
    }
    RealmBoxCompanions_Action_OnEnter(RealmBoxCompanionsFrameFormParty, "form")
    assert(RealmBoxCompanionsFramePreview.text == "Groupe · .playerbots bot addclass priest ; .playerbots bot addclass hunter")
    RealmBoxCompanions_FormParty()
    for index = 1, 4 do RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8) end
    assert(#uninvited == 0)
    assert(#sentMessages == 2)
    assert(sentMessages[1].message == ".playerbots bot addclass priest")
    assert(sentMessages[2].message == ".playerbots bot addclass hunter")
    party = {
      { name = "HumanFriend", connected = false, classToken = "PALADIN" },
      { name = "OtherHuman", connected = true, classToken = "MAGE" },
      { name = "ThirdHuman", connected = false, classToken = "PRIEST" },
      { name = "FourthHuman", connected = true, classToken = "HUNTER" },
    }
    RealmBoxCompanions_FormParty()
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 1)
    assert(#sentMessages == 2)
    assert(#uninvited == 0)
    assert(RealmBoxCompanionsFrameStatus.text == "Votre groupe de cinq est déjà complet")
  `);
});

test("targets only the saved primary currently connected in the party using verified whisper commands", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = {
      { name = "Manuela", connected = true, classToken = "MAGE" },
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
    }
    targetName = "Manuela"
    RealmBoxCompanions_SelectScope("target")
    RealmBoxCompanions_Run("follow")
    assert(#sentMessages == 0)
    assert(RealmBoxCompanionsFrameStatus.text == "Ciblez le compagnon principal enregistré, connecté dans ce groupe")
    RealmBoxCompanions_SetPrimary()
    RealmBoxCompanions_Run("follow")
    assert(sentMessages[1].message == "follow")
    assert(sentMessages[1].channel == "WHISPER")
    assert(sentMessages[1].recipient == "Manuela")
    assert(RealmBoxCompanionsFramePreview.text == "Cible Manuela · follow")
    RealmBoxCompanions_Run("stay")
    assert(sentMessages[2].message == "stay")
    RealmBoxCompanions_SetBehavior("guard")
    assert(sentMessages[3].message == "nc +stay,-follow,-new rpg,-grind")
    assert(RealmBoxCompanionsDB.primaryBehaviorPreference == "guard")
    assert(RealmBoxCompanionsDB.behaviorPreference == nil)
    RealmBoxCompanions_ToggleBoost()
    assert(sentMessages[4].message == "co +boost")
    assert(RealmBoxCompanionsDB.primaryBoostPreference == true)
    assert(RealmBoxCompanionsDB.boostPreference == nil)
    for index = 1, 4 do
      assert(sentMessages[index].channel == "WHISPER")
      assert(sentMessages[index].recipient == "Manuela")
    end
    RealmBoxCompanions_Run("regroup")
    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 4)
    assert(RealmBoxCompanionsFrameStatus.text == "Cette action est disponible uniquement pour le groupe")
    RealmBoxCompanions_FormParty()
    assert(RealmBoxCompanionsFrameStatus.text == "Cette action concerne le groupe : choisissez Groupe")
    targetName = "Kayarid"
    RealmBoxCompanions_Run("follow")
    assert(#sentMessages == 4)
    targetName = "Manuela"
    party[1].connected = false
    RealmBoxCompanions_Run("stay")
    assert(#sentMessages == 4)
    party = { { name = "Stranger", connected = true, classToken = "MAGE" } }
    RealmBoxCompanions_Run("follow")
    assert(#sentMessages == 4)
  `);
});

test("suspends commands, formation and local action preferences during combat then resumes safely", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    RealmBoxCompanions_FormParty()
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    assert(#sentMessages == 1)
    party = { { name = "Kayarid", connected = true, classToken = "PALADIN" } }
    targetName = "Kayarid"
    targetAttackable = true
    playerCombat = true
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_REGEN_DISABLED")
    RealmBoxCompanions_Run("follow")
    RealmBoxCompanions_Run("attack")
    RealmBoxCompanions_SetBehavior("guard")
    RealmBoxCompanions_ToggleBoost()
    RealmBoxCompanions_FormParty()
    RealmBoxCompanions_SelectPreset("arcane")
    RealmBoxCompanions_SelectScope("target")
    RealmBoxCompanions_SetPrimary()
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 2)
    assert(#sentMessages == 1)
    assert(RealmBoxCompanionsDB.activePreset == "balanced")
    assert(RealmBoxCompanionsDB.commandScope == "group")
    assert(RealmBoxCompanionsDB.behaviorPreference == nil)
    assert(RealmBoxCompanionsDB.boostPreference == nil)
    assert(RealmBoxCompanionsDB.primaryCompanionName == nil)
    assert(RealmBoxCompanionsFrameFollow.enabled == false)
    assert(RealmBoxCompanionsFrameFormParty.enabled == false)
    assert(RealmBoxCompanionsFrameStatus.text == "Formation suspendue pendant le combat")
    playerCombat = false
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_REGEN_ENABLED")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    assert(#sentMessages == 2)
    assert(sentMessages[2].message == ".playerbots bot addclass priest")
    RealmBoxCompanions_SelectScope("target")
    assert(RealmBoxCompanionsDB.commandScope == "group")
    assert(RealmBoxCompanionsFrameStatus.text == "Attendez la fin de la formation en cours")
  `);
});

test("expires formation while paused and cancels queued additions when the party fills", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    RealmBoxCompanions_FormParty()
    playerCombat = true
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 30.01)
    assert(#sentMessages == 0)
    assert(RealmBoxCompanionsFrameStatus.text == "Formation expirée · les commandes restantes sont annulées")
    playerCombat = false
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 1)
    assert(#sentMessages == 0)
    RealmBoxCompanions_FormParty()
    party = {
      { name = "A", connected = true, classToken = "PALADIN" },
      { name = "B", connected = true, classToken = "PRIEST" },
      { name = "C", connected = true, classToken = "MAGE" },
      { name = "D", connected = true, classToken = "HUNTER" },
    }
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    assert(#sentMessages == 0)
    assert(#uninvited == 0)
  `);
});

test("release confirmation expires and is invalidated by composition or combat changes", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = { { name = "First", connected = true, classToken = "MAGE" } }
    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 0)
    assert(RealmBoxCompanionsFrameLeave.text == "Confirmer la libération")
    assert(RealmBoxCompanionsFramePreview.text == "Groupe · nc +new rpg,+grind,-follow,-stay ; leave")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 8.01)
    assert(RealmBoxCompanionsFrameLeave.text == "Libérer l'équipe")
    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 0)
    party[1].name = "Replacement"
    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 0)
    playerCombat = true
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_REGEN_DISABLED")
    playerCombat = false
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_REGEN_ENABLED")
    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 0)
    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 2)
    assert(sentMessages[1].message == "nc +new rpg,+grind,-follow,-stay")
    assert(sentMessages[2].message == "leave")
    assert(#uninvited == 0)
  `);
});

test("keeps command previews and unavailable reasons bilingual and fails closed on invalid saved data", () => {
  const state = createLuaState(String.raw`
    RealmBoxCompanionsDB = {
      language = "en", activePreset = "arbitrary", commandScope = "unknown",
      primaryCompanionName = false, primaryBehaviorPreference = "guard",
      squadMembers = { balanced = { "bad", { name = false }, { name = "Observed", classToken = 1 } } },
    }
  `);
  initialize(state);
  runLua(state, String.raw`
    assert(RealmBoxCompanionsDB.activePreset == "balanced")
    assert(RealmBoxCompanionsDB.commandScope == "group")
    assert(RealmBoxCompanionsDB.primaryCompanionName == nil)
    assert(RealmBoxCompanionsDB.primaryBehaviorPreference == nil)
    assert(#RealmBoxCompanionsDB.squadMembers.balanced == 1)
    RealmBoxCompanions_Action_OnEnter(RealmBoxCompanionsFrameFollow, "follow")
    assert(RealmBoxCompanionsFramePreview.text == "Unavailable · Build a party first")
    party = { { name = "Observed", connected = true, classToken = "MAGE" } }
    RealmBoxCompanions_Action_OnEnter(RealmBoxCompanionsFrameFollow, "follow")
    assert(RealmBoxCompanionsFramePreview.text == "Party · follow")
    playerCombat = true
    RealmBoxCompanions_Action_OnEnter(RealmBoxCompanionsFrameFollow, "follow")
    assert(RealmBoxCompanionsFramePreview.text == "Unavailable · Unavailable in combat · no command will be sent")
    RealmBoxCompanions_ToggleLanguage()
    RealmBoxCompanions_Action_OnEnter(RealmBoxCompanionsFrameFollow, "follow")
    assert(RealmBoxCompanionsFramePreview.text == "Indisponible · Indisponible en combat · aucune commande ne sera envoyée")
    RealmBoxCompanions_SelectPreset("not-allowed")
    RealmBoxCompanions_SelectScope("not-allowed")
    assert(#sentMessages == 0)
    assert(#DEFAULT_CHAT_FRAME.messages == 2)
  `);
});

test("never turns observed names into generic add commands or a promised recall", () => {
  assert.doesNotMatch(addon, /\.playerbots bot add(?:\s|["'])/);
  assert.doesNotMatch(xml, /<EditBox\b/);
  assert.match(addon, /\.playerbots bot addclass paladin/);
  assert.match(addon, /\.playerbots bot addclass priest/);
  assert.match(addon, /\.playerbots bot addclass mage/);
  assert.match(addon, /\.playerbots bot addclass hunter/);
});

test("blocks commands for an entirely offline party or any fighting party member", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = { { name = "Offline", connected = false, classToken = "MAGE" } }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")
    RealmBoxCompanions_Run("follow")
    assert(#sentMessages == 0)
    assert(RealmBoxCompanionsFrameFollow.enabled == false)
    assert(RealmBoxCompanionsFrameStatus.text == "Aucun membre du groupe n'est connecté")
    party[1].connected = true
    RealmBoxCompanions_Run("leave")
    assert(RealmBoxCompanionsFrameLeave.text == "Confirmer la libération")
    party[1].combat = true
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "UNIT_FLAGS", "party1")
    RealmBoxCompanions_Run("follow")
    RealmBoxCompanions_ToggleBoost()
    assert(#sentMessages == 0)
    assert(RealmBoxCompanionsFrameLeave.text == "Libérer l'équipe")
    assert(RealmBoxCompanionsFrameFollow.enabled == false)
    assert(RealmBoxCompanionsFrameStatus.text == "Indisponible en combat · aucune commande ne sera envoyée")
    party[1].combat = false
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "UNIT_FLAGS", "party1")
    RealmBoxCompanions_Run("leave")
    assert(#sentMessages == 0)
    assert(RealmBoxCompanionsFrameLeave.text == "Confirmer la libération")
    RealmBoxCompanions_ToggleLanguage()
    party[1].connected = false
    RealmBoxCompanions_Run("follow")
    assert(RealmBoxCompanionsFrameStatus.text == "No party member is connected")
  `);
});

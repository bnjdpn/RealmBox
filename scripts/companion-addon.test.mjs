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
RealmBoxCompanionsFrameFormParty = NewWidget("formParty")
RealmBoxCompanionsFrameFollow = NewWidget("follow")
RealmBoxCompanionsFrameAttack = NewWidget("attack")
RealmBoxCompanionsFrameStay = NewWidget("stay")
RealmBoxCompanionsFrameRegroup = NewWidget("regroup")
RealmBoxCompanionsFrameBehavior = NewWidget("behavior")
RealmBoxCompanionsFrameBoost = NewWidget("boost")
RealmBoxCompanionsFrameLeave = NewWidget("leave")
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
function UnitExists(unit) return unit == "target" and targetAttackable end
function UnitCanAttack(source, target) return source == "player" and target == "target" and targetAttackable end
function UnitIsDeadOrGhost() return false end
function UninviteUnit(name) table.insert(uninvited, name) end
function SendChatMessage(message, channel)
  table.insert(sentMessages, { message = message, channel = channel })
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
  assert.match(toc, /## Interface: 30300/);
  assert.match(toc, /## SavedVariables: RealmBoxCompanionsDB/);
  assert.match(toc, /## Version: 0\.3\.3-dev/);
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
    assert(RealmBoxCompanionsFrameBehavior.enabled == false)
    assert(RealmBoxCompanionsFrameBehavior.text == "Comportement : escorte")
    assert(RealmBoxCompanionsFrameBoost.text == "Capacités fortes : serveur")
    assert(RealmBoxCompanionsDB.boostPreference == nil)
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
    assert(RealmBoxCompanionsFrameGroupStatus.text == "Équipe : 3/4 · Paladin, Mage, Guerrier · 1 hors ligne")
    assert(RealmBoxCompanionsFrameFollow.enabled == true)
    assert(RealmBoxCompanionsFrameAttack.enabled == false)

    targetAttackable = true
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PLAYER_TARGET_CHANGED")
    assert(RealmBoxCompanionsFrameAttack.enabled == true)
    assert(RealmBoxCompanionsFrameBehavior.enabled == true)

    RealmBoxCompanions_ToggleLanguage()
    assert(RealmBoxCompanionsDB.language == "en")
    assert(RealmBoxCompanionsFrameTitle.text == "COMPANIONS")
    assert(RealmBoxCompanionsFrameGroupStatus.text == "Party: 3/4 · Paladin, Mage, Warrior · 1 offline")
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
    assert(RealmBoxCompanionsFrameStatus.text == "Sélectionnez une cible ennemie")
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
    assert(uninvited[1] == "Offline")
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    RealmBoxCompanions_OnUpdate(RealmBoxCompanionsFrame, 0.8)
    assert(sentMessages[4].message == ".playerbots bot addclass paladin")
    assert(sentMessages[5].message == ".playerbots bot addclass priest")
    assert(sentMessages[6].message == ".playerbots bot addclass hunter")
    assert(sentMessages[4].channel == "SAY")

    RealmBoxCompanions_Run("not-allowed")
    assert(#sentMessages == 6)
    assert(#DEFAULT_CHAT_FRAME.messages == 1)
  `);
});

test("changes party behavior live and releases bots back to autonomous activity", () => {
  const state = createLuaState();
  initialize(state);
  runLua(state, String.raw`
    party = {
      { name = "Kayarid", connected = true, classToken = "PALADIN" },
      { name = "Manuela", connected = true, classToken = "MAGE" },
    }
    RealmBoxCompanions_OnEvent(RealmBoxCompanionsFrame, "PARTY_MEMBERS_CHANGED")

    RealmBoxCompanions_CycleBehavior()
    assert(sentMessages[1].message == "nc +stay,-follow,-new rpg,-grind")
    assert(sentMessages[1].channel == "PARTY")
    assert(RealmBoxCompanionsDB.behaviorPreference == "guard")
    assert(RealmBoxCompanionsFrameBehavior.text == "Comportement : garde")

    RealmBoxCompanions_CycleBehavior()
    assert(sentMessages[2].message == "nc +new rpg,+grind,-follow,-stay")
    assert(RealmBoxCompanionsDB.behaviorPreference == "autonomous")
    assert(RealmBoxCompanionsFrameBehavior.text == "Comportement : autonomes")

    RealmBoxCompanions_Run("follow")
    assert(sentMessages[3].message == "follow")
    assert(RealmBoxCompanionsDB.behaviorPreference == "escort")

    RealmBoxCompanions_Run("leave")
    assert(sentMessages[4].message == "nc +new rpg,+grind,-follow,-stay")
    assert(sentMessages[5].message == "leave")
    assert(RealmBoxCompanionsDB.behaviorPreference == "autonomous")
    assert(RealmBoxCompanionsFrameStatus.text == "Équipe libérée · les bots reprennent leurs activités")
  `);
});

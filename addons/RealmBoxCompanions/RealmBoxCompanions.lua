local COMMANDS = {
  follow = "follow",
  attack = "attack",
  stay = "stay",
  regroup = "summon",
  cooldowns_on = "cooldowns on",
  leave = "leave",
}

local PARTY_TEMPLATE = {
  ".playerbots bot addclass paladin",
  ".playerbots bot addclass priest",
  ".playerbots bot addclass mage",
  ".playerbots bot addclass hunter",
}

local partyQueue = {}
local partyQueueElapsed = 0

local function SetStatus(message)
  RealmBoxCompanionsFrameStatus:SetText(message)
end

function RealmBoxCompanions_OnLoad(frame)
  frame:RegisterForDrag("LeftButton")
  frame:SetClampedToScreen(true)
  frame:SetBackdropColor(0.04, 0.05, 0.04, 0.94)
  SetStatus("Aventuriers autonomes actifs")
end

function RealmBoxCompanions_OnUpdate(frame, elapsed)
  if table.getn(partyQueue) == 0 then
    return
  end

  partyQueueElapsed = partyQueueElapsed + elapsed
  if partyQueueElapsed < 0.8 then
    return
  end
  partyQueueElapsed = 0

  local command = table.remove(partyQueue, 1)
  SendChatMessage(command, "SAY")
  if table.getn(partyQueue) == 0 then
    SetStatus("Équipe demandée · regroupement en cours")
  else
    SetStatus("Formation de l'équipe · " .. table.getn(partyQueue) .. " restant(s)")
  end
end

function RealmBoxCompanions_FormParty()
  if GetNumPartyMembers() >= 4 then
    SetStatus("Votre groupe est déjà complet")
    return
  end

  partyQueue = {}
  local missing = 4 - GetNumPartyMembers()
  for index, command in ipairs(PARTY_TEMPLATE) do
    if index > missing then
      break
    end
    table.insert(partyQueue, command)
  end
  partyQueueElapsed = 0.8
  SetStatus("Formation d'une équipe équilibrée…")
end

function RealmBoxCompanions_Run(action)
  local command = COMMANDS[action]
  if not command then
    DEFAULT_CHAT_FRAME:AddMessage("RealmBox : action refusée")
    return
  end
  if GetNumPartyMembers() == 0 then
    SetStatus("Formez d'abord une équipe")
    return
  end
  SendChatMessage(command, "PARTY")
end

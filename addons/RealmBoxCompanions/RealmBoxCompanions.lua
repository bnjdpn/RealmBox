local COMMANDS = {
  follow = "follow",
  attack = "attack",
  stay = "stay",
  regroup = "summon",
  cooldowns_on = "cooldowns on",
  leave = "leave",
}

function RealmBoxCompanions_OnLoad(frame)
  frame:RegisterForDrag("LeftButton")
  frame:SetClampedToScreen(true)
  frame:SetBackdropColor(0.04, 0.05, 0.04, 0.94)
  RealmBoxCompanionsFrameStatus:SetText("Commandes Playerbots bornées")
end

function RealmBoxCompanions_Run(action)
  local command = COMMANDS[action]
  if not command then
    DEFAULT_CHAT_FRAME:AddMessage("RealmBox : action refusée")
    return
  end
  SendChatMessage(command, "PARTY")
end


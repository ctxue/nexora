const HOST = "com.nexora.bridge";

function browserInfo() {
  const edge = navigator.userAgent.includes("Edg/");
  return edge
    ? {
        name: "Microsoft Edge",
        exe: "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
      }
    : {
        name: "Google Chrome",
        exe: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
      };
}

function nativeMessage(message) {
  return new Promise((resolve) => {
    chrome.runtime.sendNativeMessage(HOST, message, (response) => {
      void chrome.runtime.lastError;
      resolve(response ?? null);
    });
  });
}

async function collectTabs() {
  const browser = browserInfo();
  const tabs = await chrome.tabs.query({});
  return tabs
    .filter((tab) => tab.id != null && tab.url?.startsWith("http"))
    .map((tab) => ({
      id: tab.id,
      windowId: tab.windowId,
      title: tab.title ?? tab.url ?? "",
      url: tab.url,
      browserName: browser.name,
      browserExe: browser.exe,
      active: tab.active,
    }));
}

async function pushSnapshot() {
  const response = await nativeMessage({
    type: "snapshot",
    tabs: await collectTabs(),
  });
  await chrome.action.setBadgeText({
    text: response?.type === "ack" ? "ON" : "!",
  });
  await chrome.action.setBadgeBackgroundColor({
    color: response?.type === "ack" ? "#18181b" : "#b91c1c",
  });
}

async function pollCommands() {
  const response = await nativeMessage({ type: "poll" });
  const command = response?.command;
  if (
    command?.type === "activateTab" &&
    Number.isInteger(command.tabId)
  ) {
    const tab = await chrome.tabs.get(command.tabId).catch(() => null);
    if (tab) {
      await chrome.windows.update(tab.windowId, { focused: true });
      await chrome.tabs.update(tab.id, { active: true });
    }
  }
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.alarms.create("nexora-poll", { periodInMinutes: 0.5 });
  void pushSnapshot();
});
chrome.runtime.onStartup.addListener(() => {
  chrome.alarms.create("nexora-poll", { periodInMinutes: 0.5 });
  void pushSnapshot();
});
chrome.tabs.onCreated.addListener(() => void pushSnapshot());
chrome.tabs.onUpdated.addListener(() => void pushSnapshot());
chrome.tabs.onRemoved.addListener(() => void pushSnapshot());
chrome.tabs.onActivated.addListener(() => void pushSnapshot());
chrome.windows.onFocusChanged.addListener(() => void pushSnapshot());
chrome.action.onClicked.addListener(() => void pushSnapshot());
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "nexora-poll") {
    void pushSnapshot();
    void pollCommands();
  }
});
setInterval(() => {
  void pushSnapshot();
  void pollCommands();
}, 5000);

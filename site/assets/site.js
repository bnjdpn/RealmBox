const supported = new Set(["fr", "en"]);
const queryLanguage = new URLSearchParams(window.location.search).get("lang");
const storedLanguage = window.localStorage.getItem("realmbox-site-language");
const browserLanguage = window.navigator.language.toLowerCase().startsWith("fr") ? "fr" : "en";

function setLanguage(language) {
  const selected = supported.has(language) ? language : "fr";
  document.documentElement.lang = selected;
  window.localStorage.setItem("realmbox-site-language", selected);
  document.querySelectorAll("[data-set-language]").forEach((button) => {
    const active = button.dataset.setLanguage === selected;
    button.classList.toggle("active", active);
    button.setAttribute("aria-pressed", String(active));
  });
  document.title = selected === "fr" ? "RealmBox — monde 3.3.5a local" : "RealmBox — local 3.3.5a world";
}

document.querySelectorAll("[data-set-language]").forEach((button) => {
  button.addEventListener("click", () => setLanguage(button.dataset.setLanguage));
});

setLanguage(queryLanguage ?? storedLanguage ?? browserLanguage);

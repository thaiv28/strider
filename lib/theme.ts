export const THEME_KEY = "strider-theme";
export type ThemePreference = "light" | "dark" | "system";

export const THEME_SCRIPT = `(function(){try{var preference=localStorage.getItem("${THEME_KEY}");document.documentElement.dataset.theme=preference==="dark"||(preference==="system"&&matchMedia("(prefers-color-scheme: dark)").matches)?"dark":"light";}catch(e){document.documentElement.dataset.theme="light";}})();`;

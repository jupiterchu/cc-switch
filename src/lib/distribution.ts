/** Build-time distribution settings; ordinary builds retain upstream behavior. */
export const IS_AIGOCODE_COMPAT = import.meta.env.VITE_AIGOCODE_COMPAT === "1";

export const RELEASES_URL = IS_AIGOCODE_COMPAT
  ? import.meta.env.VITE_AIGOCODE_RELEASES_URL ||
    "https://github.com/jupiterchu/cc-switch/releases"
  : "https://github.com/farion1231/cc-switch/releases";

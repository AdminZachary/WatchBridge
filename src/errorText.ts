import i18n from "./i18next";

/// The backend reports some failures as a stable `err.*` key instead of prose,
/// so the interface can show them in the user's language. Anything that is not
/// a known key is returned unchanged.
export const localizeError = (error: unknown): string => {
  const text =
    typeof error === "object" && error !== null && "message" in error
      ? String((error as { message: unknown }).message)
      : String(error);
  const key = text.trim();
  if (!key.startsWith("err.")) {
    return text;
  }
  const translated = i18n.t(key);
  return translated === key ? text : translated;
};

import { useCallback, useEffect, useRef, useState } from "react";
import "./AppleID.css";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { load } from "@tauri-apps/plugin-store";
import { Modal } from "./components/Modal";
import { TwoFactorDialog, type TwoFactorRequest, type TwoFactorResponse } from "./components/TwoFactorDialog";
import { toast } from "sonner";
import { useStore } from "./StoreContext";
import { useError } from "./ErrorContext";
import { Certificate } from "./pages/Certificates";
import { useTranslation } from "react-i18next";

const storePromise = load("data.json");

export const AppleID = ({
  loggedInAs,
  setLoggedInAs,
  noKeyringAvailable,
}: {
  loggedInAs: string | null;
  setLoggedInAs: (id: string | null) => void;
  noKeyringAvailable: boolean;
}) => {
  const { t } = useTranslation();
  const [storedIds, setStoredIds] = useState<string[]>([]);
  const [forceUpdateIds, setForceUpdateIds] = useState<number>(0);
  const [emailInput, setEmailInput] = useState<string>("");
  const [passwordInput, setPasswordInput] = useState<string>("");
  const [saveCredentials, setSaveCredentials] = useState<boolean>(false);
  const [tfaRequest, setTfaRequest] = useState<TwoFactorRequest | null>(null);
  const activeTfa = useRef<TwoFactorRequest | null>(null);
  const tfaSending = useRef(false);
  const [tfaPending, setTfaPending] = useState(false);
  const [listenersReady, setListenersReady] = useState(false);
  const [loginPending, setLoginPending] = useState(false);
  const loginInProgress = useRef(false);
  const [addAccountOpen, setAddAccountOpen] = useState<boolean>(false);
  const [anisetteServer] = useStore<string>(
    "anisetteServer",
    "ani.jkcoxson.com",
  );
  const [certs, setCerts] = useState<Certificate[] | null>(null);
  const [selectedSerials, setSelectedSerials] = useState<string[]>([]);
  const [chooseCertsOpen, setChooseCertsOpen] = useState<boolean>(false);
  const { err } = useError();

  useEffect(() => {
    let getLoggedInAs = async () => {
      let account = await invoke<string | null>("logged_in_as");
      setLoggedInAs(account);
    };
    let getStoredIds = async () => {
      const store = await storePromise;
      let ids = (await store.get<string[]>("ids")) ?? [];
      setStoredIds(ids);
    };

    getLoggedInAs();
    getStoredIds();
  }, [forceUpdateIds]);

  useEffect(() => {
    setSelectedSerials(certs?.map((c) => c.serialNumber) ?? []);
  }, [certs]);

  const clearTwoFactor = useCallback(() => {
    activeTfa.current = null;
    tfaSending.current = false;
    setTfaRequest(null);
    setTfaPending(false);
  }, []);

  const respondToTwoFactor = async (requestId: string, response: TwoFactorResponse) => {
    if (activeTfa.current?.requestId !== requestId || tfaSending.current) return;
    tfaSending.current = true;
    setTfaPending(true);
    try {
      await emit("2fa-response", { requestId, response });
      // A new challenge may arrive before emit's promise resolves.
      if (activeTfa.current?.requestId === requestId) clearTwoFactor();
    } catch {
      if (activeTfa.current?.requestId === requestId) {
        tfaSending.current = false;
        setTfaPending(false);
      }
      toast.error(t("apple_id.two_factor_action_failed"));
    }
  };

  useEffect(() => {
    let disposed = false;
    const cleanup: (() => void)[] = [];
    const subscriptions = [
      () => listen<TwoFactorRequest>("2fa-required", ({ payload }) => {
        activeTfa.current = payload;
        tfaSending.current = false;
        setTfaRequest(payload);
        setTfaPending(false);
      }),
      () => listen<string>("2fa-finished", ({ payload }) => {
        if (activeTfa.current?.requestId === payload) clearTwoFactor();
      }),
      () => listen<Certificate[]>("max-certs-reached", ({ payload }) => setCerts(payload)),
    ];
    const register = async () => {
      for (const subscribe of subscriptions) {
        const unlisten = await subscribe();
        if (disposed) { unlisten(); return; }
        cleanup.push(unlisten);
      }
      setListenersReady(true);
    };
    void register().catch(() => {
      cleanup.splice(0).forEach((unlisten) => unlisten());
      if (!disposed) toast.error(t("apple_id.two_factor_listener_failed"));
    });
    return () => {
      disposed = true;
      cleanup.splice(0).forEach((unlisten) => unlisten());
      const request = activeTfa.current;
      activeTfa.current = null;
      if (request) void emit("2fa-response", { requestId: request.requestId, response: "Abort" });
    };
  }, [clearTwoFactor]);

  const runLogin = (operation: () => Promise<void>) => {
    if (!listenersReady || loginInProgress.current) return;
    loginInProgress.current = true;
    setLoginPending(true);
    const attempt = async () => {
      try {
        await operation();
      } finally {
        clearTwoFactor();
        loginInProgress.current = false;
        setLoginPending(false);
      }
    };
    toast.promise(attempt, {
      loading: t("apple_id.logging_in"),
      success: t("apple_id.logged_in_success"),
      error: (error) => err(t("apple_id.login_failed"), error),
    });
  };

  // Never pre-select certificates: revoking one is destructive and must be a
  // deliberate choice. Opening the list makes that choice visible.
  useEffect(() => {
    if (certs) {
      setSelectedSerials([]);
      setChooseCertsOpen(true);
    }
  }, [certs]);

  return (
    <>
      <h2 style={{ marginTop: 0 }}>{t("apple_id.title")}</h2>
      <div className="credentials-container">
        {loggedInAs && (
          <div className="logged-in-as card green">
            <div className="logged-info">
              <span className="logged-label">{t("apple_id.logged_in_as")}</span>
              <span className="logged-value">{loggedInAs}</span>
            </div>
            <div className="action-row">
              <button
                type="button"
                className="action-button danger"
                onClick={async () => {
                  let promise = async () => {
                    await invoke("invalidate_account");
                    setForceUpdateIds((v) => v + 1);
                  };
                  toast.promise(promise, {
                    loading: t("apple_id.signing_out"),
                    error: (e) => err(t("apple_id.sign_out_failed"), e),
                    success: t("apple_id.signed_out_success"),
                  });
                }}
              >
                {t("apple_id.sign_out")}
              </button>
            </div>
          </div>
        )}
        {storedIds.length > 0 && (
          <div className="stored-ids">
            <h3 style={{ margin: 0 }}>{t("apple_id.saved_logins")}</h3>
            <div className="stored-container card">
              {storedIds.map((id) => (
                <div key={id} className="stored">
                  <div className="stored-email">{id}</div>
                  <div className="action-row">
                    {!loggedInAs && (
                      <button
                        type="button"
                        className="action-button primary"
                        disabled={!listenersReady || loginPending}
                        onClick={() => {
                          runLogin(async () => {
                            try {
                              await invoke("login_stored", {
                                email: id,
                                anisetteServer,
                              });
                            } finally {
                              // A missing keyring entry removes stale saved metadata.
                              setForceUpdateIds((v) => v + 1);
                            }
                          });
                        }}
                      >
                        {t("apple_id.sign_in")}
                      </button>
                    )}
                    <button
                      type="button"
                      className="action-button danger"
                      onClick={async () => {
                        let promise = async () => {
                          await invoke("delete_account", { email: id });
                          setForceUpdateIds((v) => v + 1);
                        };
                        toast.promise(promise, {
                          loading: t("apple_id.deleting"),
                          error: (e) => err(t("apple_id.deletion_failed"), e),
                          success: t("apple_id.deleted_success"),
                        });
                      }}
                    >
                      {t("common.delete")}
                    </button>
                  </div>
                </div>
              ))}
              {!addAccountOpen && (
                <div
                  className="stored add-account"
                  onClick={() => {
                    setAddAccountOpen(true);
                  }}
                >
                  {t("apple_id.add_account")}
                </div>
              )}
            </div>
          </div>
        )}
        {((loggedInAs === null && storedIds.length === 0) ||
          addAccountOpen) && (
          <div className="new-login">
            {storedIds.length > 0 && <h3>{t("apple_id.new_login")}</h3>}
            <form
              className="credentials"
              onSubmit={async (e) => {
                e.preventDefault();
                if (!emailInput || !passwordInput) {
                  toast.warning(t("apple_id.enter_email_password"));
                  return;
                }
                // if (!emailInput.includes("@")) {
                //   toast.warning(t("apple_id.valid_email"));
                //   return;
                // }
                runLogin(async () => {
                  await invoke("login_new", {
                    email: emailInput,
                    password: passwordInput,
                    saveCredentials,
                    anisetteServer,
                  });
                  setForceUpdateIds((v) => v + 1);
                });
              }}
            >
              <input
                type="text"
                placeholder={t("apple_id.email_placeholder")}
                value={emailInput}
                onChange={(e) => setEmailInput(e.target.value)}
              />
              <input
                type="password"
                placeholder={t("apple_id.password_placeholder")}
                value={passwordInput}
                onChange={(e) => setPasswordInput(e.target.value)}
              />
              {noKeyringAvailable ? (
                <p className="settings-hint credentials-warning">
                  {t("apple_id.no_keyring_available")}
                </p>
              ) : (
                <div className="save-credentials">
                  <input
                    type="checkbox"
                    id="save-credentials"
                    checked={saveCredentials}
                    onChange={(e) => setSaveCredentials(e.target.checked)}
                  />
                  <label htmlFor="save-credentials">
                    {t("apple_id.save_credentials")}
                  </label>
                </div>
              )}
              <button type="submit" disabled={!listenersReady || loginPending}>{t("apple_id.login")}</button>
              {addAccountOpen && storedIds.length > 0 && (
                <button
                  onClick={() => {
                    setAddAccountOpen(false);
                  }}
                >
                  {t("common.cancel")}
                </button>
              )}
            </form>
          </div>
        )}
      </div>
      <TwoFactorDialog key={tfaRequest?.requestId ?? "idle"} request={tfaRequest} pending={tfaPending} onRespond={respondToTwoFactor} />
      <Modal sizeFit isOpen={certs !== null} zIndex={2000}>
        <h2 className="cert-header">{t("apple_id.max_certs_title")}</h2>
        <p className="certs-desc">{t("apple_id.max_certs_desc")}</p>
        <p
          className="certs-see"
          role="button"
          tabIndex={0}
          onClick={() => setChooseCertsOpen((v) => !v)}
        >
          {chooseCertsOpen
            ? t("apple_id.hide_certificate_list")
            : t("apple_id.choose_what_to_revoke")}
        </p>
        {chooseCertsOpen && certs && (
          <div className="certs-list">
            {certs.map((cert) => (
              <div
                key={cert.serialNumber}
                className="cert-item"
                onClick={() => {
                  setSelectedSerials((prev) => {
                    if (prev.includes(cert.serialNumber)) {
                      return prev.filter((s) => s !== cert.serialNumber);
                    } else {
                      return [...prev, cert.serialNumber];
                    }
                  });
                }}
              >
                <input
                  type="checkbox"
                  id={cert.serialNumber}
                  name={cert.serialNumber}
                  value={cert.serialNumber}
                  checked={selectedSerials.includes(cert.serialNumber)}
                />
                <label htmlFor={cert.serialNumber}>
                  {cert.name} - {cert.machineName}
                </label>
              </div>
            ))}
          </div>
        )}
        <div className="certs-buttons">
          {selectedSerials.length > 0 && (
            <>
              <p className="certs-desc">
                {t("apple_id.revoke_count", { count: selectedSerials.length })}
              </p>
              <p className="certs-desc">{t("apple_id.revoke_warning")}</p>
            </>
          )}
          <button
            className="action-button primary"
            disabled={selectedSerials.length === 0}
            onClick={async () => {
              await emit(
                "max-certs-response",
                selectedSerials.length > 0 ? selectedSerials : null,
              );
              setCerts(null);
              setChooseCertsOpen(false);
            }}
          >
            {t("apple_id.continue")}
          </button>
          <button
            className="action-button danger"
            onClick={async () => {
              await emit("max-certs-response", null);
              setCerts(null);
              setChooseCertsOpen(false);
            }}
          >
            {t("common.cancel")}
          </button>
        </div>
      </Modal>
    </>
  );
};

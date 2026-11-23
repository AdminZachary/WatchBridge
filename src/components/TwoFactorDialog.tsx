import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Modal } from "./Modal";

export type TwoFactorResponse =
  | { SubmitCode: string }
  | { SendSms: number }
  | "SendToDevices"
  | "ResendCode"
  | "Abort";

export interface TwoFactorRequest {
  requestId: string;
  lastError: string | null;
  unknown: boolean;
  sms: boolean;
  numbers: {
    id: number;
    numberWithDialCode: string;
    lastTwoDigits: string;
    pushMode: string;
  }[];
  selectedNumberId: number | null;
}

export function TwoFactorDialog({
  request,
  pending,
  onRespond,
}: {
  request: TwoFactorRequest | null;
  pending: boolean;
  onRespond: (requestId: string, response: TwoFactorResponse) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [code, setCode] = useState("");
  const [numberId, setNumberId] = useState<number | null>(request?.selectedNumberId ?? request?.numbers[0]?.id ?? null);

  useEffect(() => {
    setCode("");
    setNumberId(request?.selectedNumberId ?? request?.numbers[0]?.id ?? null);
  }, [request]);

  if (!request) return null;

  const validCode = /^[0-9]{6}$/.test(code);
  const currentNumber = request.numbers.find((number) => number.id === request.selectedNumberId);
  const numberLabel = (number: TwoFactorRequest["numbers"][number]) =>
    number.numberWithDialCode.trim() || t("apple_id.two_factor_phone_ending", { digits: number.lastTwoDigits });
  const respond = (response: TwoFactorResponse) => onRespond(request.requestId, response);

  return (
    <Modal
      sizeFit
      isOpen
      zIndex={2000}
      close={pending ? undefined : () => { void respond("Abort"); }}
    >
      <div className="tfa-dialog">
        <h2>{t("apple_id.two_factor_title")}</h2>
        <p>{t(request.sms ? "apple_id.two_factor_sms_prompt" : "apple_id.two_factor_prompt")}</p>
        {request.sms && currentNumber && <p className="tfa-destination">{t("apple_id.two_factor_sent_to", { number: numberLabel(currentNumber) })}</p>}
        {request.lastError && <p className="tfa-error" role="alert">{request.lastError}</p>}
        <form className="tfa-code-form" onSubmit={(event) => {
          event.preventDefault();
          if (validCode && !pending) void respond({ SubmitCode: code });
        }}>
          <input
            aria-label={t("apple_id.verification_placeholder")}
            autoComplete="one-time-code"
            inputMode="numeric"
            maxLength={6}
            placeholder={t("apple_id.verification_placeholder")}
            value={code}
            onChange={(event) => setCode(event.target.value.replace(/[^0-9]/g, ""))}
            disabled={pending}
          />
          <button className="action-button primary" type="submit" disabled={pending || !validCode}>{t("apple_id.submit")}</button>
        </form>
        {request.numbers.length > 0 ? (
          <fieldset className="tfa-sms" disabled={pending} aria-describedby="tfa-phone-hint">
            <legend>{t("apple_id.two_factor_phone")}</legend>
            <p id="tfa-phone-hint" className="tfa-hint">{t("apple_id.two_factor_choose_phone")}</p>
            <div className="tfa-phone-options">
              {request.numbers.map((number) => (
                <label key={number.id} className={`tfa-phone-option${numberId === number.id ? " selected" : ""}`}>
                  <input type="radio" name="tfa-phone" value={number.id} checked={numberId === number.id} onChange={() => setNumberId(number.id)} />
                  <span className="tfa-phone-number" dir="ltr">{numberLabel(number)}</span>
                  {request.sms && request.selectedNumberId === number.id && <span className="tfa-phone-badge">{t("apple_id.two_factor_current_phone")}</span>}
                </label>
              ))}
            </div>
            <button className="action-button primary tfa-send-sms" type="button" disabled={pending || numberId === null} onClick={() => {
              if (numberId !== null) void respond({ SendSms: numberId });
            }}>
              {t("apple_id.two_factor_send_sms")}
            </button>
          </fieldset>
        ) : <p className="tfa-hint tfa-no-numbers">{t("apple_id.two_factor_no_numbers")}</p>}
        <div className="tfa-methods">
          <button type="button" disabled={pending} onClick={() => { void respond("ResendCode"); }}>
            {t("apple_id.two_factor_resend")}
          </button>
          {(request.sms || request.unknown) && (
            <button type="button" disabled={pending} onClick={() => { void respond("SendToDevices"); }}>
              {t("apple_id.two_factor_use_devices")}
            </button>
          )}
          <button type="button" disabled={pending} onClick={() => { void respond("Abort"); }}>
            {t("common.cancel")}
          </button>
        </div>
      </div>
    </Modal>
  );
}

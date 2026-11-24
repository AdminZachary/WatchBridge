import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import "./InstallProgress.css";

const stages = ["preparing", "signing", "phoneInstall", "watchConnect", "watchTransfer", "watchInstall"] as const;
export type InstallStage = typeof stages[number];
export type InstallProgressUpdate = {
  requestId: string;
  stage: InstallStage;
  percent: number | null;
  bytesSent: number | null;
  totalBytes: number | null;
  appIndex?: number | null;
  appCount?: number | null;
};

export function validProgress(update: InstallProgressUpdate): boolean {
  return typeof update?.requestId === "string" && stages.includes(update.stage) &&
    (update.percent == null || (Number.isFinite(update.percent) && update.percent >= 0 && update.percent <= 100));
}

export function InstallProgress({ update, state, startedAt }: {
  update: InstallProgressUpdate;
  state: "running" | "done" | "failed";
  startedAt: number;
}) {
  const { t } = useTranslation();
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    setNow(Date.now());
    if (state !== "running") return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [state, startedAt]);
  const index = stages.indexOf(update.stage);
  const seconds = Math.max(0, Math.floor((now - startedAt) / 1000));
  const time = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  const percent = state === "done" ? 100 : update.percent;
  const title = state === "done" ? t("wb.progress.done") : state === "failed" ? t("wb.progress.failed") : t(`wb.progress.${update.stage}`);
  const bytes = update.stage === "watchTransfer" && update.bytesSent != null && update.totalBytes != null;

  return (
    <section className={`install-progress is-${state}`} aria-label={t("wb.progress.title")}>
      <div className="install-progress-heading">
        <div><span className="install-progress-eyebrow">{t("wb.progress.title")}</span><strong role="status">{title}</strong></div>
        <span className="install-progress-percent">{percent == null ? "—" : `${Math.round(percent)}%`}</span>
      </div>
      <div className={`install-progress-track${percent == null && state === "running" ? " is-indeterminate" : ""}`} role="progressbar"
        aria-label={state === "done" ? title : t(`wb.progress.${update.stage}`)} aria-valuemin={0} aria-valuemax={100}
        aria-valuenow={percent ?? undefined} aria-valuetext={state === "failed" ? title : percent == null ? t("wb.progress.waiting") : `${Math.round(percent)}%`}>
        <span style={{ width: percent == null ? "35%" : `${percent}%` }} />
      </div>
      <div className="install-progress-meta">
        <span>{state === "done" ? title : bytes ? `${(update.bytesSent! / 1048576).toFixed(1)} / ${(update.totalBytes! / 1048576).toFixed(1)} MiB` : percent == null && state === "running" ? t("wb.progress.waiting") : t(`wb.progress.${update.stage}`)}</span>
        <span>{t("wb.progress.elapsed", { time })}</span>
      </div>
      {(update.appCount ?? 0) > 1 && <p className="install-progress-note">{t("wb.progress.companion", { index: update.appIndex, count: update.appCount })}</p>}
      <ol className="install-progress-stages">
        {stages.map((stage, step) => {
          const completed = state === "done" || step < index;
          const active = state !== "done" && step === index;
          return <li key={stage} className={`${completed ? "is-complete" : ""}${active ? " is-current" : ""}`} aria-current={active ? "step" : undefined}>
            <span aria-hidden="true">{completed ? "✓" : step + 1}</span>{t(`wb.progress.${stage}`)}
          </li>;
        })}
      </ol>
      {state !== "done" && <p className="install-progress-note">{t("wb.progress.hint")}</p>}
    </section>
  );
}

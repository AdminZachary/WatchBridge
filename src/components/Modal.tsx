import { useEffect } from "react";
import { createPortal } from "react-dom";
import "./Modal.css";

export const Modal = ({
  isOpen,
  close,
  sizeFit,
  children,
  hideClose,
  zIndex,
}: {
  children: React.ReactNode;
  isOpen: boolean;
  close?: () => void;
  sizeFit?: boolean;
  hideClose?: boolean;
  zIndex?: number;
}) => {
  useEffect(() => {
    if (!isOpen || !close) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen, close]);

  if (!isOpen) {
    return null;
  }

  // A modal inside a disabled <fieldset> inherits the disabled state and none of its
  // buttons can be clicked, so it must render outside that subtree. Target the app
  // root rather than <body>: the light theme is applied through `.wb-app` scoped
  // rules, and a body-level modal would fall back to the dark defaults.
  const target = document.querySelector(".wb-app") ?? document.body;
  return createPortal(
    <div
      className={`modal-container`}
      style={
        zIndex
          ? {
              zIndex: zIndex.toString(),
            }
          : {}
      }
    >
      <div className={`modal${sizeFit ? " size-fit" : ""}`}>
        {!hideClose && close && (
          <button
            className="modal-close"
            onClick={() => {
              close();
            }}
          >
            &#x2715;
          </button>
        )}
        <div className="modal-content">{children}</div>
      </div>
    </div>,
    target,
  );
};

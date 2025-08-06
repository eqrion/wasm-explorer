import React, { useEffect, useRef } from "react";
import { createPortal } from "react-dom";

interface ModalProps {
  onClose?: () => void;
  children: React.ReactNode;
  allowDismiss?: boolean;
  className?: string;
}

export const Modal: React.FC<ModalProps> = ({
  onClose,
  children,
  allowDismiss = true,
  className = "",
}) => {
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && allowDismiss && onClose) {
        onClose();
      }
    };

    const handleClickOutside = (e: MouseEvent) => {
      if (allowDismiss && onClose && overlayRef.current === e.target) {
        onClose();
      }
    };

    document.addEventListener("keydown", handleEscape);
    document.addEventListener("mousedown", handleClickOutside);

    return () => {
      document.removeEventListener("keydown", handleEscape);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [allowDismiss, onClose]);

  const modalContent = (
    <div
      ref={overlayRef}
      className={`fixed inset-0 z-50 flex items-center justify-center p-4 ${
        allowDismiss ? "bg-black/50" : "bg-black/70 backdrop-blur-sm"
      }`}
      role="dialog"
      aria-modal="true"
    >
      <div
        className={`bg-white rounded-lg shadow-xl max-w-md w-full max-h-[90vh] overflow-auto ${className}`}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );

  return createPortal(modalContent, document.body);
};

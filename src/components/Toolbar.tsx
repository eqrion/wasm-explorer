import * as React from "react";
import { useRef } from "react";
import { Module } from "../Module.js";
import { ShareButton } from "./ShareButton.js";

function WasmLogo() {
  return (
    <img
      src="./logo.svg"
      alt="WebAssembly Logo"
      width="32"
      height="32"
      className="text-blue-600"
    />
  );
}

export function Toolbar(props: {
  onFileLoad: (title: string, content: ArrayBuffer) => void;
  onDownload: () => void;
  onShowHelp: () => void;
  module: Module;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        const content = e.target?.result as ArrayBuffer;
        props.onFileLoad(file.name, content);
      };
      reader.readAsArrayBuffer(file);
    }
  };

  return (
    <div className="flex items-center px-4 py-3 bg-gray-50 border-b border-gray-200 gap-4 shadow-sm">
      <div className="flex items-center gap-2">
        <WasmLogo />
        <span className="text-lg font-semibold text-gray-800">
          WebAssembly Explorer
        </span>
      </div>

      <div className="flex-1" />

      {/* <ShareButton module={props.module} /> */}

      <button
        onClick={() => props.onDownload()}
        className="px-4 py-2 bg-blue-600 text-white border-none rounded cursor-pointer text-sm font-medium hover:bg-blue-700 transition-colors"
      >
        Download
      </button>

      <button
        onClick={() => fileInputRef.current?.click()}
        className="px-4 py-2 bg-blue-600 text-white border-none rounded cursor-pointer text-sm font-medium hover:bg-blue-700 transition-colors"
      >
        Open
      </button>
      {/* <button
        onClick={() => props.onShowHelp()}
        className="py-2 text-gray-600 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors"
        title="Help"
      >
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="10" />
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
          <circle cx="12" cy="17" r="0.5" />
        </svg>
      </button> */}
      <input
        ref={fileInputRef}
        type="file"
        accept=".wasm,.wat"
        onChange={handleFileSelect}
        className="hidden"
      />
    </div>
  );
}

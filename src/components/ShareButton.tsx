import * as React from "react";
import { useState } from "react";
import * as api from "../Api.js";
import { Module } from "../Module.js";

interface ShareButtonProps {
  module: Module;
}

export function ShareButton({ module }: ShareButtonProps) {
  const [shareState, setShareState] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [shareCode, setShareCode] = useState<string>('');

  const handleShare = async () => {
    if (!module) {
      return;
    }
    
    setShareState('loading');
    try {
      const source = await module.getSource();
      const sourceBuffer = source.buffer;
      if (!(sourceBuffer instanceof ArrayBuffer)) {
        throw new Error("Source is not an ArrayBuffer");
      }

      const code = await api.storePayload(sourceBuffer);
      setShareCode(code);
      setShareState('success');
    } catch (error) {
      console.error('Share failed:', error);
      setShareState('error');
    }
  };

  const handleCopyShareCode = async () => {
    try {
      await navigator.clipboard.writeText(shareCode);
    } catch (error) {
      console.error('Copy failed:', error);
    }
  };

  const resetShare = () => {
    setShareState('idle');
    setShareCode('');
  };

  if (shareState === 'idle') {
    return (
      <button
        onClick={handleShare}
        className="px-4 py-2 bg-blue-600 text-white border-none rounded cursor-pointer text-sm font-medium hover:bg-blue-700 transition-colors"
      >
        Share
      </button>
    );
  }

  if (shareState === 'loading') {
    return (
      <div className="px-4 py-2 flex items-center gap-2">
        <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
        <span className="text-sm text-gray-600">Sharing...</span>
      </div>
    );
  }

  if (shareState === 'success') {
    return (
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={shareCode}
          readOnly
          className="px-3 py-2 text-sm border border-gray-300 rounded bg-gray-50 text-gray-700 font-mono"
          style={{ width: '120px' }}
        />
        <button
          onClick={handleCopyShareCode}
          className="px-3 py-2 bg-green-600 text-white border-none rounded cursor-pointer text-sm font-medium hover:bg-green-700 transition-colors"
          title="Copy to clipboard"
        >
          Copy
        </button>
        <button
          onClick={resetShare}
          className="px-2 py-2 text-gray-500 hover:text-gray-700 transition-colors"
          title="Close"
        >
          ✕
        </button>
      </div>
    );
  }

  if (shareState === 'error') {
    return (
      <div className="flex items-center gap-2">
        <span className="text-red-500 text-lg">⚠️</span>
        <span className="text-sm text-red-600">Share failed</span>
        <button
          onClick={resetShare}
          className="px-2 py-2 text-gray-500 hover:text-gray-700 transition-colors"
          title="Close"
        >
          ✕
        </button>
      </div>
    );
  }

  return null;
}

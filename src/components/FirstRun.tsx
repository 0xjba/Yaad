import React from "react";

interface FirstRunProps {
  onComplete: () => void;
}

const FirstRun: React.FC<FirstRunProps> = ({ onComplete }) => {
  return (
    <div className="h-full w-full bg-white/95 dark:bg-black/95 flex flex-col items-center justify-center p-8 text-center">
      <div className="mb-6">
        <div className="w-16 h-16 bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 rounded-2xl flex items-center justify-center text-3xl mx-auto mb-4">
          👋
        </div>
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-200 mb-2">Welcome to Yaad</h1>
        <p className="text-gray-500 dark:text-gray-400">Your AI-powered memory assistant is ready.</p>
      </div>

      <div className="w-full max-w-sm bg-white dark:bg-neutral-900 border border-gray-100 dark:border-gray-800 rounded-xl shadow-sm p-4 mb-8 text-left space-y-4">
        <div className="flex items-start gap-3">
          <div className="p-2 bg-red-50 dark:bg-red-900/30 text-red-500 dark:text-red-400 rounded-lg">🎤</div>
          <div>
            <p className="font-semibold text-gray-700 dark:text-gray-300 text-sm">Left Click</p>
            <p className="text-xs text-gray-500 dark:text-gray-400">Instantly start recording a thought.</p>
          </div>
        </div>
        <div className="flex items-start gap-3">
          <div className="p-2 bg-blue-50 dark:bg-blue-900/30 text-blue-500 dark:text-blue-400 rounded-lg">🔍</div>
          <div>
            <p className="font-semibold text-gray-700 dark:text-gray-300 text-sm">Right Click</p>
            <p className="text-xs text-gray-500 dark:text-gray-400">Search through your past memories.</p>
          </div>
        </div>
      </div>

      <button
        onClick={onComplete}
        className="w-full max-w-xs py-3 bg-gray-900 dark:bg-gray-100 hover:bg-black dark:hover:bg-white text-white dark:text-black rounded-xl font-medium transition-all transform hover:scale-[1.02]"
      >
        Get Started
      </button>
    </div>
  );
};

export default FirstRun;

import React from 'react';

interface KeyboardBadgeProps {
  children: React.ReactNode;
  className?: string;
}

export const KeyboardBadge: React.FC<KeyboardBadgeProps> = ({ children, className = '' }) => {
  return (
    <span className={`bg-neutral-200 dark:bg-neutral-800 px-1.5 py-0.5 rounded border border-black/5 dark:border-white/5 text-[10px] min-w-[20px] text-center ${className}`}>
      {children}
    </span>
  );
};

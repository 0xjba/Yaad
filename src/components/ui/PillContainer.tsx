import React from 'react';
import { twMerge } from 'tailwind-merge';

interface PillContainerProps {
  children: React.ReactNode;
  className?: string;
}

export const PillContainer: React.FC<PillContainerProps> = ({ children, className = '' }) => {
  return (
    <div 
      className={twMerge(
        'h-pill w-full vibrancy panel-base rounded-xl flex items-center px-3 shrink-0 z-10 gap-3',
        className
      )}
    >
      {children}
    </div>
  );
};

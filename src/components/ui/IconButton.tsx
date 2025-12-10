import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'ghost';
  active?: boolean;
  icon: React.ReactNode;
}

export const IconButton: React.FC<IconButtonProps> = ({ 
  variant = 'ghost', 
  active, 
  icon,
  className,
  ...props 
}) => {
  const baseClasses = 'flex items-center justify-center w-6 h-6 rounded-md transition-colors';
  
  const variantClasses = {
    primary: 'bg-electric text-white shadow-sm ring-1 ring-white/10',
    ghost: clsx(
      'text-txt-tertiary hover:text-txt-secondary',
      active && 'text-txt-secondary'
    )
  };

  const mergedClasses = twMerge(
    baseClasses,
    variantClasses[variant],
    className
  );

  return (
    <button {...props} className={mergedClasses}>
      {icon}
    </button>
  );
};

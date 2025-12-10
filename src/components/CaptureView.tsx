import React, { useEffect } from 'react';
import { X, Search, Plus, Check, Loader2, RotateCcw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { AudioVisualizer } from './Visualizer';
import { CaptureState } from '../types';
import { useRecording } from '../hooks/useRecording';
import { PillContainer } from './ui/PillContainer';
import { IconButton } from './ui/IconButton';
import { KeyboardBadge } from './ui/KeyboardBadge';

interface CaptureViewProps {
  onDiscard: () => void;
  onSave: (text: string) => void;
  state: CaptureState;
  setState: (state: CaptureState) => void;
  onToggleView: () => void;
}

export const CaptureView: React.FC<CaptureViewProps> = ({ onDiscard, onSave, state, setState, onToggleView }) => {
  const {
    transcript,
    isProcessing,
    isEditing,
    error,
    handleProcess,
    handleRetry,
    handleUserEdit,
    updateTranscript,
  } = useRecording({
    onSave,
    state,
    setState,
  });

  // Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
        if (state === 'recording') {
            if (e.key === 'Enter') {
                if (error) handleRetry();
                else handleProcess();
            }
            if (e.key === 'Escape') {
                // Discard and CLOSE window
                invoke('cancel_recording').catch(() => {});
                import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
                    getCurrentWindow().hide();
                });
            }
        } else if (state === 'review') {
            if (e.key === 'Enter' && !e.shiftKey) onSave(transcript);
            if (e.key === 'Escape') onDiscard();
        }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [state, transcript, onSave, onDiscard, error, handleRetry, handleProcess]);

  return (
    <div className="flex flex-col h-full w-full">
      
      {/* Top Pill */}
      <PillContainer className="justify-between relative overflow-hidden">
           <div className="flex items-center bg-black/5 dark:bg-black/20 rounded-lg p-1 border border-black/10 dark:border-white/10 shrink-0 select-none shadow-inner">
                 <IconButton variant="primary" icon={<Plus size={13} />} />
                 <IconButton variant="ghost" onClick={onToggleView} icon={<Search size={13} />} />
            </div>

           <div className="flex-1 flex items-center justify-start h-full overflow-hidden mr-[-6px]">
             {error ? (
                // 🚨 Error Message in Pill
                <div className="flex items-center text-red-400 text-xs font-medium tracking-tight pl-2 whitespace-nowrap overflow-hidden text-ellipsis animate-fade-in">
                    {error}
                </div>
             ) : state === 'recording' ? (
                <div className="flex items-center gap-3 w-full h-full">
                    <div className="h-full flex-1 flex items-center">
                        <AudioVisualizer isRecording={!isProcessing} />
                    </div>
                </div>
             ) : (
                <div className="flex items-center text-txt-tertiary text-xs font-medium tracking-tight pl-2 whitespace-nowrap overflow-hidden text-ellipsis">
                    Edit your thought or autosaving in 10s
                </div>
             )}
           </div>

           <div className="flex items-center gap-1 pl-0">
               {(state === 'recording' || error) && (
                   <>
                    <button 
                        onClick={() => {
                            // X button behavior: Discard and Close
                            invoke('cancel_recording').catch(() => {});
                            import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
                                getCurrentWindow().hide();
                            });
                        }} 
                        className="w-7 h-7 flex items-center justify-center rounded-md text-txt-tertiary hover:text-txt-primary hover:bg-black/10 dark:hover:bg-white/10"
                    >
                        <X size={15} />
                    </button>
                    
                    {error ? (
                        // 🚨 Retry Button
                        <button 
                            onClick={handleRetry}
                            className="w-7 h-7 flex items-center justify-center rounded-md transition-all duration-300 bg-black/10 dark:bg-white/10 hover:bg-black/20 dark:hover:bg-white/20 text-white"
                            title="Retry"
                        >
                            <RotateCcw size={13} />
                        </button>
                    ) : (
                        // Normal Process Button
                        <button 
                            onClick={handleProcess}
                            disabled={isProcessing}
                            className={`w-7 h-7 flex items-center justify-center rounded-md transition-all duration-300 bg-black/20 dark:bg-white/20 hover:bg-black/30 dark:hover:bg-white/30 text-white ${isProcessing ? 'cursor-wait' : ''}`}
                        >
                            {isProcessing ? <Loader2 size={15} className="animate-spin" /> : <Check size={15} />}
                        </button>
                    )}
                   </>
               )}
           </div>

           {/* 🚨 Recording Timer Line */}
           {state === 'recording' && !error && (
               <div 
                   className="absolute bottom-0 left-0 h-[2px] w-full animate-recording-limit z-overlay" 
                   style={{ backgroundColor: '#00FF9D', opacity: 0.8 }}
               />
           )}
      </PillContainer>

      {/* Review Card */}
      {state === 'review' && (
        <>
            <div className="h-3 shrink-0"></div>
            <div className="flex-1 vibrancy panel-base rounded-xl flex flex-col animate-slide-down origin-top overflow-hidden relative">
                <div className="flex-1 p-4 overflow-hidden flex flex-col">
                    <textarea
                        value={transcript}
                        onChange={(e) => updateTranscript(e.target.value)}
                        onFocus={handleUserEdit} // 🚨 Stop timer on click
                        className="flex-1 w-full bg-transparent resize-none outline-none text-txt-primary text-sm leading-relaxed placeholder-txt-tertiary font-normal"
                        placeholder="Transcript..."
                        autoFocus={false} // Don't steal focus immediately so timer can run
                    />
                </div>
                {/* Footer */}
                <div className="relative px-4 py-2 border-t border-glass-border flex items-center justify-end bg-black/5 dark:bg-black/20 text-xs font-medium overflow-hidden">
                    <div className="flex items-center gap-3 z-glass relative">
                        <button onClick={onDiscard} className="flex items-center gap-2 group cursor-pointer text-txt-secondary hover:text-txt-primary">
                            <span>Discard</span> <KeyboardBadge>Esc</KeyboardBadge>
                        </button>
                        <div className="w-px h-3 bg-black/10 dark:bg-white/10 mx-1"></div>
                        <button onClick={() => onSave(transcript)} className="flex items-center gap-2 group cursor-pointer text-txt-secondary hover:text-txt-primary">
                            <span>Save</span> <KeyboardBadge>↵</KeyboardBadge>
                        </button>
                    </div>
                    
                    {/* 🚨 THE 10S YELLOW PROGRESS BAR 🚨 */}
                    {/* Explicit style overrides to ensure visibility */}
                    {!isEditing && (
                        <div 
                            className="absolute bottom-0 left-0 h-[2px] w-full animate-autosave z-overlay" 
                            style={{ backgroundColor: '#00FF9D', opacity: 0.8 }}
                        />
                    )}
                </div>
            </div>
        </>
      )}
    </div>
  );
};
